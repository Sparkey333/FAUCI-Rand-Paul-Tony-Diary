//! Tooling for documents whose provenance is contested.
//!
//! # What this crate does and does not do
//!
//! It answers a narrow, mechanical question: **has this artifact changed since
//! it was sealed?** A sealed manifest fixes a set of files to a single Merkle
//! root, and verification reports any byte that moved.
//!
//! It deliberately does *not* answer whether an artifact is authentic. Hashing
//! a file proves only that you hold the same bytes someone else hashed; it
//! proves nothing about where those bytes came from. Authenticity is a question
//! about sourcing, and that lives in the [`ledger`] module — where claims are
//! recorded as claims, attributed to whoever made them, and cannot be promoted
//! out of `Unverified` without a citation.
//!
//! Keeping those two things in separate modules is the point of the design.

#![forbid(unsafe_code)]

pub mod digest;
pub mod ledger;
pub mod manifest;

use std::path::PathBuf;

pub use digest::{merkle_root, Digest};
pub use ledger::{Claim, ClaimStatus, Ledger, Source};
pub use manifest::{seal, verify, Change, Entry, Manifest, VerifyReport, MANIFEST_FILENAME};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("walking the tree failed: {0}")]
    Walk(String),

    #[error("serialization failed: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("manifest version {found} is not supported (this build understands {supported})")]
    UnsupportedManifestVersion { found: u32, supported: u32 },

    #[error("claim {id} cannot be marked {status} with no sources cited")]
    UncorroboratedStatus { id: String, status: String },

    #[error("duplicate claim id: {0}")]
    DuplicateClaimId(String),

    #[error("invalid claim: {0}")]
    InvalidClaim(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// The end-to-end story the crate exists to support: seal a received copy,
    /// record who claimed what about it, and later detect a quiet edit.
    #[test]
    fn seal_record_and_detect_a_later_edit() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("received.txt"), b"as received").unwrap();

        let sealed = seal(dir.path(), Some("copy as received".into())).unwrap();
        assert!(verify(&sealed, dir.path()).unwrap().is_intact());

        let mut ledger = Ledger::new();
        ledger
            .add(Claim::new(
                "release-2026-07",
                "unattributed",
                "A document was described publicly as released.",
            ))
            .unwrap();
        // Nothing has been checked, so it stays unverified.
        assert_eq!(
            ledger.get("release-2026-07").unwrap().status,
            ClaimStatus::Unverified
        );

        fs::write(dir.path().join("received.txt"), b"as received, edited").unwrap();
        let report = verify(&sealed, dir.path()).unwrap();
        assert!(!report.is_intact());
        assert!(report.summary().starts_with("TAMPERED"));
    }
}
