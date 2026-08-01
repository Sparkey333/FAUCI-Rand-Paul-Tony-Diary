//! Content addressing and Merkle aggregation.
//!
//! Every hash in this crate is domain-separated: a leaf and an interior node can
//! never produce the same preimage, so a set of file digests cannot be re-arranged
//! into a tree that forges the same root.

use std::fmt;
use std::path::Path;

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

pub const DIGEST_LEN: usize = 32;

const DOMAIN_LEAF: u8 = 0x00;
const DOMAIN_NODE: u8 = 0x01;
const EMPTY_ROOT_SEED: &[u8] = b"provenance:empty-merkle-v1";

/// A SHA-256 digest, serialized as a lowercase hex string.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Digest([u8; DIGEST_LEN]);

impl Digest {
    pub fn of(bytes: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(bytes);
        Self(h.finalize().into())
    }

    /// Hash a file by streaming it, so a multi-gigabyte scan never buys the
    /// whole file into memory.
    pub fn of_file(path: &Path) -> std::io::Result<(Self, u64)> {
        use std::io::Read;
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 64 * 1024];
        let mut size = 0u64;
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            size += n as u64;
        }
        Ok((Self(hasher.finalize().into()), size))
    }

    pub fn as_bytes(&self) -> &[u8; DIGEST_LEN] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut out = [0u8; DIGEST_LEN];
        hex::decode_to_slice(s, &mut out)?;
        Ok(Self(out))
    }

    /// Bind a path to its content digest. Renaming a file changes the leaf even
    /// when the bytes are identical, so a reshuffled release is still detected.
    ///
    /// The path length is prefixed so that `("ab", "c")` and `("a", "bc")`
    /// cannot collide.
    pub fn leaf(path: &str, content: Digest) -> Self {
        let mut h = Sha256::new();
        h.update([DOMAIN_LEAF]);
        h.update((path.len() as u64).to_le_bytes());
        h.update(path.as_bytes());
        h.update(content.0);
        Self(h.finalize().into())
    }

    fn node(left: Digest, right: Digest) -> Self {
        let mut h = Sha256::new();
        h.update([DOMAIN_NODE]);
        h.update(left.0);
        h.update(right.0);
        Self(h.finalize().into())
    }
}

/// Fold leaf digests into a single root.
///
/// Leaves are sorted so the root is independent of walk order. An odd node is
/// promoted to the next level rather than duplicated — duplicating it is the
/// classic Merkle malleability bug (CVE-2012-2459), where two distinct leaf sets
/// yield the same root.
pub fn merkle_root(leaves: &[Digest]) -> Digest {
    if leaves.is_empty() {
        return Digest::of(EMPTY_ROOT_SEED);
    }
    let mut level: Vec<Digest> = leaves.to_vec();
    level.sort_unstable();

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        let mut chunks = level.chunks_exact(2);
        for pair in &mut chunks {
            next.push(Digest::node(pair[0], pair[1]));
        }
        if let Some(&odd) = chunks.remainder().first() {
            next.push(odd);
        }
        level = next;
    }
    level[0]
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({})", self.to_hex())
    }
}

impl Serialize for Digest {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Digest::from_hex(&s).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: u8) -> Digest {
        Digest::of(&[n])
    }

    #[test]
    fn digest_matches_known_sha256_vector() {
        // NIST vector for "abc".
        assert_eq!(
            Digest::of(b"abc").to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hex_round_trips() {
        let original = Digest::of(b"round trip");
        assert_eq!(Digest::from_hex(&original.to_hex()).unwrap(), original);
    }

    #[test]
    fn rejects_malformed_hex() {
        assert!(Digest::from_hex("nonsense").is_err());
        assert!(Digest::from_hex("ab").is_err(), "wrong length must fail");
    }

    #[test]
    fn leaf_binds_path_to_content() {
        let content = Digest::of(b"same bytes");
        assert_ne!(
            Digest::leaf("diary/page-01.pdf", content),
            Digest::leaf("diary/page-02.pdf", content),
            "renaming a file must change its leaf"
        );
    }

    #[test]
    fn leaf_path_length_prefix_prevents_collision() {
        let c = Digest::of(b"x");
        assert_ne!(Digest::leaf("ab", c), Digest::leaf("a", c));
    }

    #[test]
    fn leaf_and_node_domains_are_separated() {
        let a = d(1);
        let b = d(2);
        // A two-leaf root must not equal a leaf built from the same material.
        assert_ne!(merkle_root(&[a, b]), Digest::leaf("", a));
    }

    #[test]
    fn root_is_order_independent() {
        let (a, b, c) = (d(1), d(2), d(3));
        assert_eq!(merkle_root(&[a, b, c]), merkle_root(&[c, a, b]));
    }

    #[test]
    fn root_changes_when_any_leaf_changes() {
        let base = merkle_root(&[d(1), d(2), d(3)]);
        assert_ne!(base, merkle_root(&[d(1), d(2), d(4)]));
    }

    #[test]
    fn empty_and_single_roots_are_distinct_and_stable() {
        let empty = merkle_root(&[]);
        assert_eq!(empty, merkle_root(&[]));
        assert_eq!(merkle_root(&[d(9)]), d(9), "single leaf is its own root");
        assert_ne!(empty, d(9));
    }

    #[test]
    fn odd_leaf_is_promoted_not_duplicated() {
        // With duplication, [a, b, c] and [a, b, c, c] collide. Promotion keeps
        // them distinct.
        let (a, b, c) = (d(1), d(2), d(3));
        assert_ne!(merkle_root(&[a, b, c]), merkle_root(&[a, b, c, c]));
    }

    #[test]
    fn serde_round_trips_as_hex_string() {
        let original = Digest::of(b"serde");
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, format!("\"{}\"", original.to_hex()));
        assert_eq!(serde_json::from_str::<Digest>(&json).unwrap(), original);
    }
}
