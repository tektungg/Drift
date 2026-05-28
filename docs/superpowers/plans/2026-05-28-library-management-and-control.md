# Library Management & Control — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add search/sort, multi-select bulk actions, download queue management, and (gated) per-torrent speed limits to Drift, so the client stays comfortable as the torrent list grows.

**Architecture:** Frontend-only work (search/sort, multi-select) lives in `src/`, with pure logic extracted to `src/list-ops.js` so it is unit-testable under Node. Queue management is owned by Drift in a new pure Rust module `src-tauri/src/queue.rs` (a `decide()` function over the torrent set + cap) plus a thin async controller that applies the plan via the existing engine `pause`/`unpause`. State and settings gain backward-compatible fields. Per-torrent limits are gated behind a librqbit feasibility spike.

**Tech Stack:** Tauri 2, Rust (librqbit), vanilla HTML/CSS/JS. Rust tests via `cargo test`; frontend pure-logic tests via `node --test`.

**Spec:** `docs/superpowers/specs/2026-05-28-library-management-and-control-design.md`

---

## File Structure

**Created:**
- `src/list-ops.js` — pure functions: `matchesSearch`, `filterTorrents`, `compareBy`, `sortTorrents`. No Tauri imports, so importable by both the browser app and Node tests.
- `src/list-ops.test.js` — Node test-runner tests for the above.
- `src-tauri/src/queue.rs` — pure `decide(items, max_active) -> QueuePlan` + types + unit tests; plus the async `apply_plan` controller helper.

**Modified:**
- `src/main.js` — header search/sort UI, selection model, bulk action bar, queued state + sidebar filter, context-menu entries (force start, reorder), import of `list-ops.js`.
- `src/index.html` — search input + sort control markup in the main header.
- `src/styles.css` — styles for search box, sort control, selected rows, bulk bar, queued state color.
- `src-tauri/src/state.rs` — `TorrentState::Queued`; new record fields (`queue_position`, `forced`, `dl_limit`, `ul_limit`); helper to compute next queue position.
- `src-tauri/src/settings.rs` — `max_active_downloads` field (default 3).
- `src-tauri/src/commands.rs` — queue-aware `add_torrent`/`pause`/`resume`; new commands `force_start`, `move_in_queue`; run the controller after mutations; (gated) `set_torrent_limits`.
- `src-tauri/src/main.rs` — route resume-on-launch through the controller; preserve Queued vs Paused in the progress→state persistence.
- `src-tauri/src/lib.rs` — register the `queue` module.

---

## PHASE 1 — Search & Sort

### Task 1: Pure search/sort logic + tests

**Files:**
- Create: `src/list-ops.js`
- Test: `src/list-ops.test.js`

- [ ] **Step 1: Write the failing tests**

Create `src/list-ops.test.js`:

```js
import { test } from "node:test";
import assert from "node:assert/strict";
import { matchesSearch, filterTorrents, compareBy, sortTorrents } from "./list-ops.js";

const A = { infohash: "a", name: "Ubuntu 24.04", state_label: "downloading", downloaded: 50, total: 100, down_bps: 10, total_size: 100, added_at: 3 };
const B = { infohash: "b", name: "Debian 12",    state_label: "seeding",     downloaded: 100, total: 100, down_bps: 0,  total_size: 200, added_at: 1 };
const C = { infohash: "c", name: "ubuntu-server", state_label: "downloading", downloaded: 10,  total: 100, down_bps: 99, total_size: 50,  added_at: 2 };

test("matchesSearch is case-insensitive substring", () => {
  assert.equal(matchesSearch(A, "ubuntu"), true);
  assert.equal(matchesSearch(C, "UBUNTU"), true);
  assert.equal(matchesSearch(B, "ubuntu"), false);
  assert.equal(matchesSearch(A, ""), true); // empty query matches all
});

test("filterTorrents composes state filter AND search", () => {
  const out = filterTorrents([A, B, C], "downloading", "ubuntu");
  assert.deepEqual(out.map(t => t.infohash), ["a", "c"]);
  assert.deepEqual(filterTorrents([A, B, C], "all", "").map(t => t.infohash), ["a", "b", "c"]);
  assert.deepEqual(filterTorrents([A, B, C], "seeding", "").map(t => t.infohash), ["b"]);
});

test("compareBy progress ascending", () => {
  // progress = downloaded/total: A=0.5, C=0.1 -> C before A ascending
  const cmp = compareBy("progress", "asc");
  assert.equal(cmp(A, C) > 0, true);
});

test("sortTorrents by added desc is default-friendly", () => {
  assert.deepEqual(sortTorrents([B, C, A], "added", "desc").map(t => t.infohash), ["a", "c", "b"]);
});

test("sortTorrents by name asc", () => {
  assert.deepEqual(sortTorrents([A, B, C], "name", "asc").map(t => t.infohash), ["b", "a", "c"]);
});

test("sortTorrents by speed desc", () => {
  assert.deepEqual(sortTorrents([A, B, C], "speed", "desc").map(t => t.infohash), ["c", "a", "b"]);
});

test("sortTorrents by size desc uses total_size", () => {
  assert.deepEqual(sortTorrents([A, B, C], "size", "desc").map(t => t.infohash), ["b", "a", "c"]);
});

test("sortTorrents does not mutate input", () => {
  const arr = [A, B, C];
  sortTorrents(arr, "name", "asc");
  assert.deepEqual(arr.map(t => t.infohash), ["a", "b", "c"]);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `node --test src/list-ops.test.js`
Expected: FAIL — `Cannot find module './list-ops.js'` / functions undefined.

- [ ] **Step 3: Write the implementation**

Create `src/list-ops.js`:

```js
// Pure, dependency-free list operations for the torrent list.
// Imported by both the browser app (main.js) and Node tests (list-ops.test.js).

export function matchesSearch(t, query) {
  const q = (query || "").trim().toLowerCase();
  if (!q) return true;
  return (t.name || "").toLowerCase().includes(q);
}

export function filterTorrents(torrents, stateFilter, query) {
  return torrents.filter(t =>
    (stateFilter === "all" || t.state_label === stateFilter) && matchesSearch(t, query)
  );
}

// Numeric/string key extractors for each sort key.
function sortValue(t, key) {
  switch (key) {
    case "name":     return (t.name || "").toLowerCase();
    case "progress": return t.total > 0 ? t.downloaded / t.total : 0;
    case "speed":    return t.down_bps || 0;
    case "size":     return t.total_size || 0;
    case "added":
    default:         return t.added_at || 0;
  }
}

