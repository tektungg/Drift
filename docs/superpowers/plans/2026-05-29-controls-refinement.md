# Controls Refinement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Drift's native `<select>` sort dropdown + plain `↑/↓` button with a custom on-brand sort pill + dropdown menu, add a leading magnifier icon to the search field, and refine the inline multi-select bar with action icons.

**Architecture:** Frontend-only. New icons go in `src/icons.js`. A new pure helper `sortDirectionLabel(key, dir)` lives in `src/list-ops.js` (unit-tested under Node). The sort pill + custom menu replace the native-select wiring in `src/main.js`'s `wireListControls()`, reusing the existing context-menu open/dismiss discipline. CSS for the new controls replaces the old `.list-sort`/`.list-sortdir` rules.

**Tech Stack:** Vanilla HTML/CSS/JS (ES modules), Tauri 2 WebView2. Frontend pure-logic tests via `node --test`. No Rust changes.

**Spec:** `docs/superpowers/specs/2026-05-29-controls-refinement-design.md`

---

## File Structure

**Modified:**
- `src/list-ops.js` — add pure `sortDirectionLabel(key, dir)` (the per-key direction wording map).
- `src/list-ops.test.js` — tests for `sortDirectionLabel`.
- `src/icons.js` — add `search`, `sort`, `chevron`, `trash` icons (reuse existing `downloading`/`paused` glyphs for the bulk Resume/Pause buttons).
- `src/index.html` — replace the `<select>` + dir `<button>` in `.list-controls` with the search-icon wrapper + the `#sort-trigger` pill.
- `src/styles.css` — search-with-icon, `.sort-trigger`, `.sort-menu` (+ active row, check, direction glyph), refined `.bulk-bar` icon buttons; remove `.list-sort` / `.list-sortdir`.
- `src/main.js` — import `sortDirectionLabel` + `icon`; rewrite `wireListControls()` to render the pill label and open/close a custom sort menu with key-select vs active-key-flip; add icons in `updateBulkBar()`.

No Rust, no `state.json`/`config.json` changes.

---

### Task 1: Pure `sortDirectionLabel` helper + tests

**Files:**
- Modify: `src/list-ops.js`
- Modify: `src/list-ops.test.js`

- [ ] **Step 1: Write the failing tests**

Append to `src/list-ops.test.js` (the file already imports from `./list-ops.js`; add `sortDirectionLabel` to that import and add these tests at the end):

```js
import { sortDirectionLabel } from "./list-ops.js";

test("sortDirectionLabel gives meaningful per-key wording", () => {
  assert.equal(sortDirectionLabel("added", "desc"), "↓ newest");
  assert.equal(sortDirectionLabel("added", "asc"),  "↑ oldest");
  assert.equal(sortDirectionLabel("name", "desc"),  "↓ Z–A");
  assert.equal(sortDirectionLabel("name", "asc"),   "↑ A–Z");
  assert.equal(sortDirectionLabel("progress", "desc"), "↓ high");
  assert.equal(sortDirectionLabel("progress", "asc"),  "↑ low");
  assert.equal(sortDirectionLabel("speed", "desc"), "↓ fast");
  assert.equal(sortDirectionLabel("speed", "asc"),  "↑ slow");
  assert.equal(sortDirectionLabel("size", "desc"),  "↓ large");
  assert.equal(sortDirectionLabel("size", "asc"),   "↑ small");
});

test("sortDirectionLabel falls back to a bare arrow for unknown keys", () => {
  assert.equal(sortDirectionLabel("whatever", "desc"), "↓");
  assert.equal(sortDirectionLabel("whatever", "asc"),  "↑");
});
```

> Note: the existing top import line is
> `import { matchesSearch, filterTorrents, compareBy, sortTorrents } from "./list-ops.js";`
> You may either extend it to include `sortDirectionLabel` or add a second import
> line as shown above. Either is fine — just don't duplicate the symbol.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `node --test src/list-ops.test.js`
Expected: FAIL — `sortDirectionLabel is not a function` / not exported.

- [ ] **Step 3: Implement the helper**

Append to `src/list-ops.js`:

