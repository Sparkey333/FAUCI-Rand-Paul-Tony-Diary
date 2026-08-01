//! Sealing a directory into a verifiable snapshot.
//!
//! A manifest answers one question: *is the copy in front of me byte-for-byte
//! the thing that was sealed?* It makes no claim about whether the sealed thing
//! was authentic in the first place — only whether it has changed since.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use walkdir::WalkDir;

use crate::digest::{merkle_root, Digest};
use crate::Error;

/// Filename a sealed manifest conventionally takes inside the sealed directory.
pub const MANIFEST_FILENAME: &str = "provenance.manifest.json";

/// Bumped when the hashing scheme changes; a mismatch is a hard verify failure
/// rather than a silently wrong comparison.
pub const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Root-relative, forward-slash separated, so a manifest sealed on macOS
    /// verifies unchanged on Linux.
    pub path: String,
    pub digest: Digest,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub sealed_at: OffsetDateTime,
    /// Merkle root over every entry. One number that fixes the whole tree.
    pub root: Digest,
    pub entries: Vec<Entry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    /// Present in both, but the bytes differ.
    Modified {
        path: String,
        expected: Digest,
        actual: Digest,
    },
    /// Sealed, but no longer on disk.
    Missing { path: String },
    /// On disk, but never sealed.
    Added { path: String, digest: Digest },
}

