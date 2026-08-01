---
name: byok
description: Build or update a full BYOK (bring-your-own-key) settings tab — commercial, cloud, aggregator, media, and local/Ollama/self-hosted LLM providers — each with a working link to get a key, paired with a Feature Parity tab that tracks what every provider can actually do. Use when adding provider/API-key configuration to an app, adding a new LLM provider, building a model-settings or integrations screen, or auditing provider capability coverage.
---

# BYOK tab

A BYOK screen looks trivial and usually is not. The failure modes are boring and
all of them erode trust: dead "get a key" links, a capability table nobody has
re-checked in a year, a key accidentally committed, or a local-model user who
cannot find where to put a base URL because the form assumes an API key.

The reference implementation in `reference/` is a working two-tab screen. Adapt
it rather than starting from scratch.

```
reference/providers.json   the single source of truth
reference/index.html       both tabs
reference/app.js           renders both from the registry
reference/styles.css       light + dark
scripts/check-links.py     structural validation + link rot check (run in CI)
```

Preview it with `python3 -m http.server` from `reference/` — `fetch` will not
read `providers.json` over `file://`.

---

## The one rule that shapes everything

**One registry, two views.** `providers.json` drives the keys tab *and* the
parity table. Adding a provider in one place makes it appear in both, correctly
categorised, with its links and capability row already wired.

The moment those are two lists, they drift: a provider gets a key field but
never a parity row, and the table quietly under-reports what the app supports.
Do not split them.

## Phase 1 — Cover the whole spectrum

"BYOK" usually gets built for two commercial vendors and then stalls. A complete
tab spans five kinds of provider, and each needs different affordances:

| Category | Auth shape | The affordance people actually need |
|---|---|---|
| **Commercial** (Anthropic, OpenAI, Google AI Studio, Mistral, Cohere, xAI, DeepSeek) | Bearer key | Paste field + get-key link |
| **Cloud platform** (Azure OpenAI, AWS Bedrock, Vertex AI) | Platform IAM, *not* a bearer key | Region/resource/deployment fields — a single key box is wrong here |
| **Aggregator & open-model hosts** (OpenRouter, Together, Groq, Fireworks, DeepInfra, Hugging Face, Perplexity, Cerebras, NVIDIA NIM) | Bearer key | Same as commercial; note capabilities vary by routed model |
| **Media generation** (Higgsfield, Replicate, ElevenLabs) | Bearer key, often credit-metered | Say it is credits, not tokens |
| **Local & self-hosted** (Ollama, llama.cpp, LM Studio, vLLM, LocalAI, Jan, any OpenAI-compatible server) | **None** | Editable base URL + a *Test* button. Never show a key field |

Local providers are the ones most often done badly. `requiresKey: false` should
swap the key input for a base-URL input and a liveness probe against a
provider-specific health path (`/api/tags` for Ollama, `/health` for llama.cpp
and vLLM, `/models` for LM Studio and Jan). Always include a generic
"Custom OpenAI-compatible server" entry — reverse proxies, gateways, and a
colleague's box on the LAN all land there.

## Phase 2 — Links that actually work

Every entry carries `keyUrl` (where to obtain credentials — for local runtimes,
the install page) and `docsUrl`.

Console URLs get reorganised constantly, so **verifying them once by hand is
worthless.** `scripts/check-links.py` validates the registry's structure and
probes every link; wire it into CI so a moved console page fails a build instead
of silently 404-ing for users.

It distinguishes three outcomes, which matters:

- **Broken** — a real 404/DNS failure. Fails the build.
- **Soft** — 401/403/405/429/503, usually a WAF refusing an automated client
  rather than a dead page. Reported, does not fail unless `--strict`.
- **Blocked egress** — if the network denies CONNECT to a host, that is *your
  sandbox*, not a broken link. Never record a policy denial as a dead link.

## Phase 3 — Handle keys correctly

- **Never write a key into the repository**, into `providers.json`, or into any
  file the app version-controls. The registry holds *metadata only* — no secrets.
- Browser context: `localStorage`, and say so in the UI. Desktop: the OS keychain
  (Keychain / Credential Manager / libsecret). The reference isolates this in a
  `store` object with two methods — swap it, do not scatter it.
- Key inputs are `type="password"` with a Show toggle and `autocomplete="off"`.
- **Validate the prefix at entry.** Pasting an OpenAI key into the Anthropic
  field is the single most common BYOK mistake, and the resulting 401 explains
  nothing. A `keyPrefix` check catches it where the user can still see what they
  did.
- Never log a key, never put one in a URL query, never include one in telemetry
  or an error report.

## Phase 4 — The Feature Parity tab

This is the tracking surface, and the keys tab should link to it.

Columns: streaming, tool calling, vision, structured output, embeddings, prompt
caching, reasoning. Rows: every provider in the registry.

Three things keep it honest:

1. **`null` means unknown, and renders as `?`** — visibly different from a
   supported `✓` or an unsupported `·`. An honest gap beats a guess that reads as
   fact. Never fill a cell you have not checked.
2. **A `lastReviewed` date, shown in the UI**, with the standing warning that
   capability data decays. Every row links to that provider's docs so
   re-verification is one click.
3. **Per-column coverage counts**, so a thin column ("11/29 Cache") is obvious
   without reading the grid.

For aggregators, a `✓` means the ceiling across models they route to — an
individual model may do less. Say so in the legend; otherwise the row overstates.

## Phase 5 — Wire it in

```yaml
- name: Validate provider registry and check links
  run: python3 .claude/skills/byok/scripts/check-links.py
```

Then confirm the screen actually renders — a registry that parses can still
produce an empty tab. Serve it, drive it headlessly, and assert counts rather
than eyeballing:

```js
const cards = await page.locator(".card").count();   // must equal provider count
const rows  = await page.locator("#parity tbody tr").count();
```

## Adding a provider

1. Append an object to `providers.json`: `id`, `name`, `category`, `keyUrl`,
   `docsUrl`, `envVar`, `baseUrl`, `capabilities`. Add `requiresKey: false` and a
   `healthPath` for anything local; add `keyPrefix` where the vendor has a
   recognisable one.
2. Leave unchecked capabilities as `null`.
3. Bump `lastReviewed`.
4. Run `check-links.py`. Both tabs pick the provider up with no other change.

## Finishing

Report which links were actually verified and which could not be reached from
the environment you ran in. "The CI job checks these on every push" is a true and
useful claim; "I verified all 58 links" is not, if the network refused half of
them.