```js
// Human-friendly direction wording per sort key, shown on the active row of the
// sort menu. Arrow + a word so the direction is meaningful at a glance.
const DIR_WORDS = {
  added:    { desc: "newest", asc: "oldest" },
  name:     { desc: "Z–A",    asc: "A–Z" },
  progress: { desc: "high",   asc: "low" },
  speed:    { desc: "fast",   asc: "slow" },
  size:     { desc: "large",  asc: "small" },
};

export function sortDirectionLabel(key, dir) {
  const arrow = dir === "asc" ? "↑" : "↓";
  const word = DIR_WORDS[key]?.[dir];
  return word ? `${arrow} ${word}` : arrow;
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `node --test src/list-ops.test.js`
Expected: PASS — all tests (existing + 2 new) green.

- [ ] **Step 5: Commit**

```bash
git add src/list-ops.js src/list-ops.test.js
git commit -m "Add sortDirectionLabel helper with per-key direction wording"
```

---

### Task 2: Add new icons

**Files:**
- Modify: `src/icons.js`

- [ ] **Step 1: Add the icons to the ICONS map**

In `src/icons.js`, inside the `export const ICONS = { ... }` object, add these entries (place them after the existing `wave` entry, before the theme-toggle group — anywhere inside the object is fine):

```js
  // controls
  search:  SVG('<circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/>'),
  sort:    SVG('<path d="M3 6h12"/><path d="M3 12h8"/><path d="M3 18h4"/>'),
  chevron: SVG('<path d="m6 9 6 6 6-6"/>'),
  trash:   SVG('<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/>'),
```

(`resume`/`pause` for the bulk bar reuse the existing `downloading` and `paused`
glyphs — no new icons needed for those.)

- [ ] **Step 2: Verify the module parses**

Run: `node --check src/icons.js`
Expected: exit 0, no output.

- [ ] **Step 3: Commit**

```bash
git add src/icons.js
git commit -m "Add search, sort, chevron, trash icons"
```

---

### Task 3: Replace the header sort markup

**Files:**
- Modify: `src/index.html`

- [ ] **Step 1: Replace the `.list-controls` block**

In `src/index.html`, find the current controls block:

```html
        <div class="list-controls">
          <input type="text" id="list-search" class="list-search" placeholder="Search…" autocomplete="off" spellcheck="false">
          <select id="list-sort" class="list-sort" title="Sort by">
            <option value="added">Date added</option>
            <option value="name">Name</option>
            <option value="progress">Progress</option>
            <option value="speed">Speed</option>
            <option value="size">Size</option>
          </select>
          <button id="list-sortdir" class="list-sortdir" title="Toggle direction">↓</button>
        </div>
```

Replace it entirely with:

```html
        <div class="list-controls">
          <span class="search-wrap" id="search-wrap">
            <input type="text" id="list-search" class="list-search" placeholder="Search downloads…" autocomplete="off" spellcheck="false">
          </span>
          <button id="sort-trigger" class="sort-trigger" type="button" title="Sort"></button>
        </div>
```

(The search icon is injected by JS into `#search-wrap`; the sort pill's contents
and the popup menu are rendered by JS. Leaving them empty here keeps the markup
declarative and the icon SVGs in one place — `icons.js`.)

- [ ] **Step 2: Verify the HTML is well-formed**

Open the file and confirm the block replaced cleanly (no leftover `<select>` or
`#list-sortdir`). There is no automated HTML check; visually confirm the tags balance.

- [ ] **Step 3: Commit**

```bash
git add src/index.html
git commit -m "Replace native sort select/dir-button with a sort-pill placeholder"
```

---

### Task 4: Styles — search icon, sort pill + menu, refined bulk bar

**Files:**
- Modify: `src/styles.css`

- [ ] **Step 1: Replace the old `.list-sort`/`.list-sortdir` rules**

In `src/styles.css`, find this block:

```css
.list-search { padding: 0 10px; width: 200px; transition: border-color .12s ease; }
.list-search:focus { outline: none; border-color: var(--accent); }
.list-sort { padding: 0 8px; cursor: pointer; }
.list-sortdir { padding: 0 11px; cursor: pointer; line-height: 1; }
.list-sortdir:hover, .list-sort:hover { border-color: var(--accent); }
```

Replace it with:

```css
/* Search field with a leading magnifier icon. */
.search-wrap { position: relative; display: inline-flex; align-items: center; }
.search-wrap .ic-search { position: absolute; left: 10px; width: 15px; height: 15px;
  color: var(--ink-soft); pointer-events: none; display: inline-flex; }
.search-wrap .ic-search svg { width: 15px; height: 15px; }
.list-search { padding: 0 10px 0 32px; width: 220px; border-radius: 9px;
  transition: border-color .12s ease; }
.list-search:focus { outline: none; border-color: var(--accent); }

/* Custom sort pill (replaces the native select + direction button). */
.sort-trigger { height: 34px; box-sizing: border-box; display: inline-flex; align-items: center;
  gap: 7px; background: var(--surface); border: 1px solid var(--line); color: var(--ink);
  border-radius: 9px; font-size: 13px; padding: 0 10px; cursor: pointer;
  font-family: var(--font-sans); transition: border-color .12s ease; }
.sort-trigger:hover { border-color: var(--accent); }
.sort-trigger svg { width: 15px; height: 15px; color: var(--ink-soft); }
.sort-trigger .chev { width: 13px; height: 13px; margin-left: 2px; }
.sort-trigger .sort-label-soft { color: var(--ink-soft); }

/* Sort dropdown menu — shares the context-menu visual language. */
.sort-menu { position: fixed; background: var(--bg); border: 1px solid var(--line);
  border-radius: var(--radius-sm); padding: 5px; min-width: 200px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.12); z-index: 200; }
.sort-menu .grouplabel { font-size: 10px; text-transform: uppercase; letter-spacing: 0.06em;
  color: var(--ink-soft); padding: 4px 10px 4px; }
.sort-menu .mi { display: flex; align-items: center; gap: 8px; padding: 7px 10px;
  border-radius: 6px; font-size: 13px; cursor: pointer; color: var(--ink); }
.sort-menu .mi:hover { background: var(--hover); }
.sort-menu .mi.active { background: var(--accent-soft); color: var(--accent); }
.sort-menu .mi .check { width: 14px; height: 14px; flex-shrink: 0; display: inline-flex; }
.sort-menu .mi .check svg { width: 14px; height: 14px; }
.sort-menu .mi .blank { width: 14px; height: 14px; flex-shrink: 0; }
.sort-menu .mi .dirglyph { margin-left: auto; font-size: 11px; color: var(--accent); white-space: nowrap; }
```

- [ ] **Step 2: Refine the bulk-bar buttons**

In `src/styles.css`, find:

```css
.bulk-bar button { font-size: 12px; padding: 5px 11px; }
```

Replace it with:

```css
.bulk-bar button { font-size: 12px; padding: 5px 11px; display: inline-flex;
  align-items: center; gap: 6px; }
.bulk-bar button svg { width: 14px; height: 14px; }
.bulk-bar #bulk-clear { margin-left: 4px; }
.bulk-bar .divider { width: 1px; align-self: stretch; background: var(--line); margin: 0 2px; }
```

- [ ] **Step 3: Verify the app still loads (manual)**

Run: `cd src-tauri; cargo run` (PATH must include `~/.cargo/bin`). The window
should open without console errors. The sort pill will be empty until Task 5
wires it — that's expected. Close the app.

> NOTE: `cargo run` here rebuilds with the vendored librqbit (first build is
> slow). If you only want to check CSS validity, you can skip running and rely
> on Task 5's smoke test instead.

- [ ] **Step 4: Commit**

```bash
git add src/styles.css
git commit -m "Style search icon, custom sort pill + menu, refined bulk-bar buttons"
```

---

### Task 5: Wire the custom sort pill + menu, and bulk-bar icons

**Files:**
- Modify: `src/main.js`

- [ ] **Step 1: Extend imports**

In `src/main.js`, the current imports include:

```js
import { icon, extToCategory } from "./icons.js";
import { filterTorrents, sortTorrents } from "./list-ops.js";
```

Change the second line to also import `sortDirectionLabel`:

```js
import { filterTorrents, sortTorrents, sortDirectionLabel } from "./list-ops.js";
```

- [ ] **Step 2: Inject the search icon (in `wireListControls`)**

We will rewrite `wireListControls()` entirely. Replace the WHOLE current function
(shown below for reference):

