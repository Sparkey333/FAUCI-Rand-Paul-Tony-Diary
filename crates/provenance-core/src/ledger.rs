//! A ledger of *claims*, kept deliberately separate from evidence.
//!
//! The distinction this module enforces: recording that someone asserted
//! something is not the same as establishing that the assertion is true. A claim
//! starts `Unverified` and cannot leave that state without at least one cited
//! source, so the type system makes an uncorroborated upgrade impossible rather
//! than merely discouraged.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    /// Asserted publicly; nothing has been checked. The only status a claim may
    /// hold with no sources attached.
    #[default]
    Unverified,
    /// Independently supported by at least one cited source.
    Corroborated,
    /// At least one cited source contradicts it.
    Disputed,
    /// Retracted by the party who made it.
    Withdrawn,
}

impl ClaimStatus {
    /// Every status except `Unverified` is an assertion *about the evidence*,
    /// and so requires evidence.
    pub fn requires_sources(self) -> bool {
        !matches!(self, ClaimStatus::Unverified)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Where the claim can be checked. A citation, not a proof.
    pub url: String,
    #[serde(with = "time::serde::rfc3339")]
    pub retrieved_at: OffsetDateTime,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub recorded_at: OffsetDateTime,
    /// Who made the assertion, attributed as precisely as the sources allow.
    pub actor: String,
    /// What was asserted — reported speech, never stated as fact by this repo.
    pub assertion: String,
    #[serde(default)]
    pub status: ClaimStatus,
    #[serde(default)]
    pub sources: Vec<Source>,
}

impl Claim {
    pub fn new(
        id: impl Into<String>,
        actor: impl Into<String>,
        assertion: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            recorded_at: OffsetDateTime::now_utc(),
            actor: actor.into(),
            assertion: assertion.into(),
            status: ClaimStatus::Unverified,
            sources: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: Source) -> Self {
        self.sources.push(source);
        self
    }

