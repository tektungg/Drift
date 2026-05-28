# Drift — Library Management & Control (v0.4.0)

**Status:** Design approved, ready for planning
**Date:** 2026-05-28
**Author:** brainstormed with Claude

## Goal

Make Drift comfortable to use as the torrent list grows. Today the list is a
flat, unsorted, unsearchable column and every action is one-torrent-at-a-time
with a single global speed cap. This phase adds the ergonomics and control a
daily-driver client needs: finding torrents, acting on many at once, capping how
many download concurrently, and (conditionally) limiting an individual torrent.

Four features, delivered as one cohesive spec but **sequenced** in the plan so
the high-value/low-risk work ships first and the one uncertain feature lands
last and can be dropped without affecting the rest:

1. Search & sort the list
2. Multi-select + bulk actions
3. Queue management (max active + force-start + reorder)
4. Per-torrent speed limits — **gated** on a feasibility spike

Non-goals: RSS/auto-download, built-in search indexers, media streaming,
scheduling, per-category save paths. Those are separate future phases.

---

## Feature 1 — Search & Sort

**Where:** the main header row (currently "All downloads" / "+ Add torrent")
gains a search input and a sort control.

**Search:**
- Case-insensitive substring match on the torrent display name.
- Filters live as the user types; empty query shows everything.
- Composes with the existing sidebar state filter (All / Downloading / Seeding /
  Completed / Paused / Queued) — the visible set is `stateFilter ∧ searchMatch`.

**Sort:**
- Keys: **Date added** (default, newest first), **Name**, **Progress**,
  **Download speed**, **Size (total)**.
- Clicking the active key toggles ascending/descending; a direction caret shows
  which way.
- The chosen key + direction persist across launches in `localStorage`
  (mirrors how the theme is remembered), defaulting to Date-added-desc.

**Rendering interaction (important):** the sort comparator and search filter are
applied only in the *structural* re-render path (`renderList`), **not** in the
1 Hz surgical `patchRow` path introduced in 0.3.0. This means rows do not jump
around while you watch live progress; reordering happens on add/remove, state
change, or when the user changes the sort/search — which is the desired feel.

**Engine impact:** none. Pure frontend (`main.js` + `styles.css`).

---

## Feature 2 — Multi-select & bulk actions

**Selection model (Windows-Explorer style):**
- Plain click on a row header still toggles expand (unchanged).
- **Ctrl-click** toggles a row in/out of the selection.
- **Shift-click** selects the contiguous range from the last-clicked row.
- Selection is tracked in a frontend `Set<infohash>`; selected rows get a
  distinct highlight (using the accent-soft token, clearly different from hover).
- Selection clears on Escape, on a plain click elsewhere, or when the action
  completes.

**Bulk action bar:** when ≥1 torrent is selected, a slim bar appears (anchored
in the main header area or as a floating bar above the list) showing:
- "N selected"
- **Pause** · **Resume** · **Remove** · a clear/deselect control.

**Bulk semantics:**
- Pause / Resume / Remove iterate the selection and call the existing
  `pause` / `resume` / `remove` Tauri commands per torrent.
- Bulk Remove reuses the existing remove confirmation, asking once for the whole
  batch, including the "also delete downloaded files" choice.
- Actions that don't apply to a given torrent (e.g. Resume on an already-running
  one) are no-ops, not errors.

**Engine impact:** none new — reuses existing commands. Frontend selection state
+ action bar + a thin "apply to many" loop.

---

## Feature 3 — Queue management

Drift owns the queue logic; librqbit has no queue concept, so we build it on top
of the existing per-torrent `pause`/`unpause`.

**Setting:** *Max active downloads* — integer, default **3**, `0` = unlimited.
Added to the Settings panel (new "Queue" group). Persisted in `config.json`.

**What counts against the cap:** only torrents actively *downloading*. Seeding,
completed, paused, and errored torrents do **not** consume a download slot — you
can seed many while N download.

**New state `Queued`:** a torrent that is eligible to download but waiting for a
free slot. Under the hood it is paused at the engine level; in our state it is
tagged `Queued`, distinct from a user-initiated `Paused`. This distinction is
what lets auto-rotation start a queued torrent but never resurrect one the user
explicitly paused.

**Per-torrent control (right-click menu):**
- **Resume** — clear user-pause; torrent becomes eligible (subject to the cap →
  it may immediately sit `Queued` rather than run).
- **Force start** — run now, **bypassing** the cap. Sets a sticky `forced` flag;
  forced torrents run regardless of how many others are active and do not get
  auto-paused by the controller.
- **Pause** — sticky user pause; the controller never auto-starts it.
- **Reorder:** *Move to top* / *Move up* / *Move down* / *Move to bottom*.
  Priority is an explicit `queue_position` (lower = higher priority); new
  torrents append to the end.