```js
function wireListControls() {
  const search = document.getElementById("list-search");
  const sortSel = document.getElementById("list-sort");
  const dirBtn = document.getElementById("list-sortdir");
  if (!search || !sortSel || !dirBtn) return;
  sortSel.value = sortKey;
  dirBtn.textContent = sortDir === "asc" ? "↑" : "↓";
  search.addEventListener("input", () => { searchQuery = search.value; renderList(); });
  sortSel.addEventListener("change", () => {
    sortKey = sortSel.value; localStorage.setItem("drift-sort-key", sortKey); renderList();
  });
  dirBtn.addEventListener("click", () => {
    sortDir = sortDir === "asc" ? "desc" : "asc";
    localStorage.setItem("drift-sort-dir", sortDir);
    dirBtn.textContent = sortDir === "asc" ? "↑" : "↓";
    renderList();
  });
}
```

with this implementation:

```js
// Sort keys shown in the menu, in display order.
const SORT_KEYS = [
  { key: "added",    label: "Date added" },
  { key: "name",     label: "Name" },
  { key: "progress", label: "Progress" },
  { key: "speed",    label: "Speed" },
  { key: "size",     label: "Size" },
];
function sortKeyLabel(key) {
  return (SORT_KEYS.find(k => k.key === key) || SORT_KEYS[0]).label;
}

function renderSortTrigger() {
  const t = document.getElementById("sort-trigger");
  if (!t) return;
  t.innerHTML = `${icon("sort")}<span class="sort-label-soft">Sort:</span> ${escape(sortKeyLabel(sortKey))}<span class="chev">${icon("chevron")}</span>`;
}

function closeSortMenu() {
  document.querySelectorAll(".sort-menu").forEach(n => n.remove());
}

function openSortMenu() {
  closeSortMenu();
  const trigger = document.getElementById("sort-trigger");
  if (!trigger) return;
  const menu = document.createElement("div");
  menu.className = "sort-menu";
  menu.innerHTML = `<div class="grouplabel">Sort by</div>` + SORT_KEYS.map(k => {
    const active = k.key === sortKey;
    const lead = active ? `<span class="check">${icon("completed")}</span>` : `<span class="blank"></span>`;
    const dir = active ? `<span class="dirglyph">${sortDirectionLabel(k.key, sortDir)}</span>` : "";
    return `<div class="mi ${active ? "active" : ""}" data-key="${k.key}">${lead}${escape(k.label)}${dir}</div>`;
  }).join("");
  document.body.appendChild(menu);

  // Position under the trigger, right-aligned to it.
  const r = trigger.getBoundingClientRect();
  menu.style.top = `${r.bottom + 6}px`;
  const left = Math.min(r.left, window.innerWidth - menu.offsetWidth - 8);
  menu.style.left = `${Math.max(8, left)}px`;

  menu.querySelectorAll(".mi").forEach(row => row.onclick = (e) => {
    e.stopPropagation();
    const key = row.dataset.key;
    if (key === sortKey) {
      // Re-selecting the active key flips direction.
      sortDir = sortDir === "asc" ? "desc" : "asc";
    } else {
      sortKey = key;
    }
    localStorage.setItem("drift-sort-key", sortKey);
    localStorage.setItem("drift-sort-dir", sortDir);
    renderSortTrigger();
    renderList();
    closeSortMenu();
  });

  // Dismiss on outside click / Escape. Defer attaching the click closer so the
  // opening click (already bubbling) doesn't immediately close the menu.
  setTimeout(() => {
    document.addEventListener("click", closeSortMenu, { once: true });
  }, 0);
}

function wireListControls() {
  const search = document.getElementById("list-search");
  const wrap = document.getElementById("search-wrap");
  const trigger = document.getElementById("sort-trigger");
  if (!search || !wrap || !trigger) return;

  // Leading magnifier icon inside the search field.
  if (!wrap.querySelector(".ic-search")) {
    const ic = document.createElement("span");
    ic.className = "ic-search";
    ic.innerHTML = icon("search");
    wrap.insertBefore(ic, search);
  }

  search.addEventListener("input", () => { searchQuery = search.value; renderList(); });

  renderSortTrigger();
  trigger.addEventListener("click", (e) => {
    e.stopPropagation();
    // Toggle: if a menu is open, close it; otherwise open.
    if (document.querySelector(".sort-menu")) { closeSortMenu(); return; }
    openSortMenu();
  });
  window.addEventListener("keydown", (e) => { if (e.key === "Escape") closeSortMenu(); });
}
```