export function compareBy(key, dir) {
  const sign = dir === "asc" ? 1 : -1;
  return (a, b) => {
    const va = sortValue(a, key), vb = sortValue(b, key);
    if (va < vb) return -1 * sign;
    if (va > vb) return 1 * sign;
    return 0;
  };
}

export function sortTorrents(torrents, key, dir) {
  return [...torrents].sort(compareBy(key, dir));
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `node --test src/list-ops.test.js`
Expected: PASS — all tests green.

- [ ] **Step 5: Commit**

```bash
git add src/list-ops.js src/list-ops.test.js
git commit -m "Add pure search/sort list operations with Node tests"
```

---

### Task 2: Wire search box + sort control into the header

**Files:**
- Modify: `src/index.html` (main header), `src/main.js` (import, state, renderList), `src/styles.css`

- [ ] **Step 1: Add the search + sort markup to the header**

In `src/index.html`, find the main header element that contains the `#main-title` ("All downloads") and the `#btn-add` ("+ Add torrent") button. Add a controls block between the title and the Add button so the row reads: title · (search + sort) · Add. Insert:

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

(If the header is currently a flex row with `justify-content: space-between`, wrap the title and `.list-controls` so the Add button stays on the right. The exact wrapper depends on the current markup — keep the Add button last in the row.)

- [ ] **Step 2: Add styles**

In `src/styles.css`, add:

```css
.list-controls { display: flex; align-items: center; gap: 8px; }
.list-search {
  background: var(--surface); border: 1px solid var(--line); color: var(--ink);
  border-radius: 8px; padding: 6px 10px; font-size: 13px; width: 180px;
  font-family: var(--font-sans); transition: border-color .12s ease, width .12s ease;
}
.list-search:focus { outline: none; border-color: var(--accent); width: 220px; }
.list-sort {
  background: var(--surface); border: 1px solid var(--line); color: var(--ink);
  border-radius: 8px; padding: 6px 8px; font-size: 13px; cursor: pointer; font-family: var(--font-sans);
}
.list-sortdir {
  background: var(--surface); border: 1px solid var(--line); color: var(--ink);
  border-radius: 8px; padding: 6px 9px; font-size: 13px; cursor: pointer; line-height: 1;
}
.list-sortdir:hover, .list-sort:hover { border-color: var(--accent); }
```

- [ ] **Step 3: Import list-ops and add UI state in main.js**

At the top of `src/main.js`, extend the icons import line with a new import below it:

```js
import { matchesSearch, filterTorrents, sortTorrents } from "./list-ops.js";
```

Near the other module-level state (`let currentFilter = "all";`), add:

```js
let searchQuery = "";
let sortKey = localStorage.getItem("drift-sort-key") || "added";
let sortDir = localStorage.getItem("drift-sort-dir") || "desc";
```

- [ ] **Step 4: Apply filter + sort in renderList**

In `src/main.js`, in `renderList()`, replace the line that computes `filtered`:

```js
  const filtered = currentFilter === "all" ? torrents
    : torrents.filter(t => t.state_label === currentFilter);
```

with:

```js
  const filtered = sortTorrents(
    filterTorrents(torrents, currentFilter, searchQuery),
    sortKey, sortDir
  );
```

- [ ] **Step 5: Wire the controls (once, on boot)**

In `src/main.js`, inside the boot IIFE after `renderAll();` (the first call), add wiring that updates state and re-renders. Add this helper function near `renderAll` and call it from boot:

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

Then call `wireListControls();` once, immediately after the first `renderAll();` in the boot IIFE.

- [ ] **Step 6: Verify the JS parses**

Run: `node --check src/main.js`
Expected: no output (exit 0).

- [ ] **Step 7: Manual smoke (build dev once)**

Run: `cd src-tauri; cargo run` (PATH must include `~/.cargo/bin`). In the window: type in the search box → list filters live; change sort → order changes; click the direction button → order reverses; restart → sort key/dir persist. Close the app.

- [ ] **Step 8: Commit**

```bash
git add src/index.html src/main.js src/styles.css
git commit -m "Add header search box and sort control to the torrent list"
```

---

## PHASE 2 — Multi-select & bulk actions

### Task 3: Selection model (ctrl/shift-click) + selected styling

**Files:**
- Modify: `src/main.js` (selection state + renderList click handlers + rowHtml class), `src/styles.css`

- [ ] **Step 1: Add selection state**

In `src/main.js`, near the other state, add:

```js
let selected = new Set();      // infohashes currently selected
let lastClickedIh = null;      // anchor for shift-range selection
```

- [ ] **Step 2: Mark selected rows in rowHtml**

In `rowHtml(t)`, change the opening row div to include a `selected` class when selected:

```js
  return `<div class="torrent-row ${selected.has(t.infohash) ? "selected" : ""}" data-ih="${t.infohash}">
```

- [ ] **Step 3: Handle ctrl/shift-click in renderList**

In `renderList()`, replace the `grid.onclick` assignment with a handler that branches on modifier keys. The current code is:

```js
    const grid = n.querySelector(".row-grid");
    if (grid) grid.onclick = () => toggleExpand(n.dataset.ih);
```

Replace with:

```js
    const grid = n.querySelector(".row-grid");
    if (grid) grid.onclick = (e) => {
      const ih = n.dataset.ih;
      if (e.ctrlKey || e.metaKey) {
        if (selected.has(ih)) selected.delete(ih); else selected.add(ih);
        lastClickedIh = ih;
        renderList(); updateBulkBar();
      } else if (e.shiftKey && lastClickedIh) {
        const order = [...document.querySelectorAll(".torrent-row")].map(r => r.dataset.ih);
        const i = order.indexOf(lastClickedIh), j = order.indexOf(ih);
        if (i !== -1 && j !== -1) {
          const [lo, hi] = i < j ? [i, j] : [j, i];
          for (let k = lo; k <= hi; k++) selected.add(order[k]);
        }
        renderList(); updateBulkBar();
      } else {
        toggleExpand(ih);
      }
    };
```

- [ ] **Step 4: Clear selection on Escape and on plain background click**

In `src/main.js`, add a global key handler near the other top-level `window.addEventListener` calls:

```js
window.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && selected.size) {
    selected.clear(); lastClickedIh = null; renderList(); updateBulkBar();
  }
});
```

- [ ] **Step 5: Add selected-row style**

In `src/styles.css`, add after the `.row-grid:hover` rule:

```css
.torrent-row.selected .row-grid { background: var(--accent-soft); }
.torrent-row.selected .row-grid:hover { background: var(--accent-soft); }
```

- [ ] **Step 6: Add a temporary no-op updateBulkBar so the app runs**

In `src/main.js`, add (will be replaced in Task 4):

```js
function updateBulkBar() { /* implemented in Task 4 */ }
```

- [ ] **Step 7: Verify parse + commit**

Run: `node --check src/main.js` → exit 0.

```bash
git add src/main.js src/styles.css
git commit -m "Add ctrl/shift-click multi-selection with selected-row styling"
```

---

### Task 4: Bulk action bar (pause/resume/remove)

**Files:**
- Modify: `src/index.html` (bulk bar container), `src/main.js` (replace updateBulkBar + actions), `src/styles.css`

- [ ] **Step 1: Add the bulk bar container**

In `src/index.html`, add an empty container just inside the main content area, above `#torrent-list`:

