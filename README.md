# FAUCI-Rand-Paul-Tony-Diary

Tooling for a document whose provenance is contested — in this case, the reported
July 2026 release by Rand Paul of a diary attributed to Anthony Fauci.

**This repository takes no position on whether such a document exists, is
authentic, or says what anyone claims it says.** It contains no diary text and
will not host any. What it provides is the boring, checkable infrastructure that
a contested document needs and rarely gets:

- **Sealing** — hash a folder to a single Merkle root, so any later edit to any
  file is detectable.
- **A claim ledger** — record who asserted what, when, with citations, and
  enforce that no claim can be marked corroborated without one.

The two are deliberately separate, because they answer different questions.

## What a hash can and cannot tell you

A matching hash proves that the bytes in front of you are the bytes that were
sealed. That is genuinely useful: it catches a file quietly edited between
release and reporting, a page swapped, a document trimmed.

It proves nothing about authorship, authenticity, or context. Anyone can hash
anything. If a fabricated document is sealed on day one, it will verify happily
forever — the seal only ever attests to *unchanged*, never to *genuine*.

Authenticity is a question about sourcing, so it lives in the ledger, where
claims stay attributed to whoever made them and stay `unverified` until someone
cites something.

## Layout

```
crates/provenance-core/   sealing, verification, and the claim ledger
crates/provenance-cli/    the `provenance` command
desktop/                  Tauri desktop app (macOS/Linux/Windows)
ledger/                   the claim ledger, as JSON Lines
.claude/skills/gitkit/    the /gitkit repo-hardening skill
```

## Build and test

```bash
cargo test --workspace        # 45 tests
cargo build --release         # target/release/provenance
```

## Using the CLI

```bash
# Seal a folder as you received it.
provenance seal ./evidence --note "copy as received, from public archive"

# Later — has anything changed?
provenance verify ./evidence
```

`verify` exits `0` when intact and `1` when anything differs, so it works as a CI
gate or a cron check. It detects modification, deletion, insertion, renames, and
a swap of two files' contents (which leaves total bytes unchanged).

Keep the root hash somewhere *outside* the sealed folder. A manifest that can be
edited alongside the files it describes proves nothing.

```bash
# Record a claim. New claims are always unverified.
provenance ledger add --id "release-2026-07" \
  --actor "..." --assertion "..." --source "https://..."

provenance ledger list
provenance ledger check          # fails if any claim outran its evidence
```

## Desktop app

A window over the same code — pick a folder, seal it, verify it later.

```bash
cd desktop && npx @tauri-apps/cli build
```

On Linux this needs `libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev patchelf`.

**macOS `.dmg`:** built by the `Desktop .dmg (macOS)` workflow, since disk images
can only be created on macOS. Run it from the Actions tab, then download the
artifact. The build is unsigned, so macOS quarantines it on first open — the
"damaged" message it shows is misleading:

```bash
xattr -dr com.apple.quarantine /Applications/Provenance.app
```

## Contributing to the ledger

Entries need a citation to a primary or clearly-attributed source. A claim with
no source stays `unverified` — that is the normal, honest state for a claim, not
a defect. `provenance ledger check` rejects any entry whose status outran its
evidence, and CI runs it.

## License

Apache-2.0.
