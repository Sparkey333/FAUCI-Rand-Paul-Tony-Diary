// Renders both tabs from reference/providers.json.
//
// One registry, two views: add a provider to the JSON and it appears in the
// keys tab and the parity table together. Nothing here is bundled — the file
// runs as-is in a browser or inside a desktop webview.

const CAP_ORDER = [
  "streaming",
  "tools",
  "vision",
  "structured",
  "embeddings",
  "caching",
  "reasoning",
];

const CAP_LABEL = {
  streaming: "Stream",
  tools: "Tools",
  vision: "Vision",
  structured: "JSON",
  embeddings: "Embed",
  caching: "Cache",
  reasoning: "Reason",
};

// ── Storage ────────────────────────────────────────────────────────────────
// Swap this object for an OS keychain adapter in a desktop build. Keys never
// travel with the document and are never written into the repository.
const store = {
  prefix: "byok:",
  get(id) {
    try {
      return localStorage.getItem(this.prefix + id) ?? "";
    } catch {
      return "";
    }
  },
  set(id, value) {
    try {
      if (value) localStorage.setItem(this.prefix + id, value);
      else localStorage.removeItem(this.prefix + id);
    } catch {
      /* private browsing — the field still works for this session */
    }
  },
};

const el = (sel) => document.querySelector(sel);
const state = { registry: null, filter: "", onlyConfigured: false, parityFilter: "", onlyGaps: false };

function escapeHtml(v) {
  return String(v).replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c],
  );
}

// ── Tabs ───────────────────────────────────────────────────────────────────
function showTab(which) {
  const keys = which === "keys";
  el("#tab-keys").setAttribute("aria-selected", String(keys));
  el("#tab-parity").setAttribute("aria-selected", String(!keys));
  el("#panel-keys").hidden = !keys;
  el("#panel-parity").hidden = keys;
}

// ── Keys tab ───────────────────────────────────────────────────────────────
function matches(p, needle) {
  if (!needle) return true;
  const hay = `${p.name} ${p.id} ${p.envVar ?? ""} ${p.category}`.toLowerCase();
  return hay.includes(needle.toLowerCase());
}

function renderKeys() {
  const root = el("#providers");
  const { categories, providers } = state.registry;
  root.innerHTML = "";

  let shown = 0;

  for (const cat of categories) {
    const inCat = providers.filter((p) => {
      if (p.category !== cat.id) return false;
      if (!matches(p, state.filter)) return false;
      if (state.onlyConfigured && !store.get(p.id)) return false;
      return true;
    });
    if (!inCat.length) continue;
    shown += inCat.length;

    const group = document.createElement("section");
    group.className = "group";
    group.innerHTML = `<h2>${escapeHtml(cat.label)}</h2><p class="blurb">${escapeHtml(cat.blurb)}</p>`;

    for (const p of inCat) group.appendChild(card(p));
    root.appendChild(group);
  }

  if (!shown) {
    root.innerHTML = `<div class="empty">No provider matches that filter.</div>`;
  }

  const configured = providers.filter((p) => store.get(p.id)).length;
  const needKeys = providers.filter((p) => p.requiresKey !== false).length;
  el("#key-count").textContent = `${configured} of ${needKeys} keyed · ${providers.length} providers`;
}

function card(p) {
  const node = document.createElement("article");
  node.className = "card";

  const local = p.requiresKey === false;
  const hasKey = Boolean(store.get(p.id));
  const pill = local
    ? `<span class="pill local">local</span>`
    : `<span class="pill ${hasKey ? "set" : ""}">${hasKey ? "key set" : "no key"}</span>`;

  node.innerHTML = `
    <div class="card-head">
      <h3>${escapeHtml(p.name)}</h3>
      ${pill}
    </div>
    <p class="meta">${escapeHtml(p.envVar ?? "")}${p.envVar ? " · " : ""}${escapeHtml(p.baseUrl ?? "")}</p>
    ${p.notes ? `<p class="note">${escapeHtml(p.notes)}</p>` : ""}
    <div class="row">
      ${
        local
          ? `<input type="url" value="${escapeHtml(p.baseUrl ?? "")}" aria-label="${escapeHtml(p.name)} base URL" data-role="url" />
             <button class="btn" data-role="test">Test</button>`
          : `<input type="password" placeholder="Paste key…" autocomplete="off" spellcheck="false"
                    aria-label="${escapeHtml(p.name)} API key" data-role="key" />
             <button class="btn" data-role="reveal" aria-label="Show or hide key">Show</button>`
      }
      <a class="btn primary" href="${escapeHtml(p.keyUrl)}" target="_blank" rel="noopener noreferrer">
        ${local ? "Install ↗" : "Get key ↗"}
      </a>
      <a class="btn" href="${escapeHtml(p.docsUrl)}" target="_blank" rel="noopener noreferrer">Docs ↗</a>
      <span class="status" data-role="status"></span>
    </div>
  `;

  const status = node.querySelector('[data-role="status"]');

  if (local) {
    node.querySelector('[data-role="test"]').addEventListener("click", async (e) => {
      const url = node.querySelector('[data-role="url"]').value.trim();
      const btn = e.currentTarget;
      btn.disabled = true;
      status.className = "status";
      status.textContent = "checking…";
      const ok = await probe(url, p.healthPath);
      status.className = `status ${ok ? "good" : "bad"}`;
      status.textContent = ok ? "reachable" : "no response";
      btn.disabled = false;
    });
  } else {
    const input = node.querySelector('[data-role="key"]');
    input.value = store.get(p.id);

    input.addEventListener("input", () => {
      const v = input.value.trim();
      store.set(p.id, v);
      const badge = node.querySelector(".pill");
      badge.classList.toggle("set", Boolean(v));
      badge.textContent = v ? "key set" : "no key";

      // A wrong-vendor paste is the most common BYOK mistake, and the resulting
      // 401 says nothing useful. Catch it at the point of entry instead.
      if (v && p.keyPrefix && !v.startsWith(p.keyPrefix)) {
        status.className = "status bad";
        status.textContent = `expected a key starting "${p.keyPrefix}"`;
      } else {
        status.className = "status";
        status.textContent = "";
      }
      el("#key-count").textContent = countLabel();
    });

    node.querySelector('[data-role="reveal"]').addEventListener("click", (e) => {
      const showing = input.type === "text";
      input.type = showing ? "password" : "text";
      e.currentTarget.textContent = showing ? "Show" : "Hide";
    });
  }

  return node;
}