```html
<div id="bulk-bar" class="bulk-bar" hidden></div>
```

- [ ] **Step 2: Style the bulk bar**

In `src/styles.css`:

```css
.bulk-bar {
  display: flex; align-items: center; gap: 10px;
  background: var(--surface); border: 1px solid var(--line);
  border-radius: 10px; padding: 8px 12px; margin-bottom: 12px; font-size: 13px;
}
.bulk-bar .count { font-weight: 500; }
.bulk-bar .spacer { flex: 1; }
.bulk-bar button { font-size: 12px; padding: 5px 11px; }
```

- [ ] **Step 3: Implement updateBulkBar + bulk actions**

In `src/main.js`, replace the temporary `function updateBulkBar() {}` from Task 3 with:

```js
function updateBulkBar() {
  const bar = document.getElementById("bulk-bar");
  if (!bar) return;
  if (selected.size === 0) { bar.hidden = true; bar.innerHTML = ""; return; }
  bar.hidden = false;
  bar.innerHTML = `
    <span class="count">${selected.size} selected</span>
    <div class="spacer"></div>
    <button class="btn-ghost" id="bulk-resume">Resume</button>
    <button class="btn-ghost" id="bulk-pause">Pause</button>
    <button class="btn-ghost" id="bulk-remove">Remove</button>
    <button class="btn-ghost" id="bulk-clear">Clear</button>`;
  document.getElementById("bulk-resume").onclick = () => bulkAction("resume");
  document.getElementById("bulk-pause").onclick  = () => bulkAction("pause");
  document.getElementById("bulk-remove").onclick = () => bulkRemove();
  document.getElementById("bulk-clear").onclick  = () => {
    selected.clear(); lastClickedIh = null; renderList(); updateBulkBar();
  };
}

async function bulkAction(cmd) {
  const ids = [...selected];
  for (const ih of ids) {
    try { await invoke(cmd, { infohash: ih }); } catch (e) { /* no-op if N/A */ }
  }
  torrents = await invoke("snapshot");
  selected.clear(); lastClickedIh = null;
  renderAll(); updateBulkBar();
}

async function bulkRemove() {
  const ids = [...selected];
  const del = confirm(`Remove ${ids.length} torrent(s)?\n\nOK = remove and DELETE downloaded files.\nCancel = keep files (just remove from list).`)
    // Two-step: first confirm intent, then ask about files. Keep it simple: OK=delete files.
  ;
  // If the user dismissed the dialog entirely, `confirm` returns false; treat
  // Cancel as "remove but keep files" only if they explicitly chose remove.
  const reallyRemove = window.confirm(`Proceed to remove ${ids.length} torrent(s)?`);
  if (!reallyRemove) return;
  for (const ih of ids) {
    try { await invoke("remove", { infohash: ih, deleteFiles: del }); } catch (e) {}
  }
  torrents = await invoke("snapshot");
  selected.clear(); lastClickedIh = null;
  renderAll(); updateBulkBar();
}
```

> NOTE: `confirm()` works in WebView2. If a nicer modal is desired later, reuse the existing modal pattern; for now native confirm keeps the task small.

- [ ] **Step 4: Keep selection valid after refreshes**

In `src/main.js`, in `applyProgress` where a structural `renderAll()` happens, prune any selected infohashes that no longer exist. At the top of `renderAll()` add:

```js
  for (const ih of [...selected]) if (!torrents.some(t => t.infohash === ih)) selected.delete(ih);
```

- [ ] **Step 5: Verify parse**

Run: `node --check src/main.js` → exit 0.

- [ ] **Step 6: Manual smoke**

`cd src-tauri; cargo run`. Ctrl-click two rows → bulk bar shows "2 selected"; Pause → both pause; shift-click a range → all selected; Remove → confirm → rows gone; Escape clears selection.

- [ ] **Step 7: Commit**

```bash
git add src/index.html src/main.js src/styles.css
git commit -m "Add bulk action bar (pause/resume/remove) for multi-selection"
```

---

## PHASE 3 — Queue management

### Task 5: Extend the state model (Queued + new fields)

**Files:**
- Modify: `src-tauri/src/state.rs`

- [ ] **Step 1: Write failing tests for the new fields + defaults**

In `src-tauri/src/state.rs`, add to the `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn legacy_record_loads_with_defaults() {
        // A record written before the queue fields existed must still deserialize.
        let json = r#"{"torrents":[{"infohash":"aaa","display_name":"x",
            "save_path":"C:/","state":"downloading","added_at":0,"total_size":0,
            "selected_files":null}]}"#;
        let s: PersistedState = serde_json::from_str(json).unwrap();
        assert_eq!(s.torrents.len(), 1);
        assert_eq!(s.torrents[0].queue_position, 0);
        assert_eq!(s.torrents[0].forced, false);
        assert_eq!(s.torrents[0].dl_limit, 0);
        assert_eq!(s.torrents[0].ul_limit, 0);
    }

    #[test]
    fn queued_state_serde_roundtrips() {
        let st = TorrentState::Queued;
        let j = serde_json::to_string(&st).unwrap();
        assert_eq!(j, "\"queued\"");
        let back: TorrentState = serde_json::from_str(&j).unwrap();
        assert_eq!(back, TorrentState::Queued);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd src-tauri; cargo test --lib state::tests::legacy_record_loads_with_defaults`
Expected: FAIL to compile (`queue_position` unknown field / `Queued` variant missing).

- [ ] **Step 3: Add the variant and fields**