impl Change {
    pub fn path(&self) -> &str {
        match self {
            Change::Modified { path, .. }
            | Change::Missing { path }
            | Change::Added { path, .. } => path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub expected_root: Digest,
    pub actual_root: Digest,
    pub changes: Vec<Change>,
}

impl VerifyReport {
    /// True only when the directory is byte-for-byte what was sealed.
    pub fn is_intact(&self) -> bool {
        self.changes.is_empty() && self.expected_root == self.actual_root
    }

    pub fn summary(&self) -> String {
        if self.is_intact() {
            return "intact: content matches the sealed manifest".to_string();
        }
        let (mut modified, mut missing, mut added) = (0, 0, 0);
        for c in &self.changes {
            match c {
                Change::Modified { .. } => modified += 1,
                Change::Missing { .. } => missing += 1,
                Change::Added { .. } => added += 1,
            }
        }
        format!("TAMPERED: {modified} modified, {missing} missing, {added} added")
    }
}

/// Walk `root` and hash every regular file.
///
/// Symlinks are never followed — a link pointing outside the sealed directory
/// would otherwise let content change without the manifest noticing. `.git` and
/// any previously written manifest are excluded so that sealing is idempotent.
fn collect(root: &Path) -> Result<BTreeMap<String, (Digest, u64)>, Error> {
    if !root.is_dir() {
        return Err(Error::NotADirectory(root.to_path_buf()));
    }
    let mut found = BTreeMap::new();

    for entry in WalkDir::new(root).follow_links(false).sort_by_file_name() {
        let entry = entry.map_err(|e| Error::Walk(e.to_string()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let abs = entry.path();
        let rel = abs
            .strip_prefix(root)
            .map_err(|_| Error::Walk(format!("{} escaped the sealed root", abs.display())))?;

        if rel.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }
        let rel = normalize(rel);
        if rel == MANIFEST_FILENAME {
            continue;
        }

        let (digest, size) = Digest::of_file(abs).map_err(|e| Error::Io {
            path: abs.to_path_buf(),
            source: e,
        })?;
        found.insert(rel, (digest, size));
    }
    Ok(found)
}

fn normalize(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn root_of(entries: &[Entry]) -> Digest {
    let leaves: Vec<Digest> = entries
        .iter()
        .map(|e| Digest::leaf(&e.path, e.digest))
        .collect();
    merkle_root(&leaves)
}

/// Seal `root` into a manifest. Entries are sorted by path, so re-sealing
/// unchanged content reproduces an identical manifest apart from `sealed_at`.
pub fn seal(root: &Path, note: Option<String>) -> Result<Manifest, Error> {
    let entries: Vec<Entry> = collect(root)?
        .into_iter()
        .map(|(path, (digest, size))| Entry { path, digest, size })
        .collect();

    Ok(Manifest {
        version: MANIFEST_VERSION,
        sealed_at: OffsetDateTime::now_utc(),
        root: root_of(&entries),
        entries,
        note,
    })
}

/// Compare `root` on disk against a previously sealed manifest.
pub fn verify(manifest: &Manifest, root: &Path) -> Result<VerifyReport, Error> {
    if manifest.version != MANIFEST_VERSION {
        return Err(Error::UnsupportedManifestVersion {
            found: manifest.version,
            supported: MANIFEST_VERSION,
        });
    }

    let mut on_disk = collect(root)?;
    let mut changes = Vec::new();

    for entry in &manifest.entries {
        match on_disk.remove(&entry.path) {
            None => changes.push(Change::Missing {
                path: entry.path.clone(),
            }),
            Some((actual, _)) if actual != entry.digest => changes.push(Change::Modified {
                path: entry.path.clone(),
                expected: entry.digest,
                actual,
            }),
            Some(_) => {}
        }
    }
    // Whatever is left was never sealed.
    for (path, (digest, _)) in on_disk {
        changes.push(Change::Added { path, digest });
    }
    changes.sort_by(|a, b| a.path().cmp(b.path()));

    let current = seal(root, None)?;
    Ok(VerifyReport {
        expected_root: manifest.root,
        actual_root: current.root,
        changes,
    })
}

impl Manifest {
    pub fn to_json(&self) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(s: &str) -> Result<Self, Error> {
        Ok(serde_json::from_str(s)?)
    }

    pub fn write_to(&self, path: &PathBuf) -> Result<(), Error> {
        std::fs::write(path, self.to_json()? + "\n").map_err(|e| Error::Io {
            path: path.clone(),
            source: e,
        })
    }

    pub fn read_from(path: &Path) -> Result<Self, Error> {
        let raw = std::fs::read_to_string(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::from_json(&raw)
    }

    /// Recompute the root from the entries and compare it to the stored root.
    /// Catches a manifest that was hand-edited after sealing.
    pub fn is_self_consistent(&self) -> bool {
        root_of(&self.entries) == self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("pages")).unwrap();
        fs::write(dir.path().join("pages/01.txt"), b"first page").unwrap();
        fs::write(dir.path().join("pages/02.txt"), b"second page").unwrap();
        fs::write(dir.path().join("README.txt"), b"provenance notes").unwrap();
        dir
    }

    #[test]
    fn seal_then_verify_is_intact() {
        let dir = fixture();
        let m = seal(dir.path(), None).unwrap();
        assert_eq!(m.entries.len(), 3);
        let report = verify(&m, dir.path()).unwrap();
        assert!(report.is_intact(), "{}", report.summary());
    }

    #[test]
    fn detects_modified_content() {
        let dir = fixture();
        let m = seal(dir.path(), None).unwrap();
        fs::write(dir.path().join("pages/01.txt"), b"first page (altered)").unwrap();

        let report = verify(&m, dir.path()).unwrap();
        assert!(!report.is_intact());
        assert_ne!(report.expected_root, report.actual_root);
        assert!(matches!(
            report.changes.as_slice(),
            [Change::Modified { path, .. }] if path == "pages/01.txt"
        ));
    }

    #[test]
    fn detects_single_flipped_byte() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("doc.bin"), b"aaaa").unwrap();
        let m = seal(dir.path(), None).unwrap();
        fs::write(dir.path().join("doc.bin"), b"aaab").unwrap();
        assert!(!verify(&m, dir.path()).unwrap().is_intact());
    }

    #[test]
    fn detects_deletion() {
        let dir = fixture();
        let m = seal(dir.path(), None).unwrap();
        fs::remove_file(dir.path().join("pages/02.txt")).unwrap();

        let report = verify(&m, dir.path()).unwrap();
        assert!(matches!(
            report.changes.as_slice(),
            [Change::Missing { path }] if path == "pages/02.txt"
        ));
    }

    #[test]
    fn detects_addition() {
        let dir = fixture();
        let m = seal(dir.path(), None).unwrap();
        fs::write(dir.path().join("pages/03.txt"), b"inserted later").unwrap();

        let report = verify(&m, dir.path()).unwrap();
        assert!(matches!(
            report.changes.as_slice(),
            [Change::Added { path, .. }] if path == "pages/03.txt"
        ));
    }

    #[test]
    fn detects_rename_even_with_identical_bytes() {
        let dir = fixture();
        let m = seal(dir.path(), None).unwrap();
        fs::rename(
            dir.path().join("pages/01.txt"),
            dir.path().join("pages/01-renamed.txt"),
        )
        .unwrap();

        let report = verify(&m, dir.path()).unwrap();
        assert!(!report.is_intact());
        assert_eq!(report.changes.len(), 2, "one missing, one added");
    }

    #[test]
    fn detects_swapped_file_contents() {
        // Total bytes are unchanged; only the path-to-content binding moved.
        let dir = fixture();
        let m = seal(dir.path(), None).unwrap();
        fs::write(dir.path().join("pages/01.txt"), b"second page").unwrap();
        fs::write(dir.path().join("pages/02.txt"), b"first page").unwrap();

        let report = verify(&m, dir.path()).unwrap();
        assert!(!report.is_intact(), "swap must not verify as intact");
        assert_ne!(report.expected_root, report.actual_root);
    }

    #[test]
    fn sealing_is_deterministic_and_ignores_the_manifest_itself() {
        let dir = fixture();
        let first = seal(dir.path(), None).unwrap();
        first.write_to(&dir.path().join(MANIFEST_FILENAME)).unwrap();

        let second = seal(dir.path(), None).unwrap();
        assert_eq!(first.root, second.root, "manifest must not seal itself");
        assert_eq!(first.entries, second.entries);
    }

    #[test]
    fn git_directory_is_excluded() {
        let dir = fixture();
        fs::create_dir_all(dir.path().join(".git/objects")).unwrap();
        fs::write(dir.path().join(".git/objects/abc"), b"loose object").unwrap();

        let m = seal(dir.path(), None).unwrap();
        assert!(m.entries.iter().all(|e| !e.path.starts_with(".git")));
    }

    #[test]
    fn manifest_json_round_trips() {
        let dir = fixture();
        let m = seal(dir.path(), Some("chain-of-custody note".into())).unwrap();
        let back = Manifest::from_json(&m.to_json().unwrap()).unwrap();
        assert_eq!(m, back);
        assert_eq!(back.note.as_deref(), Some("chain-of-custody note"));
    }

    #[test]
    fn self_consistency_catches_an_edited_manifest() {
        let dir = fixture();
        let mut m = seal(dir.path(), None).unwrap();
        assert!(m.is_self_consistent());

        m.entries[0].digest = Digest::of(b"forged");
        assert!(!m.is_self_consistent(), "edited entry must break the root");
    }

    #[test]
    fn version_mismatch_is_an_error_not_a_false_pass() {
        let dir = fixture();
        let mut m = seal(dir.path(), None).unwrap();
        m.version = 999;
        assert!(matches!(
            verify(&m, dir.path()),
            Err(Error::UnsupportedManifestVersion { .. })
        ));
    }

    #[test]
    fn sealing_a_file_path_is_rejected() {
        let dir = fixture();
        assert!(matches!(
            seal(&dir.path().join("README.txt"), None),
            Err(Error::NotADirectory(_))
        ));
    }

    #[test]
    fn empty_directory_seals_to_the_empty_root() {
        let dir = TempDir::new().unwrap();
        let m = seal(dir.path(), None).unwrap();
        assert!(m.entries.is_empty());
        assert!(verify(&m, dir.path()).unwrap().is_intact());
    }
}
