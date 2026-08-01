---
name: gitkit
description: Bring a repository up to a shippable baseline — CI, lint and format gates, tests, PR template, branch hygiene, release automation, and desktop app packaging (including building a macOS .dmg and getting it onto the user's Mac to test). Use when the user asks to "gitkit" a repo, ramp up or harden a bare repository, set up CI/release plumbing, or produce and test a distributable desktop build.
---

# gitkit

Take a repository from wherever it is to a state where a stranger could clone it,
build it, and trust the result. Work through the phases in order; each one
assumes the previous passed.

**The rule that matters:** every gate this skill adds must pass locally before it
is pushed. Adding a CI job and discovering in CI that it fails is the failure
mode this skill exists to prevent. Run the gate, fix what it finds, then commit.

---

## Phase 0 — Survey before touching anything

Never assume the repo is empty or that a tool is missing. Check:

```bash
git log --oneline -10 && git status --short
ls -a                       # existing CI, hooks, configs, lockfiles
```

- Read any `CLAUDE.md`, `CONTRIBUTING.md`, or existing workflow files first —
  match the conventions already there rather than imposing new ones.
- Identify the language and toolchain from lockfiles, not from file extensions.
- **Check what already exists before adding it.** A second linter config or a
  duplicate CI job is worse than none. If the repo already has a working gate,
  improve it; do not add a parallel one.

## Phase 1 — Make the build reproducible

1. Pin the toolchain (`rust-toolchain.toml`, `.nvmrc`, `.python-version`).
2. Commit the lockfile. For a library, this is still worth it for CI determinism.
3. Confirm a clean clone builds:
   ```bash
   git clean -xdn        # dry run first — look before you delete
   ```
4. Add a `.gitignore` covering build output, editor droppings, and any artifact
   the tools here generate.

## Phase 2 — Tests and gates, locally first

Add, in this order, running each locally before moving on:

| Gate | Rust | Node | Python |
|---|---|---|---|
| Format | `cargo fmt --all --check` | `prettier --check .` | `ruff format --check` |
| Lint | `cargo clippy --all-targets` | `eslint .` | `ruff check` |
| Test | `cargo test --workspace` | `npm test` | `pytest` |

Treat lint warnings as errors in CI (`RUSTFLAGS: -D warnings`, `--max-warnings 0`).
Do this *after* the code is clean, never before — otherwise the first CI run is
red for reasons unrelated to the change.

Where a program's **exit codes** are part of its interface, assert them in CI
with an actual invocation. A test that only checks stdout will not notice a tool
that reports failure while exiting 0.

## Phase 3 — CI

One workflow, jobs split by concern, with:

- `concurrency` + `cancel-in-progress` so pushes supersede each other.
- A dependency cache keyed to the lockfile.
- `workflow_dispatch` so it can be run by hand.
- Branch filters that include the working branch, not just the default branch.

If a job needs system packages (webview, image libs, database headers), install
them explicitly in the job. Do not rely on whatever the runner image happens to
ship this month.

## Phase 4 — Repository furniture

- `.github/pull_request_template.md` — a layout to fill in, kept short.
- `README.md` that states what the thing is, how to build it, and how to run the
  tests. Include the honest limitations; a README that oversells costs trust.
- A license file if there isn't one.

## Phase 5 — Desktop packaging and the macOS .dmg

Only if the repo ships a desktop app. Covers Tauri; the shape is the same for
Electron with different commands.

### 5a. Establish what can be built where

This is the step people skip, and it wastes the most time. **A `.dmg` can only be
created on macOS** — `hdiutil` does not exist elsewhere, and cross-compiling does
not help. So:

```bash
uname -s          # Darwin = can build .dmg locally; Linux = cannot
```

On Linux you can still fully validate everything *except* the disk image:

```bash
# Tauri needs the system webview to compile at all.
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev patchelf
cd desktop && npx --yes @tauri-apps/cli@latest build
```

That produces `.deb` / `.rpm` / `.AppImage` through the *same* bundling pipeline
the `.dmg` uses. If it succeeds, the config is sound and the macOS build is very
likely to work. Set `"targets": "all"` in `tauri.conf.json` so each platform
emits its own native bundles from one config.

### 5b. Verify the app actually runs, headless

Compiling is not evidence the app works. On a machine with no display:

```bash
sudo apt-get install -y xvfb imagemagick
Xvfb :99 -screen 0 1200x900x24 &
DISPLAY=:99 WEBKIT_DISABLE_COMPOSITING_MODE=1 ./path/to/app &
sleep 12
DISPLAY=:99 import -window root /tmp/app.png    # then look at it
```

Read the screenshot. Two failures only a screenshot catches:

- **A blank or unstyled window** — the frontend assets did not get embedded.
- **A window that renders but does nothing** — the JS↔native bridge did not
  resolve. Tauri v2 only exposes `window.__TAURI__` when
  `"withGlobalTauri": true` is set, and without a bundler that global is the
  only way in. Make the failure visible rather than silent:

  ```js
  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke) { /* render a visible error banner */ }
  ```

  Then a screenshot distinguishes "working" from "silently inert" at a glance.

Beware `pkill -f <appname>` — `-f` matches the whole command line, so it will
match and kill the very shell running it. Kill by PID.

### 5c. Build the .dmg in CI

On `workflow_dispatch` and `v*` tags, `runs-on: macos-14`:

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    targets: aarch64-apple-darwin,x86_64-apple-darwin
- run: npx --yes @tauri-apps/cli@latest build --target universal-apple-darwin
  working-directory: desktop
```

Then prove the artifact is what it claims:

```bash
lipo -archs "$APP/Contents/MacOS/<binary>"   # must list arm64 AND x86_64

# `open -a` looks up an application NAME via Launch Services and will fail with
# "Unable to find application named ..." when given a path. Drop the -a.
open "$APP" && sleep 12 && pgrep -x <binary>   # must still be running
```

Universal matters: an arm64-only build silently fails to launch on Intel Macs.
Upload with `actions/upload-artifact` and `if-no-files-found: error`, so a
missing bundle fails the run instead of producing an empty download.

**Order the upload before the launch check.** Steps stop at the first failure,
so a smoke test placed first turns a flaky launch on a headless runner into a
run with no downloadable artifact at all — the build worked and the user still
got nothing. Upload first, then verify: the job still goes red, but the image
is there to inspect.

### 5d. Get it onto the user's Mac and test it

The artifact is a zip. Tell the user, explicitly:

1. Actions tab → the run → **Artifacts** → download → unzip → open the `.dmg`.
2. Drag the app to `/Applications`.
3. **It will not open on first try.** An unsigned build is quarantined by
   Gatekeeper, and the error macOS shows ("damaged and can't be opened") is
   misleading — the build is fine, it is just unsigned:
   ```bash
   xattr -dr com.apple.quarantine /Applications/<App>.app
   ```
   Or right-click the app → Open → Open.

Say this up front. Users reasonably read the Gatekeeper message as a broken
download and report a bug that isn't one.

Proper signing needs an Apple Developer ID (paid). If the user has one, add
`APPLE_CERTIFICATE`, `APPLE_SIGNING_IDENTITY`, and notarization credentials as
repository secrets. If not, the quarantine step above is the correct workaround —
do not pretend the build is signed.

**Never claim a `.dmg` was tested on macOS from a Linux session.** State what was
actually verified and what still needs a Mac.

## Phase 6 — Ship

```bash
git switch -c <branch>          # never commit straight to the default branch
git add -A && git commit
git push -u origin <branch>
```

Retry a failed push up to 4 times with exponential backoff (2s, 4s, 8s, 16s) —
but only for network errors. A rejected push is a real conflict; fetch and
reconcile instead.

Open a **draft** PR. Then drive it to green: a CI failure on a PR opened here is
this skill's job to fix, not the user's to discover.

---

## Finishing

Report what passed, what was skipped, and what the user must do on their own
machine. A gate that was added but never run is not done — say so plainly rather
than implying coverage that does not exist.