In `src-tauri/src/state.rs`, change the enum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TorrentState { Downloading, Seeding, Paused, Completed, Stalled, Queued }
```

And the record struct — add the four fields with `#[serde(default)]`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentRecord {
    pub infohash: String,
    pub display_name: String,
    pub save_path: PathBuf,
    pub state: TorrentState,
    pub added_at: i64,
    pub total_size: u64,
    pub selected_files: Option<Vec<usize>>,
    #[serde(default)] pub queue_position: u32,
    #[serde(default)] pub forced: bool,
    #[serde(default)] pub dl_limit: u32,
    #[serde(default)] pub ul_limit: u32,
}
```

- [ ] **Step 4: Update the test helper `rec()` to set the new fields**

In the test module, change the `rec()` helper struct literal to include the new fields:

```rust
        TorrentRecord {
            infohash: ih.into(),
            display_name: "x".into(),
            save_path: PathBuf::from("C:/"),
            state: TorrentState::Downloading,
            added_at: 0, total_size: 0, selected_files: None,
            queue_position: 0, forced: false, dl_limit: 0, ul_limit: 0,
        }
```

- [ ] **Step 5: Add a helper to compute the next queue position**

In `src-tauri/src/state.rs`, add a method on `StateStore`:

```rust
    /// Highest existing queue_position + 1 (0 if empty). New torrents append to
    /// the end of the queue.
    pub fn next_queue_position(&self) -> u32 {
        self.inner.lock().unwrap().torrents.iter()
            .map(|t| t.queue_position).max().map(|m| m + 1).unwrap_or(0)
    }
```

- [ ] **Step 6: Run tests**

Run: `cd src-tauri; cargo test --lib state`
Expected: PASS (all state tests, including the two new ones).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/state.rs
git commit -m "Add Queued state and queue/limit fields to torrent record"
```

---

### Task 6: Pure queue `decide()` core + tests

**Files:**
- Create: `src-tauri/src/queue.rs`
- Modify: `src-tauri/src/lib.rs` (register module)

- [ ] **Step 1: Register the module**

In `src-tauri/src/lib.rs`, add alongside the other `pub mod` lines:

```rust
pub mod queue;
```

- [ ] **Step 2: Write the queue module with failing tests**

Create `src-tauri/src/queue.rs`:

```rust
//! Drift owns its download queue; librqbit has no queue concept. `decide()` is a
//! pure function: given the current torrent set and the max-active-downloads cap,
//! it returns which torrents should start (unpause) and which should be queued
//! (pause). A thin async wrapper applies the plan via the engine.

/// User intent for a torrent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Desired { Run, Pause }

/// A torrent's queue-relevant facts, built from its persisted record.
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub infohash: String,
    pub desired: Desired, // Run = eligible to download; Pause = user-paused (sticky)
    pub forced: bool,     // bypass the cap
    pub position: u32,    // lower = higher priority
    pub finished: bool,   // seeding/completed — does not need a download slot
    pub running_now: bool,// currently occupying a download slot (Downloading/Stalled)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct QueuePlan {
    pub to_start: Vec<String>, // unpause these (start downloading)
    pub to_pause: Vec<String>, // pause these (queued — over the cap)
}

/// Decide the plan. `max_active == 0` means unlimited.
///
/// Rules:
/// * Only non-finished, `Desired::Run` torrents are "eligible" (managed here).
/// * Forced eligible torrents always run, regardless of the cap.
/// * Remaining slots (cap minus forced-eligible count) go to the remaining
///   eligible torrents in ascending `position` order; the rest are queued.
/// * `to_start` = eligible-should-run torrents not already running.
/// * `to_pause` = eligible-should-queue torrents that are currently running.
/// * Finished and user-paused torrents are never touched.
pub fn decide(items: &[QueueItem], max_active: u32) -> QueuePlan {
    let unlimited = max_active == 0;

    let mut eligible: Vec<&QueueItem> =
        items.iter().filter(|i| i.desired == Desired::Run && !i.finished).collect();
    eligible.sort_by_key(|i| i.position);

    let forced_count = eligible.iter().filter(|i| i.forced).count() as u32;
    let mut remaining: i64 = if unlimited { i64::MAX } else { max_active as i64 - forced_count as i64 };

    let mut should_run: Vec<&str> = Vec::new();
    // Forced always run.
    for i in eligible.iter().filter(|i| i.forced) {
        should_run.push(&i.infohash);
    }
    // Then fill remaining slots with non-forced, in priority order.
    for i in eligible.iter().filter(|i| !i.forced) {
        if remaining > 0 {
            should_run.push(&i.infohash);
            remaining -= 1;
        }
    }

    let mut plan = QueuePlan::default();
    for i in &eligible {
        let wants_run = should_run.contains(&i.infohash.as_str());
        if wants_run && !i.running_now {
            plan.to_start.push(i.infohash.clone());
        } else if !wants_run && i.running_now {
            plan.to_pause.push(i.infohash.clone());
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(ih: &str, desired: Desired, forced: bool, pos: u32, finished: bool, running: bool) -> QueueItem {
        QueueItem { infohash: ih.into(), desired, forced, position: pos, finished, running_now: running }
    }

    #[test]
    fn cap_respected_starts_lowest_positions() {
        let items = vec![
            item("a", Desired::Run, false, 0, false, false),
            item("b", Desired::Run, false, 1, false, false),
            item("c", Desired::Run, false, 2, false, false),
        ];
        let plan = decide(&items, 2);
        assert_eq!(plan.to_start, vec!["a", "b"]);
        assert!(plan.to_pause.is_empty());
    }

    #[test]
    fn over_cap_running_gets_paused() {
        let items = vec![
            item("a", Desired::Run, false, 0, false, true),
            item("b", Desired::Run, false, 1, false, true),
            item("c", Desired::Run, false, 2, false, true),
        ];
        let plan = decide(&items, 2);
        // a,b should keep running (within cap, already running -> not in to_start),
        // c is over cap and running -> pause.
        assert!(plan.to_start.is_empty());
        assert_eq!(plan.to_pause, vec!["c"]);
    }

    #[test]
    fn forced_bypasses_cap() {
        let items = vec![
            item("a", Desired::Run, true,  5, false, false), // forced, low priority
            item("b", Desired::Run, false, 0, false, false),
            item("c", Desired::Run, false, 1, false, false),
        ];
        let plan = decide(&items, 1);
        // forced a always runs; remaining = 1 - 1 = 0 -> b,c stay queued.
        assert_eq!(plan.to_start, vec!["a"]);
        assert!(plan.to_pause.is_empty());
    }

    #[test]
    fn unlimited_starts_all_eligible() {
        let items = vec![
            item("a", Desired::Run, false, 0, false, false),
            item("b", Desired::Run, false, 1, false, false),
        ];
        let plan = decide(&items, 0);
        assert_eq!(plan.to_start, vec!["a", "b"]);
    }

    #[test]
    fn finished_and_paused_are_ignored() {
        let items = vec![
            item("seed", Desired::Run,  false, 0, true,  true),  // seeding, running -> ignored
            item("paus", Desired::Pause, false, 1, false, false),// user-paused -> ignored
            item("dl",   Desired::Run,  false, 2, false, false),
        ];
        let plan = decide(&items, 1);
        assert_eq!(plan.to_start, vec!["dl"]);
        assert!(plan.to_pause.is_empty());
    }

    #[test]
    fn idempotent_when_already_correct() {
        let items = vec![
            item("a", Desired::Run, false, 0, false, true),  // running, within cap
            item("b", Desired::Run, false, 1, false, false), // queued, over cap
        ];
        let plan = decide(&items, 1);
        assert!(plan.to_start.is_empty());
        assert!(plan.to_pause.is_empty());
    }
}
```

