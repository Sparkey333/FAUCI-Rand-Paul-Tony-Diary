//! Tauri command surface over `provenance-core`.
//!
//! The bridge stays deliberately thin: every command is a direct call into the
//! core crate, so the logic that decides whether something is intact is the same
//! code the CLI and the test suite exercise. Errors are stringified at this
//! boundary because that is what crosses into JavaScript.

use std::path::PathBuf;

use provenance_core::{manifest, Digest, Manifest, VerifyReport, MANIFEST_FILENAME};
use serde::Serialize;

#[derive(Serialize)]
pub struct SealOutcome {
    pub root: String,
    pub file_count: usize,
    pub manifest_path: String,
    pub sealed_at: String,
}

#[derive(Serialize)]
pub struct FileHash {
    pub digest: String,
    pub size: u64,
}

/// Hash every file under `path` and write a manifest beside them.
#[tauri::command]
fn seal_directory(path: String, note: Option<String>) -> Result<SealOutcome, String> {
    let dir = PathBuf::from(&path);
    let note = note.filter(|n| !n.trim().is_empty());

    let m = manifest::seal(&dir, note).map_err(|e| e.to_string())?;
    let target = dir.join(MANIFEST_FILENAME);
    m.write_to(&target).map_err(|e| e.to_string())?;

    Ok(SealOutcome {
        root: m.root.to_hex(),
        file_count: m.entries.len(),
        manifest_path: target.display().to_string(),
        sealed_at: m.sealed_at.to_string(),
    })
}

/// Compare `path` against the manifest sitting inside it.
#[tauri::command]
fn verify_directory(path: String) -> Result<VerifyReport, String> {
    let dir = PathBuf::from(&path);
    let manifest_path = dir.join(MANIFEST_FILENAME);

    if !manifest_path.exists() {
        return Err(format!(
            "no {MANIFEST_FILENAME} in this folder — seal it first, or open the folder you sealed"
        ));
    }

    let m = Manifest::read_from(&manifest_path).map_err(|e| e.to_string())?;
    if !m.is_self_consistent() {
        return Err(
            "the manifest was edited after sealing — its root does not match its own entries"
                .to_string(),
        );
    }
    manifest::verify(&m, &dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn hash_file(path: String) -> Result<FileHash, String> {
    let (digest, size) = Digest::of_file(&PathBuf::from(&path)).map_err(|e| e.to_string())?;
    Ok(FileHash {
        digest: digest.to_hex(),
        size,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            seal_directory,
            verify_directory,
            hash_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Provenance window");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Exercises the command functions directly — no webview needed, so this
    /// runs on any platform including CI containers without a display.
    #[test]
    fn seal_then_verify_through_the_command_surface() {
        let dir = std::env::temp_dir().join("provenance-desktop-test-seal");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("doc.txt"), b"contents").unwrap();

        let path = dir.display().to_string();
        let sealed = seal_directory(path.clone(), Some("note".into())).unwrap();
        assert_eq!(sealed.file_count, 1);

        let report = verify_directory(path.clone()).unwrap();
        assert!(report.is_intact());

        fs::write(dir.join("doc.txt"), b"contents, altered").unwrap();
        assert!(!verify_directory(path).unwrap().is_intact());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn verifying_an_unsealed_folder_explains_itself() {
        let dir = std::env::temp_dir().join("provenance-desktop-test-unsealed");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let err = verify_directory(dir.display().to_string()).unwrap_err();
        assert!(err.contains("seal it first"), "unhelpful error: {err}");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hashing_a_missing_file_is_an_error() {
        assert!(hash_file("/nonexistent/file.txt".into()).is_err());
    }
}
