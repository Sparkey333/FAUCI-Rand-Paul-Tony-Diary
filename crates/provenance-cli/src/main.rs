//! Command line front-end.
//!
//! Exit codes are stable so this is usable as a CI gate:
//!   0 — intact / success
//!   1 — tamper detected (verify only)
//!   2 — the command itself failed

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use provenance_core::{
    ledger::{Claim, ClaimStatus, Ledger, Source},
    manifest, Digest, Error, Manifest, MANIFEST_FILENAME,
};
use time::OffsetDateTime;

const EXIT_OK: u8 = 0;
const EXIT_TAMPERED: u8 = 1;
const EXIT_ERROR: u8 = 2;

#[derive(Parser)]
#[command(
    name = "provenance",
    about = "Seal, verify, and record claims about documents of contested provenance.",
    long_about = "Seals a directory to a Merkle root so later edits are detectable, and \
                  keeps a ledger of who claimed what.\n\n\
                  Sealing proves only that bytes have not changed since you sealed them. \
                  It does not establish that a document is authentic.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Hash a directory and write a manifest fixing its contents.
    Seal {
        /// Directory to seal.
        dir: PathBuf,
        /// Chain-of-custody note stored in the manifest.
        #[arg(long)]
        note: Option<String>,
        /// Where to write the manifest (default: <dir>/provenance.manifest.json).
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Check a directory against a previously written manifest.
    Verify {
        /// Directory to check.
        dir: PathBuf,
        /// Manifest to check against (default: <dir>/provenance.manifest.json).
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Emit the report as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Print the SHA-256 of a single file.
    Hash {
        /// File to hash.
        file: PathBuf,
    },

    /// Inspect the claim ledger.
    Ledger {
        #[command(subcommand)]
        action: LedgerAction,
    },
}

#[derive(Subcommand)]
enum LedgerAction {
    /// List recorded claims and their status.
    List {
        /// Ledger file (JSON Lines).
        #[arg(long, default_value = "ledger/claims.jsonl")]
        file: PathBuf,
        /// Show only claims with this status.
        #[arg(long)]
        status: Option<String>,
    },
    /// Append a new claim. New claims are always recorded as unverified.
    Add {
        #[arg(long, default_value = "ledger/claims.jsonl")]
        file: PathBuf,
        /// Stable identifier, e.g. "release-2026-07".
        #[arg(long)]
        id: String,
        /// Who made the assertion.
        #[arg(long)]
        actor: String,
        /// What was asserted, in reported speech.
        #[arg(long)]
        assertion: String,
        /// Citation URL. Repeatable.
        #[arg(long = "source")]
        sources: Vec<String>,
    },
    /// Validate the ledger without changing it.
    Check {
        #[arg(long, default_value = "ledger/claims.jsonl")]
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn run(cli: Cli) -> Result<u8, Error> {
    match cli.command {
        Command::Seal { dir, note, out } => cmd_seal(&dir, note, out),
        Command::Verify {
            dir,
            manifest,
            json,
        } => cmd_verify(&dir, manifest, json),
        Command::Hash { file } => cmd_hash(&file),
        Command::Ledger { action } => cmd_ledger(action),
    }
}

fn manifest_path(dir: &Path, explicit: Option<PathBuf>) -> PathBuf {
    explicit.unwrap_or_else(|| dir.join(MANIFEST_FILENAME))
}

fn cmd_seal(dir: &Path, note: Option<String>, out: Option<PathBuf>) -> Result<u8, Error> {
    let m = manifest::seal(dir, note)?;
    let target = manifest_path(dir, out);
    m.write_to(&target)?;

    println!("sealed  {} file(s)", m.entries.len());
    println!("root    {}", m.root);
    println!("written {}", target.display());
    Ok(EXIT_OK)
}

fn cmd_verify(dir: &Path, explicit: Option<PathBuf>, json: bool) -> Result<u8, Error> {
    let path = manifest_path(dir, explicit);
    let m = Manifest::read_from(&path)?;

    if !m.is_self_consistent() {
        eprintln!(
            "error: {} was edited after sealing — its root does not match its own entries",
            path.display()
        );
        return Ok(EXIT_TAMPERED);
    }

    let report = manifest::verify(&m, dir)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.summary());
        println!("expected root {}", report.expected_root);
        println!("actual   root {}", report.actual_root);
        for change in &report.changes {
            match change {
                manifest::Change::Modified { path, .. } => println!("  modified  {path}"),
                manifest::Change::Missing { path } => println!("  missing   {path}"),
                manifest::Change::Added { path, .. } => println!("  added     {path}"),
            }
        }
    }
    Ok(if report.is_intact() {
        EXIT_OK
    } else {
        EXIT_TAMPERED
    })
}

fn cmd_hash(file: &Path) -> Result<u8, Error> {
    let (digest, size) = Digest::of_file(file).map_err(|e| Error::Io {
        path: file.to_path_buf(),
        source: e,
    })?;
    println!("{digest}  {}  ({size} bytes)", file.display());
    Ok(EXIT_OK)
}

fn load_ledger(file: &Path) -> Result<Ledger, Error> {
    if !file.exists() {
        return Ok(Ledger::new());
    }
    let raw = std::fs::read_to_string(file).map_err(|e| Error::Io {
        path: file.to_path_buf(),
        source: e,
    })?;
    Ledger::from_jsonl(&raw)
}

fn cmd_ledger(action: LedgerAction) -> Result<u8, Error> {
    match action {
        LedgerAction::List { file, status } => {
            let ledger = load_ledger(&file)?;
            let wanted = status.as_deref().map(parse_status).transpose()?;

            let mut shown = 0;
            for claim in ledger.claims() {
                if wanted.is_some_and(|w| w != claim.status) {
                    continue;
                }
                shown += 1;
                println!("{:<24} [{:?}] {}", claim.id, claim.status, claim.actor);
                println!("    {}", claim.assertion);
                for s in &claim.sources {
                    println!("    source: {}", s.url);
                }
            }
            if shown == 0 {
                println!("no matching claims in {}", file.display());
            }
            Ok(EXIT_OK)
        }

        LedgerAction::Add {
            file,
            id,
            actor,
            assertion,
            sources,
        } => {
            let mut ledger = load_ledger(&file)?;
            let now = OffsetDateTime::now_utc();
            let mut claim = Claim::new(id, actor, assertion);
            for url in sources {
                claim = claim.with_source(Source {
                    url,
                    retrieved_at: now,
                    description: "cited at time of entry".into(),
                });
            }
            let id = claim.id.clone();
            ledger.add(claim)?;

            if let Some(parent) = file.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| Error::Io {
                        path: parent.to_path_buf(),
                        source: e,
                    })?;
                }
            }
            std::fs::write(&file, ledger.to_jsonl()?).map_err(|e| Error::Io {
                path: file.clone(),
                source: e,
            })?;

            println!("recorded {id} as unverified in {}", file.display());
            Ok(EXIT_OK)
        }

        LedgerAction::Check { file } => {
            let ledger = load_ledger(&file)?;
            println!("{} claim(s) valid in {}", ledger.len(), file.display());
            for status in [
                ClaimStatus::Unverified,
                ClaimStatus::Corroborated,
                ClaimStatus::Disputed,
                ClaimStatus::Withdrawn,
            ] {
                let n = ledger.by_status(status).count();
                if n > 0 {
                    println!("  {status:?}: {n}");
                }
            }
            Ok(EXIT_OK)
        }
    }
}