> Implementation notes for the engineer:
> - `escape()` is the existing HTML-escape helper in `main.js` (used elsewhere).
> - `icon("completed")` is the existing check-mark glyph (a tick) — reused as the
>   active-row check. `icon("sort")` / `icon("chevron")` were added in Task 2.
> - The dismiss pattern mirrors the existing `openContextMenu` (one-shot
>   `document` click listener). Opening uses `stopPropagation` + a deferred
>   listener so the opening click can't self-close the menu.

- [ ] **Step 3: Add icons to the bulk bar**

In `src/main.js`, in `updateBulkBar()`, replace the `bar.innerHTML = ...` template
(currently text-only buttons) with icon+label buttons. The current block is:

```js
  bar.innerHTML = `
    <span class="count">${selected.size} selected</span>
    <div class="spacer"></div>
    <button class="btn-ghost" id="bulk-resume">Resume</button>
    <button class="btn-ghost" id="bulk-pause">Pause</button>
    <button class="btn-ghost" id="bulk-remove">Remove</button>
    <button class="btn-ghost" id="bulk-clear">Clear</button>`;
```

Replace it with:

```js
  bar.innerHTML = `
    <span class="count">${selected.size} selected</span>
    <div class="spacer"></div>
    <button class="btn-ghost" id="bulk-resume">${icon("downloading")}Resume</button>
    <button class="btn-ghost" id="bulk-pause">${icon("paused")}Pause</button>
    <button class="btn-ghost" id="bulk-remove">${icon("trash")}Remove</button>
    <span class="divider"></span>
    <button class="btn-ghost" id="bulk-clear">Clear</button>`;
```

(The four `document.getElementById("bulk-…").onclick = …` lines right below stay
exactly as they are.)

- [ ] **Step 4: Verify the JS parses**

Run: `node --check src/main.js`
Expected: exit 0, no output.

- [ ] **Step 5: Manual smoke test**

Run: `cd src-tauri; cargo run`. In the window:
- Search field shows the magnifier icon; typing filters; focus shows the accent border with no width jump.
- The sort pill reads `Sort: Date added ▾`. Click it → menu opens under it.
- The active row has a check + e.g. `↓ newest`. Click **Name** → list re-sorts, pill updates, menu closes.
- Reopen, click the **active** row → direction flips (`↑ A–Z` ⇄ `↓ Z–A`), list re-sorts.
- Click outside / press Escape → menu closes. Open/close several times → it never gets stuck open or insta-closes.
- Restart the app → the sort key + direction persist.
- Ctrl-click a couple of rows → the bulk bar shows Resume/Pause/Remove with icons + a divider before Clear.
Close the app.

- [ ] **Step 6: Commit**

```bash
git add src/main.js
git commit -m "Wire custom sort pill + dropdown menu and bulk-bar action icons"
```

---

## Self-Review (completed by plan author)

**Spec coverage:**
- Feature 1 (search field: leading icon, "Search downloads…", border-only focus) → Task 2 (icon), Task 3 (placeholder), Task 4 (CSS), Task 5 Step 2 (inject icon) ✓
- Feature 2 (custom sort pill + menu; active key check + direction word; click-active-flips; persist; remove native select/dir button) → Task 1 (wording helper), Task 2 (icons), Task 3 (markup removal), Task 4 (CSS), Task 5 (wiring) ✓
- Feature 3 (refined inline bulk bar with icons) → Task 4 Step 2 (CSS), Task 5 Step 3 (icons in updateBulkBar) ✓
- Testing: `sortDirectionLabel` unit-tested (Task 1); manual smoke (Task 5) ✓
- Risks (menu-dismiss leakage, click-through) → addressed in Task 5 Step 2 (one-shot listener + stopPropagation + deferred attach) ✓

**Placeholder scan:** No TBD/TODO; every code step has complete code. The
`icon("completed")` reuse for the check mark and `icon("downloading")`/
`icon("paused")` reuse for bulk buttons are existing glyphs (verified present in
`icons.js`).

**Type/name consistency:** `wireListControls`, `renderSortTrigger`,
`openSortMenu`, `closeSortMenu`, `sortKeyLabel`, `SORT_KEYS`, `sortDirectionLabel`,
element ids `#search-wrap` / `#sort-trigger` / `#list-search`, and CSS classes
`.search-wrap`/`.ic-search`/`.sort-trigger`/`.sort-menu`/`.mi`/`.dirglyph` are
used consistently across Tasks 3–5. The native `#list-sort`/`#list-sortdir` are
removed in Task 3 and no longer referenced after Task 5's rewrite.
