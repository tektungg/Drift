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
        assert!(plan.to_start.is_empty());
        assert_eq!(plan.to_pause, vec!["c"]);
    }

    #[test]
    fn forced_bypasses_cap() {
        let items = vec![
            item("a", Desired::Run, true,  5, false, false),
            item("b", Desired::Run, false, 0, false, false),
            item("c", Desired::Run, false, 1, false, false),
        ];
        let plan = decide(&items, 1);
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
            item("seed", Desired::Run,  false, 0, true,  true),
            item("paus", Desired::Pause, false, 1, false, false),
            item("dl",   Desired::Run,  false, 2, false, false),
        ];
        let plan = decide(&items, 1);
        assert_eq!(plan.to_start, vec!["dl"]);
        assert!(plan.to_pause.is_empty());
    }

    #[test]
    fn idempotent_when_already_correct() {
        let items = vec![
            item("a", Desired::Run, false, 0, false, true),
            item("b", Desired::Run, false, 1, false, false),
        ];
        let plan = decide(&items, 1);
        assert!(plan.to_start.is_empty());
        assert!(plan.to_pause.is_empty());
    }
}