function countLabel() {
  const { providers } = state.registry;
  const configured = providers.filter((p) => store.get(p.id)).length;
  const needKeys = providers.filter((p) => p.requiresKey !== false).length;
  return `${configured} of ${needKeys} keyed · ${providers.length} providers`;
}

/** Best-effort liveness check for a local server. */
async function probe(baseUrl, healthPath) {
  const base = baseUrl.replace(/\/+$/, "");
  const candidates = healthPath ? [base + healthPath, base] : [base + "/models", base];
  for (const url of candidates) {
    try {
      const ctrl = new AbortController();
      const timer = setTimeout(() => ctrl.abort(), 3000);
      const res = await fetch(url, { signal: ctrl.signal });
      clearTimeout(timer);
      if (res.ok) return true;
    } catch {
      /* try the next candidate */
    }
  }
  return false;
}

// ── Parity tab ─────────────────────────────────────────────────────────────
function renderParity() {
  const { providers, categories } = state.registry;
  const thead = el("#parity thead");
  const tbody = el("#parity tbody");
  const catLabel = Object.fromEntries(categories.map((c) => [c.id, c.label]));

  thead.innerHTML = `<tr><th>Provider</th>${CAP_ORDER.map(
    (k) => `<th title="${escapeHtml(state.registry.capabilityKeys[k])}">${CAP_LABEL[k]}</th>`,
  ).join("")}</tr>`;

  const rows = providers.filter((p) => {
    if (!matches(p, state.parityFilter)) return false;
    if (state.onlyGaps) {
      const caps = p.capabilities ?? {};
      return CAP_ORDER.some((k) => caps[k] !== true);
    }
    return true;
  });

  tbody.innerHTML =
    rows
      .map((p) => {
        const caps = p.capabilities ?? {};
        const cells = CAP_ORDER.map((k) => {
          const v = caps[k];
          const [cls, glyph] =
            v === true ? ["yes", "✓"] : v === false ? ["no", "·"] : ["unk", "?"];
          return `<td class="cap"><span class="${cls}">${glyph}</span></td>`;
        }).join("");
        return `<tr>
          <td>
            <div class="provider-cell">
              <span>${escapeHtml(p.name)}</span>
              <span class="cat-tag">${escapeHtml(catLabel[p.category] ?? p.category)} ·
                <a href="${escapeHtml(p.docsUrl)}" target="_blank" rel="noopener noreferrer">docs ↗</a>
              </span>
            </div>
          </td>${cells}
        </tr>`;
      })
      .join("") || `<tr><td colspan="${CAP_ORDER.length + 1}"><div class="empty">Nothing matches.</div></td></tr>`;

  // Coverage per capability, so a thin column is obvious at a glance.
  const stats = CAP_ORDER.map((k) => {
    const n = providers.filter((p) => p.capabilities?.[k] === true).length;
    return `<span><b>${n}</b>/${providers.length} ${CAP_LABEL[k]}</span>`;
  }).join("");
  const unknown = providers.filter((p) =>
    CAP_ORDER.some((k) => p.capabilities?.[k] == null),
  ).length;
  el("#parity-stats").innerHTML =
    stats + (unknown ? `<span><b>${unknown}</b> with unverified rows</span>` : "");
}

// ── Boot ───────────────────────────────────────────────────────────────────
async function init() {
  try {
    const res = await fetch("providers.json");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    state.registry = await res.json();
  } catch (err) {
    el("#providers").innerHTML =
      `<div class="empty">Could not load <code>providers.json</code> (${escapeHtml(err.message)}).<br />
       Serve this directory over HTTP rather than opening the file directly —
       <code>python3 -m http.server</code> — since <code>fetch</code> is blocked on <code>file://</code>.</div>`;
    return;
  }

  el("#last-reviewed").textContent = state.registry.lastReviewed ?? "unknown";

  el("#tab-keys").addEventListener("click", () => showTab("keys"));
  el("#tab-parity").addEventListener("click", () => showTab("parity"));
  document.addEventListener("click", (e) => {
    if (e.target.dataset?.goto === "parity") showTab("parity");
  });

  el("#filter").addEventListener("input", (e) => {
    state.filter = e.target.value;
    renderKeys();
  });
  el("#only-configured").addEventListener("change", (e) => {
    state.onlyConfigured = e.target.checked;
    renderKeys();
  });
  el("#parity-filter").addEventListener("input", (e) => {
    state.parityFilter = e.target.value;
    renderParity();
  });
  el("#only-gaps").addEventListener("change", (e) => {
    state.onlyGaps = e.target.checked;
    renderParity();
  });

  renderKeys();
  renderParity();
}

init();