- [ ] **Step 3: Run to verify (tests should pass once it compiles)**

Run: `cd src-tauri; cargo test --lib queue`
Expected: PASS — all six `queue::tests` green.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/queue.rs src-tauri/src/lib.rs
git commit -m "Add pure queue decide() core with unit tests"
```

---

### Task 7: Max-active-downloads setting

**Files:**
- Modify: `src-tauri/src/settings.rs`, `src/main.js` (settings panel), `src/index.html` (none)

- [ ] **Step 1: Write a failing test**

In `src-tauri/src/settings.rs` test module, add:

```rust
    #[test]
    fn max_active_defaults_to_three() {
        let c = Config::default();
        assert_eq!(c.max_active_downloads, 3);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cd src-tauri; cargo test --lib settings::tests::max_active_defaults_to_three`
Expected: FAIL to compile (no field `max_active_downloads`).

- [ ] **Step 3: Add the field**

In `src-tauri/src/settings.rs`, add to `Config` (after `magnet_handler`):

```rust
    /// Max torrents downloading at once; the rest wait in the queue.
    /// 0 = unlimited. Defaulted for backward-compatible configs.
    #[serde(default = "default_max_active")]
    pub max_active_downloads: u32,
```

Add the default fn near `default_theme`:

```rust
fn default_max_active() -> u32 { 3 }
```

And set it in `Default for Config`:

```rust
            max_active_downloads: 3,
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri; cargo test --lib settings`
Expected: PASS.

- [ ] **Step 5: Surface it in the Settings panel UI**

In `src/main.js`, inside the settings panel HTML, add a new group after the "Behavior" group:

```js
    <div class="settings-group">
      <div class="group-label">Queue</div>
      <div class="settings-row"><span>Max active downloads (0 = unlimited)</span>
        <input class="num" type="text" id="s-maxactive" value="${cfg.max_active_downloads ?? 3}"></div>
    </div>
```

Then in the `s-save` click handler's `value` object, add the field:

```js
      max_active_downloads: +document.getElementById("s-maxactive").value || 0,
```

- [ ] **Step 6: Verify parse + commit**

Run: `node --check src/main.js` → exit 0.

```bash
git add src-tauri/src/settings.rs src/main.js
git commit -m "Add max-active-downloads setting (default 3)"
```

---

### Task 8: Queue controller wiring (apply plan; route add/pause/resume/startup)

**Files:**
- Modify: `src-tauri/src/queue.rs` (add async `apply_plan` + `build_items`), `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] **Step 1: Add the controller helpers to queue.rs**

In `src-tauri/src/queue.rs`, append (outside the test module):

```rust
use crate::engine::Engine;
use crate::magnet::InfoHash;
use crate::state::{StateStore, TorrentState};
use std::sync::Arc;

/// Build `decide()` inputs from the persisted state.
pub fn build_items(state: &StateStore) -> Vec<QueueItem> {
    state.snapshot().torrents.iter().map(|r| QueueItem {
        infohash: r.infohash.clone(),
        desired: if matches!(r.state, TorrentState::Paused) { Desired::Pause } else { Desired::Run },
        forced: r.forced,
        position: r.queue_position,
        finished: matches!(r.state, TorrentState::Seeding | TorrentState::Completed),
        running_now: matches!(r.state, TorrentState::Downloading | TorrentState::Stalled),
    }).collect()
}

/// Recompute the plan and apply it: unpause `to_start` (→ Downloading), pause
/// `to_pause` (→ Queued). Persists the resulting state labels.
pub async fn reconcile(engine: &Engine, state: &Arc<StateStore>, max_active: u32) {
    let plan = decide(&build_items(state), max_active);
    for ih in &plan.to_start {
        if engine.resume(&InfoHash(ih.clone())).await.is_ok() {
            if let Some(mut r) = find(state, ih) { r.state = TorrentState::Downloading; let _ = state.upsert(r); }
        }
    }
    for ih in &plan.to_pause {
        if engine.pause(&InfoHash(ih.clone())).await.is_ok() {
            if let Some(mut r) = find(state, ih) { r.state = TorrentState::Queued; let _ = state.upsert(r); }
        }
    }
}

fn find(state: &StateStore, ih: &str) -> Option<crate::state::TorrentRecord> {
    state.snapshot().torrents.into_iter().find(|t| t.infohash == ih)
}
```

- [ ] **Step 2: New torrents get a queue position + reconcile after add**

In `src-tauri/src/commands.rs`, in `add_torrent`, when building the `TorrentRecord`, set the position and start it as a candidate. Change the `state: TorrentState::Downloading,` line region to capture a position and add fields:

```rust
    let pos = ctx.state.next_queue_position();
    ctx.state.upsert(TorrentRecord {
        infohash: ih.as_str().into(),
        display_name: meta.name.clone(),
        save_path,
        state: TorrentState::Downloading,
        added_at: chrono_now_ms(),
        total_size: meta.total_size,
        selected_files: req.selected_files,
        queue_position: pos,
        forced: false,
        dl_limit: 0,
        ul_limit: 0,
    }).map_err(|e| e.to_string())?;

    // Honor the queue cap: a freshly-added torrent may need to wait.
    let max_active = ctx.settings.get().max_active_downloads;
    crate::queue::reconcile(&ctx.engine, &ctx.state, max_active).await;
```

- [ ] **Step 2b: Add the `use` for the new symbols if needed**

`commands.rs` already imports `TorrentRecord, TorrentState`. No new `use` needed for `reconcile` (called by full path).

- [ ] **Step 3: Make `resume` queue-aware**

