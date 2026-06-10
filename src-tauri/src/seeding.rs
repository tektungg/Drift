//! Pure decision logic for automatic seeding limits. The 1 Hz progress task in
//! `main.rs` feeds it facts each tick; if it says stop, the torrent is
//! engine-paused and persisted as Completed.

/// Facts about a currently-seeding torrent, as of `now_ms`.
#[derive(Debug, Clone)]
pub struct SeedFacts {
    pub uploaded: u64,
    pub total: u64,
    /// When the torrent first finished downloading (ms epoch). `None` for
    /// torrents finished before 0.6.0 — their clock starts when first observed.
    pub completed_at: Option<i64>,
    pub now_ms: i64,
    /// Force-started torrents are exempt — "seed anyway" escape hatch.
    pub forced: bool,
}

/// True if the torrent has met either global seeding limit.
/// `ratio_limit == 0.0` and `time_limit_mins == 0` each mean "unlimited".
/// Ratio uses `total` (torrent size) as the denominator, not downloaded bytes,
/// so pre-existing data can't divide by zero or inflate the ratio.
pub fn should_stop(facts: &SeedFacts, ratio_limit: f64, time_limit_mins: u32) -> bool {
    if facts.forced {
        return false;
    }
    if ratio_limit > 0.0 && facts.total > 0 {
        let ratio = facts.uploaded as f64 / facts.total as f64;
        if ratio >= ratio_limit {
            return true;
        }
    }
    if time_limit_mins > 0 {
        if let Some(t) = facts.completed_at {
            if facts.now_ms - t >= time_limit_mins as i64 * 60_000 {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(uploaded: u64, total: u64, completed_at: Option<i64>, now_ms: i64, forced: bool) -> SeedFacts {
        SeedFacts { uploaded, total, completed_at, now_ms, forced }
    }

    #[test]
    fn unlimited_never_stops() {
        assert!(!should_stop(&facts(u64::MAX, 1, Some(0), i64::MAX, false), 0.0, 0));
    }

    #[test]
    fn ratio_at_limit_stops() {
        assert!(should_stop(&facts(200, 100, None, 0, false), 2.0, 0));
    }

    #[test]
    fn ratio_below_limit_keeps_seeding() {
        assert!(!should_stop(&facts(199, 100, None, 0, false), 2.0, 0));
    }

    #[test]
    fn ratio_above_limit_stops() {
        assert!(should_stop(&facts(201, 100, None, 0, false), 2.0, 0));
    }

    #[test]
    fn zero_total_never_trips_ratio() {
        assert!(!should_stop(&facts(500, 0, None, 0, false), 2.0, 0));
    }

    #[test]
    fn time_at_limit_stops() {
        // 30 min limit, completed exactly 30 min ago.
        assert!(should_stop(&facts(0, 100, Some(0), 30 * 60_000, false), 0.0, 30));
    }

    #[test]
    fn time_below_limit_keeps_seeding() {
        assert!(!should_stop(&facts(0, 100, Some(0), 30 * 60_000 - 1, false), 0.0, 30));
    }

    #[test]
    fn missing_completed_at_never_trips_time() {
        assert!(!should_stop(&facts(0, 100, None, i64::MAX, false), 0.0, 1));
    }

    #[test]
    fn forced_bypasses_both_limits() {
        assert!(!should_stop(&facts(u64::MAX, 1, Some(0), i64::MAX, true), 0.1, 1));
    }

    #[test]
    fn either_limit_suffices() {
        // Ratio not met, time met.
        assert!(should_stop(&facts(0, 100, Some(0), 60 * 60_000, false), 5.0, 30));
        // Time not met, ratio met.
        assert!(should_stop(&facts(500, 100, Some(0), 1, false), 2.0, 999));
    }
}
