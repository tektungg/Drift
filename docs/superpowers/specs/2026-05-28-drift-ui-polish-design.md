# Drift UI/UX Polish — Design Spec

**Date:** 2026-05-28
**Status:** Approved for implementation planning
**Scope:** Visual/UX polish of the existing Drift interface. **No layout restructuring** — the sidebar + list structure stays exactly as-is. This is a refinement pass within that structure.

---

## 1. Goal

Drift is functionally complete but visually sparse: plain-text torrent rows, no icons, no empty state, weak at-a-glance state differentiation, a plain sidebar, and unrefined dialogs. This spec polishes all four areas while staying true to the warm, restrained Claude-inspired aesthetic (cream background, coral accent, serif headings, generous whitespace).

**Constraint:** Frontend-only. The Rust backend already emits everything the UI needs (`progress` events and the `snapshot` command carry `name`, `state_label`, `downloaded`, `total`, `down_bps`, `up_bps`, `peers`; `torrent_files`/`peek` return each file's `path`). No new commands, events, or Rust changes are required.

Note on the row icon: the torrent-list row only has the torrent **name** available (not the file list — that's fetched lazily on expand). So the row's file-type icon is derived from the **torrent name's extension** when present (e.g. `movie.mkv` → Video), falling back to a generic **folder** icon when the name has no recognizable media extension (which is the common case for multi-file/folder torrents). The Add-dialog file rows, which DO have each file's path, derive an accurate icon per file.

**Non-goals:** No card grid, no detail pane, no dashboard header. The magnet-toast window and native Windows titlebar stay as they are.

---

## 2. Design tokens (additions to `:root`)

Add these to the existing palette in `styles.css`. State colors are intentionally desaturated to sit calmly on the cream:

```css
--st-downloading: #D97757;  /* coral — same as --accent */
--st-seeding:     #7D9B76;  /* sage green */
--st-completed:   #6E8F86;  /* muted teal */
--st-paused:      #A8A096;  /* warm gray */
--st-stalled:     #CDA04E;  /* amber */
--st-error:       #C0573C;  /* deep coral-red (reuse for "error" state) */

--prog-track:     #ECE5D6;  /* progress bar background (replaces --accent-soft as the track) */
--icon-bg:        #EFE9DB;  /* neutral square behind a file-type line icon */
--icon-fg:        #6B645A;  /* line-icon stroke color (= --ink-soft) */
```

A small helper maps a `state_label` string to its color via a CSS class (`.st-downloading`, `.st-seeding`, …) so both the dot and the progress fill can reference it.

---

## 3. Torrent rows ("Dot + tinted progress")

Each row is a 3-column grid: **icon · body · right-meta**.

- **Icon (left):** 30×30 rounded square (`--icon-bg`) containing a monochrome line SVG for the file-type category (see §6). Derived from the torrent's dominant file extension.
- **Body (center):**
  - Torrent name (one line, ellipsis on overflow).
  - Progress bar: 6px, rounded, track `--prog-track`, fill tinted to the **state color** (coral while downloading, sage while seeding, etc.). Width transitions smoothly (`transition: width 0.3s ease`).
  - Meta line (11px, `--ink-soft`): `<downloaded> / <total> · <speed> · <peers> peers · ETA <eta>` while downloading; collapses to relevant fields otherwise (e.g. seeding shows `<size> · ratio <r> · ↑ <up>`; paused shows `<downloaded> / <total> · paused`).
- **Right-meta:** percent (bold) on top; below it a state label preceded by a small colored dot (8px circle in the state color).

**Interactions:**
- Hover: row background lifts to `rgba(0,0,0,0.02)`.
- Click: expands the file list inline (existing behavior; restyle the expanded panel to use the same line icons and the `--prog-track` bars).
- Right-click: existing context menu (unchanged behavior; restyle for consistency).

**ETA:** compute on the frontend from `down_bps` and remaining bytes (`(total-downloaded)/down_bps`), formatted as `2m`, `1h 12m`, `<1m`, or `—` when speed is 0.

---

## 4. Empty state ("With action hints")

Shown in the main content area when the **current filter** has zero torrents.

- Centered column: a 64×64 rounded square (`--accent-soft`) holding a wave glyph (the Drift mark — a simple line "wave" SVG, monochrome coral), a serif heading, a one-line subtitle, then three **hint cards**.
- **All filter, zero torrents** (first run): heading "Nothing downloading yet", subtitle "Three ways to get started:", then three hint cards:
  1. 🔗-style link icon — **Paste a magnet link** — "Drift watches your clipboard and offers to add it."
  2. file icon — **Drop a `.torrent` file** — "Drag it anywhere onto this window."
  3. plus icon — **Click Add torrent** — "Paste or browse in the dialog."
- **Other filters, zero matches** (e.g. Seeding filter with nothing seeding): a lighter variant — just the glyph + a contextual line ("Nothing seeding right now."), no hint cards. Reuses the same component with a `variant="filter"` flag.

Hint-card icons use the monochrome line set; cards are `--surface` with a 1px `--line` border, left-aligned text.

---

## 5. Sidebar refinements

Keep the existing structure (filters list → spacer → bottom block). Refinements:

- Each filter row gets a **leading line icon** (15px, `--icon-fg`) + label + live count (right-aligned, muted). Icons:
  - All → grid/list glyph
  - Downloading → down-arrow
  - Seeding → up-arrow
  - Completed → check
  - Paused → pause bars
- Active filter: `--accent-soft` background (existing), icon opacity 1.
- **Totals card** (bottom, above Settings): a bordered `--bg` card with an uppercase "TOTAL" label and a row showing `↓ <down>/s` and `↑ <up>/s` (aggregate across all torrents). Replaces the current loose text.
- **Settings**: a gear line-icon + "Settings" pinned at the very bottom (existing position, restyled to match filter rows).

---

## 6. Icon system (monochrome line SVGs)

A single source-of-truth JS module/object mapping a key → inline SVG markup string. All icons share `viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"` and inherit color via `currentColor` (set to `--icon-fg`, or a state color where relevant).

**File-type categories** (derived from extension, reusing the same category logic as the backend's `CategoryMap`):
- Video, Audio, Image, Document, Archive, Program, Other (generic file).

**UI icons:** sidebar filters (grid, down-arrow, up-arrow, check, pause, gear), empty-state hints (link, file, plus), and the Drift wave glyph.

A small `extToCategory(filename)` helper (mirroring the backend's `CategoryMap` extension lists) maps a filename to a category key, and `iconFor(category)` returns the SVG.

- **Torrent rows:** call `extToCategory(torrent.name)`. If it resolves to a real media category (the name ends in a known extension, typical of single-file torrents), use that icon; otherwise use the generic **folder** icon (the common case for multi-file/folder torrents, whose name has no extension). This needs only the `name` the row already has — no file-list fetch.
- **Add-dialog file rows:** call `extToCategory(file.path)` per file for an accurate per-file icon (paths are available here).

---

## 7. Dialogs & toasts

**Add Torrent dialog:**
- File list rows gain the line file-type icon and use `--prog-track`-style spacing.
- Save-path row keeps the front-truncated `code` + Change…/Reset (already implemented); restyle to match.
- Checkboxes use `accent-color: var(--accent)`.

**Settings panel:**
- Group fields under uppercase section labels: **Downloads** (default folder, download limit, upload limit), **Behavior** (clipboard watch, close-to-tray, start with Windows), **Categories** (a disclosure that expands the extension editors — keep existing `<details>` but restyle).
- Replace the three boolean **checkboxes with toggle switches**: a 34×20 rounded pill, coral when on, `#D8D0C0` when off, with a white knob that slides. Pure CSS (`.switch` / `.switch.off`), driven by a hidden checkbox or a `data-on` attribute toggled in JS.

**Toasts:** keep the existing toast-stack; align padding/border-radius with the refreshed tokens. Error toasts keep the coral border.

---

## 8. Micro-interactions

- Progress bar width: `transition: width 0.3s ease` (exists; verify it survives the restyle).
- Row hover: 120ms background fade.
- Toggle switch knob: `transition: all 0.18s ease`.
- Settings panel slide-in: existing `transform 0.2s` (unchanged).
- Expanded file panel: appears/disappears without layout jank (no animation required, but must not flash).

Keep motion subtle and fast — nothing bouncy.

---

## 9. Files touched

- `src/styles.css` — new tokens, state-color classes, row restyle, sidebar restyle, empty-state styles, toggle-switch styles, icon-square styles.
- `src/main.js` — icon module + `extToCategory` helper, row render update (icon + tinted progress + dot/label + ETA), sidebar render update (filter icons + totals card), empty-state render (both variants), settings render (sectioned + toggle switches), Add-dialog file-row icons.
- `src/index.html` — minimal/no change (containers already exist).
- No Rust changes.

If `main.js` grows unwieldy with the icon SVG strings, extract a small `src/icons.js` module that exports the icon map, and import it. This keeps `main.js` focused on rendering logic.

---

## 10. Testing

UI-only; no unit tests added (consistent with the project's existing approach — frontend isn't unit-tested).

**Manual verification** (add to `docs/smoke-checklist.md` under a new "UI polish" section):
- Each state (downloading/seeding/completed/paused/stalled) shows the correct dot color and tinted progress.
- File-type icons render for video/audio/image/document/archive/program/other.
- Empty state (all filter, zero) shows the three hint cards; a filter with zero matches shows the lighter variant.
- Sidebar filter icons + live counts + totals card update as torrents change.
- Settings toggles flip coral/gray and persist after Save + reopen.
- Add dialog file rows show icons; long save paths still front-truncate.
- Hover and progress transitions are smooth; no flicker on the 1 Hz progress updates.
- ETA shows a sensible value while downloading and `—` when paused/stalled.

---

## 11. Out of scope (possible later)

- Dark mode (palette already designed around a warm-charcoal counterpart in the original spec).
- Per-torrent detail pane / inspector.
- Drag-to-reorder, multi-select, bulk actions.
- Animated empty-state illustration.