In `src-tauri/src/commands.rs`, replace the body of `resume` so it clears user-pause then reconciles (the torrent may end up Queued rather than Downloading if over cap):

```rust
#[tauri::command]
pub async fn resume(ctx: tauri::State<'_, AppCtx>, infohash: String) -> Result<(), String> {
    // Mark as eligible (not user-paused). The controller decides whether it runs
    // now or waits in the queue.
    if let Some(mut r) = ctx.state.snapshot().torrents.into_iter().find(|t| t.infohash == infohash) {
        if matches!(r.state, crate::state::TorrentState::Paused) {
            r.state = crate::state::TorrentState::Queued; // provisional; reconcile may promote to Downloading
            ctx.state.upsert(r).map_err(|e| e.to_string())?;
        }
    }
    let max_active = ctx.settings.get().max_active_downloads;
    crate::queue::reconcile(&ctx.engine, &ctx.state, max_active).await;
    Ok(())
}
```

- [ ] **Step 4: Make `pause` reconcile afterwards (free a slot)**

In `src-tauri/src/commands.rs`, in `pause`, after setting state to Paused and persisting, add a reconcile so a queued torrent can take the freed slot:

```rust
#[tauri::command]
pub async fn pause(ctx: tauri::State<'_, AppCtx>, infohash: String) -> Result<(), String> {
    ctx.engine.pause(&InfoHash(infohash.clone())).await.map_err(|e| e.to_string())?;
    if let Some(mut r) = ctx.state.snapshot().torrents.into_iter().find(|t| t.infohash == infohash) {
        r.state = TorrentState::Paused;
        r.forced = false; // pausing clears any forced flag
        ctx.state.upsert(r).map_err(|e| e.to_string())?;
    }
    let max_active = ctx.settings.get().max_active_downloads;
    crate::queue::reconcile(&ctx.engine, &ctx.state, max_active).await;
    Ok(())
}
```

- [ ] **Step 5: Reconcile when a download finishes (in the progress task)**

In `src-tauri/src/main.rs`, in the progress fan-out task, when a torrent transitions to `seeding`/`completed`, a download slot frees up. After the block that persists the new state (the `if let Some(s) = new_state { ... }` region), add a reconcile when the new state is a finishing one. Insert after `state_for_emit.upsert(rec)` handling, still inside the `if prev_emitted...` block:

```rust
                            // A finished download frees a slot — let the queue advance.
                            if matches!(u.state_label.as_str(), "seeding" | "completed") {
                                let max_active = settings_for_emit.get().max_active_downloads;
                                crate::queue::reconcile(&engine_for_emit, &state_for_emit, max_active).await;
                            }
```

This requires the progress task to capture clones of the engine and settings. Where the task is spawned in `main.rs` setup, add before the `tauri::async_runtime::spawn`:

```rust
            let engine_for_emit = engine.clone();
            let settings_for_emit = settings.clone();
```

and move them into the async block (they are captured by the `move` closure automatically once referenced).

- [ ] **Step 6: Route resume-on-launch through the controller**

In `src-tauri/src/main.rs`, the startup resume loop currently unpauses every non-paused torrent directly. Replace that loop's effect with a single reconcile so the cap is honored on launch. After the existing resume loop (or replacing its body), add:

```rust
            // Honor the queue cap on launch instead of blindly resuming everything.
            {
                let max_active = settings.get().max_active_downloads;
                tauri::async_runtime::block_on(
                    crate::queue::reconcile(&engine, &state, max_active)
                );
            }
```

Keep the existing per-torrent `resume_existing` loop (it re-attaches handles); the reconcile then pauses any that exceed the cap and marks them Queued.

- [ ] **Step 7: Build to verify it compiles**

Run: `cd src-tauri; cargo build`
Expected: compiles (warnings ok).

- [ ] **Step 8: Run the full test suite**

Run: `cd src-tauri; cargo test`
Expected: PASS (state, settings, queue, existing tests).

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/queue.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "Wire queue controller into add/pause/resume/finish/startup"
```

---

### Task 9: Force-start + reorder commands

**Files:**
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/main.rs` (register commands)

- [ ] **Step 1: Add the `force_start` command**

In `src-tauri/src/commands.rs`, add:

```rust
/// Force a torrent to run regardless of the active-downloads cap.
#[tauri::command]
pub async fn force_start(ctx: tauri::State<'_, AppCtx>, infohash: String) -> Result<(), String> {
    if let Some(mut r) = ctx.state.snapshot().torrents.into_iter().find(|t| t.infohash == infohash) {
        r.forced = true;
        if matches!(r.state, TorrentState::Paused | TorrentState::Queued) {
            r.state = TorrentState::Downloading; // provisional; reconcile confirms
        }
        ctx.state.upsert(r).map_err(|e| e.to_string())?;
    }
    let max_active = ctx.settings.get().max_active_downloads;
    crate::queue::reconcile(&ctx.engine, &ctx.state, max_active).await;
    Ok(())
}
```

- [ ] **Step 2: Add the `move_in_queue` command**

In `src-tauri/src/commands.rs`, add a command that reorders by recomputing positions. It supports four directions via a string:

```rust
/// Reorder a torrent's queue priority. `dir` is "top" | "up" | "down" | "bottom".
#[tauri::command]
pub async fn move_in_queue(ctx: tauri::State<'_, AppCtx>, infohash: String, dir: String) -> Result<(), String> {
    // Work on a sorted-by-position vector, move the target, then renumber 0..n.
    let mut recs = ctx.state.snapshot().torrents;
    recs.sort_by_key(|r| r.queue_position);
    let idx = recs.iter().position(|r| r.infohash == infohash)
        .ok_or_else(|| "torrent not found".to_string())?;
    let new_idx = match dir.as_str() {
        "top" => 0,
        "bottom" => recs.len().saturating_sub(1),
        "up" => idx.saturating_sub(1),
        "down" => (idx + 1).min(recs.len().saturating_sub(1)),
        _ => return Err("bad direction".into()),
    };
    if new_idx != idx {
        let item = recs.remove(idx);
        recs.insert(new_idx, item);
    }
    // Renumber and persist.
    for (i, r) in recs.iter_mut().enumerate() {
        r.queue_position = i as u32;
        ctx.state.upsert(r.clone()).map_err(|e| e.to_string())?;
    }
    let max_active = ctx.settings.get().max_active_downloads;
    crate::queue::reconcile(&ctx.engine, &ctx.state, max_active).await;
    Ok(())
}
```

- [ ] **Step 3: Register the new commands**