    /// Move a claim to a new status. Rejected unless the evidence supports it.
    pub fn set_status(&mut self, status: ClaimStatus) -> Result<(), Error> {
        if status.requires_sources() && self.sources.is_empty() {
            return Err(Error::UncorroboratedStatus {
                id: self.id.clone(),
                status: format!("{status:?}"),
            });
        }
        self.status = status;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.id.trim().is_empty() {
            return Err(Error::InvalidClaim("id must not be empty".into()));
        }
        if self.assertion.trim().is_empty() {
            return Err(Error::InvalidClaim(format!(
                "claim {} has an empty assertion",
                self.id
            )));
        }
        if self.status.requires_sources() && self.sources.is_empty() {
            return Err(Error::UncorroboratedStatus {
                id: self.id.clone(),
                status: format!("{:?}", self.status),
            });
        }
        Ok(())
    }
}

/// An append-oriented set of claims, persisted as JSON Lines so that each record
/// is one diffable line in git history.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    claims: Vec<Claim>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, claim: Claim) -> Result<(), Error> {
        claim.validate()?;
        if self.claims.iter().any(|c| c.id == claim.id) {
            return Err(Error::DuplicateClaimId(claim.id));
        }
        self.claims.push(claim);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Claim> {
        self.claims.iter().find(|c| c.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Claim> {
        self.claims.iter_mut().find(|c| c.id == id)
    }

    pub fn claims(&self) -> &[Claim] {
        &self.claims
    }

    pub fn len(&self) -> usize {
        self.claims.len()
    }

    pub fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }

    pub fn by_status(&self, status: ClaimStatus) -> impl Iterator<Item = &Claim> {
        self.claims.iter().filter(move |c| c.status == status)
    }

    /// Serialize as JSON Lines. Blank lines are never emitted.
    pub fn to_jsonl(&self) -> Result<String, Error> {
        let mut out = String::new();
        for claim in &self.claims {
            out.push_str(&serde_json::to_string(claim)?);
            out.push('\n');
        }
        Ok(out)
    }

    /// Parse JSON Lines, skipping blank lines and `#` comments. Every record is
    /// validated, so a malformed ledger fails loudly at load rather than
    /// producing quietly wrong counts later.
    pub fn from_jsonl(s: &str) -> Result<Self, Error> {
        let mut ledger = Ledger::new();
        for (i, line) in s.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let claim: Claim = serde_json::from_str(line)
                .map_err(|e| Error::InvalidClaim(format!("line {}: {e}", i + 1)))?;
            ledger.add(claim)?;
        }
        Ok(ledger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Source {
        Source {
            url: "https://www.congress.gov/".into(),
            retrieved_at: OffsetDateTime::now_utc(),
            description: "primary record".into(),
        }
    }

    #[test]
    fn new_claims_start_unverified() {
        let c = Claim::new("c-1", "Example Actor", "A document was released.");
        assert_eq!(c.status, ClaimStatus::Unverified);
        assert!(c.sources.is_empty());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn cannot_corroborate_without_a_source() {
        let mut c = Claim::new("c-1", "Example Actor", "A document was released.");
        let err = c.set_status(ClaimStatus::Corroborated).unwrap_err();
        assert!(matches!(err, Error::UncorroboratedStatus { .. }));
        assert_eq!(c.status, ClaimStatus::Unverified, "status must not change");
    }

    #[test]
    fn cannot_dispute_without_a_source_either() {
        let mut c = Claim::new("c-1", "Example Actor", "A document was released.");
        assert!(c.set_status(ClaimStatus::Disputed).is_err());
    }

    #[test]
    fn corroborating_with_a_source_succeeds() {
        let mut c =
            Claim::new("c-1", "Example Actor", "A document was released.").with_source(source());
        c.set_status(ClaimStatus::Corroborated).unwrap();
        assert_eq!(c.status, ClaimStatus::Corroborated);
    }

    #[test]
    fn withdrawing_still_requires_evidence() {
        // A retraction is itself a claim about the record, so it needs a cite.
        let mut c = Claim::new("c-1", "Example Actor", "A document was released.");
        assert!(c.set_status(ClaimStatus::Withdrawn).is_err());
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut l = Ledger::new();
        l.add(Claim::new("c-1", "A", "first")).unwrap();
        assert!(matches!(
            l.add(Claim::new("c-1", "B", "second")),
            Err(Error::DuplicateClaimId(_))
        ));
        assert_eq!(l.len(), 1);
    }

    #[test]
    fn empty_assertion_is_rejected() {
        let mut l = Ledger::new();
        assert!(l.add(Claim::new("c-1", "A", "   ")).is_err());
    }

    #[test]
    fn empty_id_is_rejected() {
        let mut l = Ledger::new();
        assert!(l.add(Claim::new("  ", "A", "something")).is_err());
    }

    #[test]
    fn jsonl_round_trips() {
        let mut l = Ledger::new();
        l.add(Claim::new("c-1", "Actor One", "first assertion"))
            .unwrap();
        let mut c2 = Claim::new("c-2", "Actor Two", "second assertion").with_source(source());
        c2.set_status(ClaimStatus::Corroborated).unwrap();
        l.add(c2).unwrap();

        let back = Ledger::from_jsonl(&l.to_jsonl().unwrap()).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back.get("c-2").unwrap().status, ClaimStatus::Corroborated);
        assert_eq!(back.get("c-1").unwrap().status, ClaimStatus::Unverified);
    }

    #[test]
    fn jsonl_skips_blanks_and_comments() {
        let mut l = Ledger::new();
        l.add(Claim::new("c-1", "Actor", "assertion")).unwrap();
        let text = format!("# a comment\n\n{}\n\n", l.to_jsonl().unwrap().trim());
        assert_eq!(Ledger::from_jsonl(&text).unwrap().len(), 1);
    }

    #[test]
    fn a_hand_edited_status_fails_to_load() {
        // Someone marks a claim corroborated in the file without citing anything.
        let forged = r#"{"id":"c-1","recorded_at":"2026-07-01T00:00:00Z","actor":"A","assertion":"x","status":"corroborated","sources":[]}"#;
        assert!(matches!(
            Ledger::from_jsonl(forged),
            Err(Error::UncorroboratedStatus { .. })
        ));
    }

    #[test]
    fn malformed_line_reports_its_number() {
        let err = Ledger::from_jsonl("{not json}").unwrap_err();
        assert!(err.to_string().contains("line 1"), "got: {err}");
    }

    #[test]
    fn filters_by_status() {
        let mut l = Ledger::new();
        l.add(Claim::new("c-1", "A", "one")).unwrap();
        let mut c2 = Claim::new("c-2", "B", "two").with_source(source());
        c2.set_status(ClaimStatus::Corroborated).unwrap();
        l.add(c2).unwrap();

        assert_eq!(l.by_status(ClaimStatus::Unverified).count(), 1);
        assert_eq!(l.by_status(ClaimStatus::Corroborated).count(), 1);
        assert_eq!(l.by_status(ClaimStatus::Disputed).count(), 0);
    }

    #[test]
    fn empty_ledger_serializes_to_nothing() {
        assert_eq!(Ledger::new().to_jsonl().unwrap(), "");
        assert!(Ledger::from_jsonl("").unwrap().is_empty());
    }
}