fn parse_status(s: &str) -> Result<ClaimStatus, Error> {
    match s.to_ascii_lowercase().as_str() {
        "unverified" => Ok(ClaimStatus::Unverified),
        "corroborated" => Ok(ClaimStatus::Corroborated),
        "disputed" => Ok(ClaimStatus::Disputed),
        "withdrawn" => Ok(ClaimStatus::Withdrawn),
        other => Err(Error::InvalidClaim(format!(
            "unknown status {other:?} (expected unverified, corroborated, disputed, or withdrawn)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_parsing_is_case_insensitive() {
        assert_eq!(
            parse_status("Corroborated").unwrap(),
            ClaimStatus::Corroborated
        );
        assert_eq!(parse_status("UNVERIFIED").unwrap(), ClaimStatus::Unverified);
    }

    #[test]
    fn unknown_status_is_rejected() {
        assert!(parse_status("true").is_err());
    }

    #[test]
    fn manifest_path_defaults_inside_the_directory() {
        let dir = Path::new("/tmp/evidence");
        assert_eq!(
            manifest_path(dir, None),
            Path::new("/tmp/evidence").join(MANIFEST_FILENAME)
        );
        assert_eq!(
            manifest_path(dir, Some(PathBuf::from("/elsewhere/m.json"))),
            PathBuf::from("/elsewhere/m.json")
        );
    }

    #[test]
    fn missing_ledger_file_loads_as_empty() {
        assert!(load_ledger(Path::new("/nonexistent/claims.jsonl"))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