In `src-tauri/src/main.rs`, add to the `tauri::generate_handler![ ... ]` list:

```rust
            commands::force_start,
            commands::move_in_queue,
```

- [ ] **Step 4: Build + test**

Run: `cd src-tauri; cargo build && cargo test`
Expected: compiles and tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "Add force_start and move_in_queue commands"
```

---

### Task 10: Frontend — Queued state, sidebar filter, context menu entries

**Files:**
- Modify: `src/main.js` (FILTERS, iconKey, context menu, state label handling), `src/styles.css`, `src-tauri/src/main.rs` (preserve Queued in progress persistence)

- [ ] **Step 1: Preserve Queued vs Paused in the progress→state persistence**

In `src-tauri/src/main.rs`, the progress task maps `u.state_label` → `TorrentState`. When the engine reports `"paused"`, it must NOT clobber a record we deliberately set to `Queued` (queued torrents are paused at the engine level). Change the match so `"paused"` is handled relative to the existing record. Replace the `new_state` match arm for paused with logic that preserves Queued:

Find:

```rust
                                "paused"      => Some(TorrentState::Paused),
```

Replace with:

```rust
                                // Engine reports "paused" for BOTH user-paused and
                                // queued torrents. Preserve whichever the record
                                // already holds; never downgrade Queued -> Paused.
                                "paused" => Some(match rec.state {
                                    TorrentState::Queued => TorrentState::Queued,
                                    _ => TorrentState::Paused,
                                }),
```

(`rec` is already the matched record in that scope.)

- [ ] **Step 2: Add Queued to the sidebar filters**

In `src/main.js`, add to the `FILTERS` array (after `paused`):

```js
  { key: "queued", label: "Queued" },
```

And in `renderSidebar`, extend `iconKey`:

```js
  const iconKey = { all: "all", downloading: "downloading", seeding: "seeding", completed: "completed", paused: "paused", queued: "downloading" };
```

(Reuse the downloading icon for queued, or add a dedicated glyph in `icons.js` if desired.)

- [ ] **Step 3: Add a color token + state styling for queued**

In `src/styles.css` `:root`, add:

```css
  --st-queued:      #B0A99C;
```

In `:root[data-theme="dark"]`, add:

```css
  --st-queued:      #B0A99C;
```

The row state pill already uses `var(--st-<label>)` via `rowHtml`'s `stColorVar`, so `queued` picks up automatically. The label capitalization in `rowHtml` also works (`"Queued"`).

- [ ] **Step 4: Add context-menu entries (Force start + reorder)**

In `src/main.js` `openContextMenu`, extend the `items` array. After the pause/resume item, add force-start when not actively downloading, and reorder entries. Insert into the `items` array:

```js
    ...( (t.state_label === "queued" || t.state_label === "paused")
        ? [{ label: "Force start", fn: () => invoke("force_start", { infohash: ih }) }]
        : [] ),
    { label: "Move to top",    fn: () => invoke("move_in_queue", { infohash: ih, dir: "top" }) },
    { label: "Move up",        fn: () => invoke("move_in_queue", { infohash: ih, dir: "up" }) },
    { label: "Move down",      fn: () => invoke("move_in_queue", { infohash: ih, dir: "down" }) },
    { label: "Move to bottom", fn: () => invoke("move_in_queue", { infohash: ih, dir: "bottom" }) },
```

(Place these before the "Remove" entries so destructive actions stay last.)

- [ ] **Step 5: Verify parse**

Run: `node --check src/main.js` → exit 0.

- [ ] **Step 6: Build + manual smoke**

Run: `cd src-tauri; cargo run`. Set Max active downloads = 1 in Settings, Save. Add two torrents → one Downloading, one Queued (visible in the Queued sidebar filter). Pause the active one → the queued one starts. Right-click the queued one → Force start → both run. Right-click → Move to top → priority changes. Restart the app → at most 1 downloads, the rest Queued.

- [ ] **Step 7: Commit**

```bash
git add src/main.js src/styles.css src-tauri/src/main.rs
git commit -m "Frontend: queued state, sidebar filter, force-start and reorder menu"
```

---

## PHASE 4 — Per-torrent speed limits (GATED)

### Task 11: Feasibility spike — does librqbit support per-torrent rate limits?

**Files:** none (research + a decision note)

- [ ] **Step 1: Investigate the librqbit API**

Determine whether the pinned librqbit (8.1.x) exposes a per-`ManagedTorrent` rate limiter or per-torrent add-options for upload/download caps. Check:

```bash
cd src-tauri
cargo doc -p librqbit --no-deps
```

Then inspect, in the generated docs or source, for any of: a `ratelimits` field on `ManagedTorrent` (mirroring `Session::ratelimits`), per-torrent options in `AddTorrentOptions`, or a method like `set_download_bps`/`set_upload_bps` on the torrent handle. Also grep the dependency source:

Use the Grep tool over the cargo registry source for librqbit for `ratelimit` on torrent/state types.

- [ ] **Step 2: Record the decision**

Append a short "Feasibility result" note to the spec file `docs/superpowers/specs/2026-05-28-library-management-and-control-design.md` under Feature 4, stating SUPPORTED (with the exact API) or NOT SUPPORTED.

- [ ] **Step 3: Branch**

- If **SUPPORTED** → proceed to Task 12.
- If **NOT SUPPORTED** → mark Feature 4 dropped in the spec, commit the note, and **stop here**. The phase is complete without per-torrent limits, per the pre-agreed exit.

```bash
git add docs/superpowers/specs/2026-05-28-library-management-and-control-design.md
git commit -m "Record per-torrent rate-limit feasibility result"
```

---

### Task 12 (CONDITIONAL — only if Task 11 == SUPPORTED): Per-torrent limits

**Files:**
- Modify: `src-tauri/src/engine.rs` (per-torrent limit method), `src-tauri/src/commands.rs` (command), `src-tauri/src/main.rs` (register + apply on resume), `src/main.js` (menu entry + dialog + expanded-row display)

- [ ] **Step 1: Add an engine method**

In `src-tauri/src/engine.rs`, add a method using the API confirmed in Task 11 (example shape — adapt to the real API):

```rust
    /// Set per-torrent rate limits (KB/s; 0 = unlimited for that direction).
    pub fn set_torrent_limits(&self, ih: &InfoHash, down_kbps: u32, up_kbps: u32) -> Result<()> {
        let handle = self.get_handle(ih)?;
        let down = std::num::NonZeroU32::new(down_kbps.saturating_mul(1024));
        let up = std::num::NonZeroU32::new(up_kbps.saturating_mul(1024));
        // Replace with the actual per-torrent ratelimits API confirmed in the spike:
        handle.ratelimits().set_download_bps(down);
        handle.ratelimits().set_upload_bps(up);
        Ok(())
    }
