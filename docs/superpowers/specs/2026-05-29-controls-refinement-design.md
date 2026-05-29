# Drift — Search / Sort / Selection Controls Refinement

**Status:** Design approved, ready for planning
**Date:** 2026-05-29
**Author:** brainstormed with Claude (visual mockups)

## Goal

The list controls added in v0.4.0 work but look unrefined next to the rest of
Drift — specifically the **native OS `<select>`** sort dropdown and the **plain
text `↑`/`↓`** direction button, which render with OS-default styling that
clashes with Drift's warm, crafted aesthetic. This phase replaces them with
custom, on-brand controls and lightly refines the search field and the
multi-select action bar.

Frontend-only, no Rust changes. Targets a 0.4.x patch release.

**In scope:** search field polish, sort control (custom pill + dropdown menu),
selection bar visual refinement.
**Out of scope:** sidebar filters (deliberately left as-is), the selection
*behavior* (ctrl/shift-click model is unchanged — only the bar's look changes),
and any backend/queue logic.

---

## Feature 1 — Search field

Refine the existing `#list-search` input:
- Add a **leading magnifier icon** inside the field (absolutely positioned, the
  input gets left padding so text clears it). Icon uses `currentColor` at
  `--ink-soft`.
- Placeholder text: **"Search downloads…"**.
- Border radius `9px`; fixed width `220px`. On focus, only the **border color**
  animates to `--accent` (no width animation — width animation caused the
  earlier reflow/flicker, already removed).

A new `search` icon is added to `src/icons.js` (magnifier: circle + handle).

Behavior is unchanged: live substring filter via the existing `searchQuery`
state and `filterTorrents`.

---

## Feature 2 — Sort control (the core change)

Replace BOTH the native `<select id="list-sort">` and the separate
`<button id="list-sortdir">` with a **single custom sort pill** plus a **custom
dropdown menu**.

### The pill (`#sort-trigger`)
A ghost-button styled to match Drift's surfaces:
`[sort-lines icon]  Sort: <Active Label>  [chevron-down]`
- Height 34px, radius 9px, `--surface` background, `--line` border; hover →
  `--accent` border (same language as other controls).
- `<Active Label>` reflects the current sort key (Date added / Name / Progress /
  Speed / Size).
- A new `sort` icon (three descending lines) and `chevron` icon are added to
  `src/icons.js`.

### The menu (`#sort-menu`)
Opens on pill click, styled with the existing context-menu tokens
(`.context-menu` look: `--bg`, `--line`, `radius-sm`, soft shadow):
- A small uppercase group label "Sort by".
- One row per key. The **active** key row uses the `--accent-soft` /`--accent`
  active treatment, shows a leading **check** icon, and a trailing
  **direction glyph** with a word: `↓ newest` / `↑ oldest` for Date added,
  and generic `↓`/`↑` (or `Z→A`/`A→Z` style is NOT required — keep simple
  arrows) for the others.
- Inactive rows have a blank leading slot (no check) and no direction glyph.

### Interaction
- Click an **inactive** key → switch to that key, keep the current direction,
  re-render, close the menu, persist.
- Click the **active** key → **flip the direction** (asc⇄desc), re-render,
  close, persist.
- Click outside or press Escape → close the menu (reuse the existing
  context-menu dismiss approach: a one-shot document click listener +
  `closeContextMenu`-style teardown; do not leave duplicate listeners).
- Persistence unchanged: `localStorage` keys `drift-sort-key` and
  `drift-sort-dir`.

### Direction glyph wording
Per key, the active-row trailing label is:
- `added`    → `↓ newest` (desc) / `↑ oldest` (asc)
- `name`     → `↓ Z–A` (desc) / `↑ A–Z` (asc)
- `progress` → `↓ high` (desc) / `↑ low` (asc)
- `speed`    → `↓ fast` (desc) / `↑ slow` (asc)
- `size`     → `↓ large` (desc) / `↑ small` (asc)

This makes the otherwise-ambiguous direction meaningful at a glance.

### Removed
- The native `<select id="list-sort">` element.
- The `<button id="list-sortdir">` element.
Their CSS rules (`.list-sort`, `.list-sortdir`) are replaced by the new
`.sort-trigger` / `.sort-menu` rules.

---

## Feature 3 — Selection action bar (visual refinement only)

Keep the inline `#bulk-bar` above the list (NOT a floating pill — decided), but
refine its look:
- Each action button gets a small leading **icon**: Resume (down-arrow into
  tray glyph), Pause (two bars), Remove (trash). Clear stays text or gets an ✕.
- Tighten: consistent button height, slightly stronger "N selected" weight,
  actions grouped to the right, Clear visually separated (e.g. a thin divider
  or an ✕ icon button).
- Reuse the existing `btn-ghost` base; add an optional `.has-icon` treatment
  (icon + label with a small gap).
- The `[hidden]` fix from the prior commit stays (bar only shows when
  `selected.size > 0`).

New icons may be reused from existing ones where possible (Pause already exists
conceptually via the `paused` filter glyph); add `resume`/`trash` to `icons.js`
if not present.

---

## Files touched

- `src/index.html` — replace the `<select>` + dir `<button>` in `.list-controls`
  with the search-icon wrapper + the `#sort-trigger` pill; menu is created in JS.
- `src/icons.js` — add `search`, `sort`, `chevron`, `trash` (and `resume` if
  needed) icons.
- `src/styles.css` — search-with-icon, `.sort-trigger`, `.sort-menu`
  (+ active row, check, direction glyph), refined `.bulk-bar` buttons; remove
  `.list-sort` / `.list-sortdir`.
- `src/main.js` — replace the `wireListControls()` native-select/dir-button
  wiring with: render the sort pill label, open/close the custom menu, handle
  key-select vs active-key-direction-flip, keep `renderList()` + localStorage;
  add icons in `updateBulkBar()`.

No Rust, no `state.json`/`config.json` changes.

---

## Testing

- **Unit (frontend):** the existing `list-ops.test.js` already covers the sort
  comparator + search filter; no logic change there. Add a tiny pure helper
  `sortDirectionLabel(key, dir)` (returns the glyph/word per the table above)
  and unit-test it in `list-ops.test.js` so the wording map is verified.
- **Manual smoke:** open sort menu; switch keys; click active key to flip
  direction; verify arrow + word; reopen reflects state; restart persists;
  search icon + focus border; select rows → refined bar with icons; Escape /
  outside-click closes the menu; no leftover document listeners (open/close
  repeatedly, confirm a single dismiss each time).

## Risks

- **Menu dismiss listener leakage:** the existing context menu uses a one-shot
  `document.addEventListener("click", …, {once:true})`. The sort menu must use
  the same disciplined teardown so repeated open/close doesn't stack listeners
  or immediately self-close. Mitigated by following the existing pattern exactly.
- **Click-through:** the pill click that opens the menu must not be immediately
  caught by the outside-click closer (guard with `stopPropagation` or defer
  attaching the closer, as the existing context menu does).
