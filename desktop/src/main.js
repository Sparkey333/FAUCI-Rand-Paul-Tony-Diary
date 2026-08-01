// Front-end for the Provenance window.
//
// Uses the global Tauri API (`withGlobalTauri`) so the app ships without a
// bundler — the whole frontend is three static files.

// Fail loudly rather than silently: without `withGlobalTauri` in tauri.conf.json
// — or when index.html is opened directly in a browser — these globals are
// absent and every button would quietly do nothing.
const bridge = window.__TAURI__;
const invoke = bridge?.core?.invoke;
const open = bridge?.dialog?.open;

const els = {
  choose: document.getElementById("choose"),
  chosen: document.getElementById("chosen"),
  note: document.getElementById("note"),
  seal: document.getElementById("seal"),
  verify: document.getElementById("verify"),
  result: document.getElementById("result"),
};

let folder = null;

function setFolder(path) {
  folder = path;
  els.chosen.textContent = path ?? "No folder selected";
  els.chosen.dataset.empty = path ? "false" : "true";
  els.seal.disabled = !path;
  els.verify.disabled = !path;
}

function escapeHtml(value) {
  return String(value).replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c],
  );
}

function render(kind, heading, body) {
  els.result.hidden = false;
  els.result.innerHTML = `<div class="verdict ${kind}"><h2>${escapeHtml(heading)}</h2>${body}</div>`;
}

function definitions(pairs) {
  const rows = pairs
    .map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${escapeHtml(v)}</dd>`)
    .join("");
  return `<dl>${rows}</dl>`;
}

/// Run an async action with the buttons disabled, so a slow hash over a large
/// folder cannot be started twice.
async function withBusy(label, fn) {
  const wasDisabled = [els.seal.disabled, els.verify.disabled];
  els.seal.disabled = els.verify.disabled = true;
  render("info", label, "<p>Hashing…</p>");
  try {
    await fn();
  } catch (err) {
    render("error", "Could not complete", `<p>${escapeHtml(err)}</p>`);
  } finally {
    [els.seal.disabled, els.verify.disabled] = wasDisabled;
  }
}

els.choose.addEventListener("click", async () => {
  const picked = await open({ directory: true, multiple: false });
  if (typeof picked === "string") {
    setFolder(picked);
    els.result.hidden = true;
  }
});

els.seal.addEventListener("click", () =>
  withBusy("Sealing", async () => {
    const outcome = await invoke("seal_directory", {
      path: folder,
      note: els.note.value || null,
    });
    render(
      "ok",
      `Sealed ${outcome.file_count} file${outcome.file_count === 1 ? "" : "s"}`,
      definitions([
        ["root", outcome.root],
        ["sealed at", outcome.sealed_at],
        ["manifest", outcome.manifest_path],
      ]) +
        `<p style="margin-top:14px;color:var(--muted);font-size:13px">Keep this root somewhere outside the folder. A copy that can be edited alongside the files it describes proves nothing.</p>`,
    );
  }),
);

els.verify.addEventListener("click", () =>
  withBusy("Verifying", async () => {
    const report = await invoke("verify_directory", { path: folder });
    const intact =
      report.changes.length === 0 && report.expected_root === report.actual_root;

    const roots = definitions([
      ["expected root", report.expected_root],
      ["actual root", report.actual_root],
    ]);

    if (intact) {
      render("ok", "Intact", roots + "<p>Every file matches the sealed manifest.</p>");
      return;
    }

    const items = report.changes
      .map((c) => {
        const kind = c.kind.toLowerCase();
        return `<li><span class="tag ${escapeHtml(kind)}">${escapeHtml(kind)}</span>${escapeHtml(c.path)}</li>`;
      })
      .join("");

    render(
      "bad",
      `Changed since sealing — ${report.changes.length} difference${report.changes.length === 1 ? "" : "s"}`,
      roots + `<ul class="changes">${items}</ul>`,
    );
  }),
);

if (!invoke || !open) {
  setFolder(null);
  els.choose.disabled = true;
  render(
    "error",
    "Tauri bridge unavailable",
    "<p>This page has to run inside the Provenance app window. Opened directly in a browser, it has no way to reach the hashing code.</p>",
  );
} else {
  setFolder(null);
  // Visible confirmation that the bridge resolved, replaced by the first result.
  render("info", "Ready", "<p>Choose a folder to seal or verify.</p>");
}