```

- [ ] **Step 2: Add the command**

In `src-tauri/src/commands.rs`:

```rust
/// Set per-torrent download/upload limits (KB/s; 0 = unlimited). Persists to state.
#[tauri::command]
pub async fn set_torrent_limits(ctx: tauri::State<'_, AppCtx>, infohash: String, dl_limit: u32, ul_limit: u32) -> Result<(), String> {
    ctx.engine.set_torrent_limits(&InfoHash(infohash.clone()), dl_limit, ul_limit).map_err(|e| e.to_string())?;
    if let Some(mut r) = ctx.state.snapshot().torrents.into_iter().find(|t| t.infohash == infohash) {
        r.dl_limit = dl_limit; r.ul_limit = ul_limit;
        ctx.state.upsert(r).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 3: Register the command**

In `src-tauri/src/main.rs` handler list: `commands::set_torrent_limits,`.

- [ ] **Step 4: Re-apply saved limits on resume**

In `src-tauri/src/queue.rs` `reconcile`, after a successful `to_start` resume, re-apply the record's saved limits:

```rust
            if let Some(mut r) = find(state, ih) {
                r.state = TorrentState::Downloading;
                let _ = state.upsert(r.clone());
                if r.dl_limit > 0 || r.ul_limit > 0 {
                    let _ = engine.set_torrent_limits(&InfoHash(ih.clone()), r.dl_limit, r.ul_limit);
                }
            }
```

- [ ] **Step 5: Frontend — menu entry + dialog + display**

In `src/main.js` `openContextMenu`, add an item:

```js
    { label: "Set speed limit…", fn: () => openLimitDialog(ih) },
```

Add a small dialog function near `openAddDialog`:

```js
function openLimitDialog(ih) {
  const t = torrents.find(x => x.infohash === ih) || {};
  const root = document.getElementById("modal-root");
  root.innerHTML = `
    <div class="modal-backdrop"><div class="modal">
      <h2>Speed limit</h2>
      <div class="settings-row"><span>Download (KB/s, 0 = unlimited)</span>
        <input class="num" id="lim-dl" type="text" value="${t.dl_limit || 0}"></div>
      <div class="settings-row"><span>Upload (KB/s, 0 = unlimited)</span>
        <input class="num" id="lim-ul" type="text" value="${t.ul_limit || 0}"></div>
      <div style="display:flex; justify-content:flex-end; gap:8px; margin-top:16px">
        <button class="btn-ghost" id="lim-cancel">Cancel</button>
        <button class="btn-primary" id="lim-save">Save</button>
      </div>
    </div></div>`;
  document.getElementById("lim-cancel").onclick = () => root.innerHTML = "";
  document.getElementById("lim-save").onclick = async () => {
    try {
      await invoke("set_torrent_limits", {
        infohash: ih,
        dlLimit: +document.getElementById("lim-dl").value || 0,
        ulLimit: +document.getElementById("lim-ul").value || 0,
      });
      root.innerHTML = "";
      torrents = await invoke("snapshot"); renderAll();
    } catch (e) { showToast("error", friendlyError(e)); }
  };
}
```

> Note: the `snapshot` command returns a `TorrentDto`, which does not include `dl_limit`/`ul_limit`. To show current values in the dialog and expanded row, extend `TorrentDto` (`events.rs`) and the `snapshot`/progress mapping to carry `dl_limit`/`ul_limit`, mirroring how `added_at` is carried. Add that mapping in this step.

- [ ] **Step 6: Build + test + manual smoke**

Run: `cd src-tauri; cargo build && cargo test`. Then `cargo run`: set a 50 KB/s download limit on an active torrent → its speed caps near 50 KB/s; restart → limit re-applies.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Add per-torrent speed limits (gated feature)"
```

---

## Finalization

### Task 13: Version bump + full verification

**Files:** `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`

- [ ] **Step 1: Bump versions to 0.4.0**

In `src-tauri/tauri.conf.json` set `"version": "0.4.0"`. In `src-tauri/Cargo.toml` set `version = "0.4.0"`.

- [ ] **Step 2: Full test run**

Run: `cd src-tauri; cargo test` → PASS. Run: `node --test src/list-ops.test.js` → PASS. Run: `node --check src/main.js` → exit 0.

- [ ] **Step 3: Commit (do NOT build installers or publish unless asked)**

```bash
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m "Bump version to 0.4.0"
```

> Per standing instruction: do not run `cargo tauri build` or publish a release until the user explicitly requests it.

---

## Self-Review (completed by plan author)

**Spec coverage:**
- Search & sort → Tasks 1–2 ✓
- Multi-select + bulk actions → Tasks 3–4 ✓
- Queue: max-active setting → Task 7 ✓; Queued state → Task 5 ✓; controller/auto-rotation → Tasks 6, 8 ✓; force-start + reorder → Task 9 ✓; sidebar + menu + persistence preservation → Task 10 ✓; resume-on-launch through controller → Task 8 Step 6 ✓
- Per-torrent limits gated → Tasks 11–12 ✓
- State/settings backward-compat defaults → Tasks 5, 7 ✓
- Tests: queue core, sort comparator, search filter → Tasks 6, 1 ✓
- Version 0.4.0 → Task 13 ✓

**Placeholder scan:** Task 12 marks the engine `ratelimits()` call as "adapt to the real API" — this is intentional because the exact symbol is unknown until Task 11's spike; the surrounding command/state/UI code is complete. No other placeholders.

**Type consistency:** `decide(items, max_active) -> QueuePlan`, `QueueItem`/`Desired`/`QueuePlan`, `build_items`, `reconcile`, `next_queue_position`, `force_start`, `move_in_queue(dir)`, `set_torrent_limits(dl_limit, ul_limit)` are used consistently across Rust tasks. Frontend `filterTorrents(torrents, stateFilter, query)` and `sortTorrents(torrents, key, dir)` signatures match between Task 1 (definition) and Task 2 (use). Tauri arg casing: Rust snake_case params (`delete_files`, `dl_limit`) map to JS camelCase (`deleteFiles`, `dlLimit`) per Tauri convention, matching the existing `remove` call.

**Known follow-up:** showing live per-torrent limit/queued extras in the expanded row depends on extending `TorrentDto` (noted in Task 12 Step 5).