**Queue controller:** a new module (`queue.rs`) with a **pure** decision core:

```
decide(torrents: &[QueueItem], max_active: u32) -> QueuePlan
// QueueItem: { infohash, desired: Running|Paused, forced, position, is_downloading_now }
// QueuePlan: { to_start: Vec<infohash>, to_pause: Vec<infohash> }
```

The pure function takes the current torrent set + cap and returns which torrents
should be started and which should be paused to honor the cap and priority. A
thin async wrapper applies the plan via the engine and is invoked on the events
that can change the active count: torrent added, a download finishes
(→ seeding/completed), user pause/resume/force, max-active changed, and on
startup (resume). Keeping the core pure makes it unit-testable without the engine
or a running session.

**Persistence:** `queue_position`, the paused-vs-queued distinction, and the
`forced` flag are stored in `state.json` so the queue survives restarts. On
launch, the resume flow hands the persisted set to the controller, which decides
what actually starts (respecting the cap) instead of blindly unpausing all.

**Sidebar:** add a **Queued** count + filter alongside the existing states.

---

## Feature 4 — Per-torrent speed limits (GATED)

**Conditional on a feasibility spike — first task in this feature's plan.**

The session already exposes `session.ratelimits.set_download_bps/upload_bps`
(global). Per-torrent limiting is the unknown: verify whether librqbit exposes a
per-`ManagedTorrent` rate limiter (or per-torrent options) in the pinned version.

- **If supported:** add `dl_limit`/`ul_limit` (KB/s, 0 = unlimited) to the
  torrent record; a *Set speed limit…* entry in the right-click menu opens a
  small dialog; the active limit is shown in the expanded-row detail line.
  Applied on set and re-applied on resume/restart.
- **If NOT supported:** **drop the feature.** Do not hand-roll a throttle. The
  global cap already covers the common single-user case. Record the finding in
  the plan and move on. This is an explicit, pre-agreed exit.

---

## Cross-cutting changes

**State model (`state.rs`):**
- Add `Queued` to the `TorrentState` enum.
- Add fields to the persisted torrent record: `queue_position: u32`,
  `forced: bool`, `dl_limit: u32`, `ul_limit: u32` (all behind `#[serde(default)]`
  so existing `state.json` files load unchanged).
- Update the progress→state persistence in `main.rs` so a slot-waiting torrent
  is recorded as `Queued`, a user-paused one as `Paused`, and a forced one keeps
  running — i.e. the engine's "is_paused" alone no longer decides the label.

**Settings (`settings.rs`):** add `max_active_downloads: u32` (default 3) with
`#[serde(default)]`; surfaced in a new "Queue" group in the Settings panel.

**Commands (`commands.rs`):** new/extended Tauri commands for force-start, queue
reorder, and (if gated feature ships) set-per-torrent-limit. Bulk actions reuse
existing commands from the frontend.

**Frontend (`main.js`, `styles.css`):** header search + sort control, selection
model + bulk bar, queued state styling + sidebar entry, reorder/force entries in
the context menu, optional per-torrent-limit dialog.

**Version:** target **0.4.0**. No build/publish until explicitly requested.

---

## Testing

- **Unit (Rust):** the `queue.rs` pure `decide()` core — cap honored, priority
  order respected, forced torrents bypass the cap, user-paused never auto-start,
  seeding doesn't consume slots, `0` = unlimited.
- **Unit (frontend):** sort comparator (each key + direction) and search filter
  (case-insensitive substring, compose with state filter) as pure functions.
- **Manual smoke:** ctrl/shift selection + bulk pause/resume/remove; add >cap
  torrents and watch auto-rotation; force-start bypasses cap; reorder changes who
  runs next; queue survives a restart; (if applicable) per-torrent limit takes
  effect.

## Sequencing (for the plan)

1. Search & sort (frontend only) — ships first, immediate value, zero risk.
2. Multi-select + bulk actions (frontend) — builds on the same list UI.
3. Queue management (state + `queue.rs` + settings + sidebar + context menu).
4. Per-torrent limits — feasibility spike first, then implement or drop.

## Risks

- **Per-torrent rate limiting** may not exist in the pinned librqbit — mitigated
  by the gate (drop, don't build a throttle).
- **State-label logic** in `main.rs` grows more conditional with `Queued`/forced;
  keep the decision in one place (ideally derived from the same model the queue
  controller uses) to avoid divergence between sidebar counts and actual behavior.
- **Queue + resume-on-launch interaction:** must route startup through the
  controller rather than the current "unpause everything not user-paused" loop,
  or the cap is ignored on the first run after restart.
