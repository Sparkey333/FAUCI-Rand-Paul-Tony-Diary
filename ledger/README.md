# Claim ledger

`claims.jsonl` records **who asserted what**, one JSON object per line so each
entry is a single diffable line in git history.

It is currently empty. That is the correct starting state: an entry should be
added when there is a citation to attach to it, not before.

## Rules

- A new claim is always `unverified`. Nothing else is honest for an assertion
  nobody has checked.
- A claim cannot be `corroborated`, `disputed`, or `withdrawn` with an empty
  `sources` array. `provenance ledger check` enforces this at load time, so
  hand-editing a status into the file fails rather than silently passing.
- `assertion` is reported speech — what someone said, not what is true. Write
  "X stated that…", never the bare fact.
- `actor` should be as precise as the sources support. If a claim was reported
  without clear attribution, say so rather than guessing at a name.

## Fields

| Field | Meaning |
|---|---|
| `id` | Stable identifier, e.g. `release-2026-07` |
| `recorded_at` | RFC 3339 timestamp of when the entry was made |
| `actor` | Who made the assertion |
| `assertion` | What was asserted, in reported speech |
| `status` | `unverified` \| `corroborated` \| `disputed` \| `withdrawn` |
| `sources` | `{ url, retrieved_at, description }` — a citation, not a proof |

## Adding an entry

```bash
provenance ledger add \
  --id "release-2026-07" \
  --actor "<who said it>" \
  --assertion "<what they said, in reported speech>" \
  --source "https://<primary or clearly attributed source>"
```

Prefer primary sources — a congressional record, a floor transcript, an official
statement, an archived original. A secondary report is acceptable when it is the
best available, but note that in the source description.

A `corroborated` status means *at least one independent source supports it*. It
does not mean the underlying document is genuine; no ledger entry can establish
that.
