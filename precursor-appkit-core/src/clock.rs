//! "How much real time has passed?" from two imperfect clocks.
//!
//! * The wall clock (RTC via `llio`) keeps counting through suspend and
//!   reboots, but may be unset and can be changed by the user.
//! * The monotonic ticktimer (`Ticktimer::elapsed_ms`) never jumps, but stops
//!   while the device is suspended and restarts at boot.
//!
//! Policy: use the wall clock when it is set and hasn't gone backwards;
//! otherwise the monotonic delta. Sub-second remainders are carried, so
//! calling often (e.g. from a 250 ms pump) still adds up — the bug in the
//! first pet version, which truncated each call to whole seconds and so never
//! advanced at all when the wall clock was unset.
//!
//! Shared by pet and timers-style apps. Callers cap catch-up themselves
//! (e.g. pet caps at 14 days).

/// Tracks elapsed time across calls. Persist [`Timekeeper::last_wall_s`] to
/// count time while the app wasn't running.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Timekeeper {
    last_wall_s: Option<u64>,
    last_mono_ms: Option<u64>,
    carry_ms: u64,
}

impl Timekeeper {
    /// Resume from a saved wall-clock reading (`None` = never saved or unknown).
    pub fn from_saved(last_wall_s: Option<u64>) -> Self { Self { last_wall_s, ..Self::default() } }

    /// Wall reading to store with a snapshot.
    pub fn last_wall_s(&self) -> Option<u64> { self.last_wall_s }

    /// Milliseconds elapsed since the previous call. `wall_s`: wall clock in
    /// seconds if set; `mono_ms`: ticktimer reading.
    pub fn elapsed_ms(&mut self, wall_s: Option<u64>, mono_ms: u64) -> u64 {
        let mono_delta = self.last_mono_ms.map(|m| mono_ms.saturating_sub(m));
        let e = match (wall_s, self.last_wall_s) {
            (Some(w), Some(l)) if w >= l => {
                let wall_ms = (w - l) * 1000;
                // The wall clock only has 1 s resolution: when it hasn't
                // ticked, the monotonic delta is the better short-term measure.
                // Over long gaps (suspend), wall time wins.
                match mono_delta {
                    Some(m) if wall_ms <= m + 1000 && m <= wall_ms + 1000 => m,
                    _ => wall_ms,
                }
            }
            // Clock set back, or first wall reading: trust the monotonic delta.
            _ => mono_delta.unwrap_or(0),
        };
        // Baselines always move to the latest readings; an unset wall clock
        // forgets its old reading so its return can't double-count time.
        self.last_wall_s = wall_s;
        self.last_mono_ms = Some(mono_ms);
        e
    }

    /// Whole seconds elapsed since the previous call; the remainder carries
    /// to the next call.
    pub fn elapsed_secs(&mut self, wall_s: Option<u64>, mono_ms: u64) -> u64 {
        let total = self.carry_ms + self.elapsed_ms(wall_s, mono_ms);
        self.carry_ms = total % 1000;
        total / 1000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: frequent calls with the wall clock unset must add up.
    #[test]
    fn frequent_calls_without_wall_clock_accumulate() {
        let mut t = Timekeeper::default();
        let mut secs = 0;
        for i in 0..=40u64 {
            secs += t.elapsed_secs(None, i * 250);
        }
        assert_eq!(secs, 10);
    }

    #[test]
    fn frequent_calls_with_wall_clock_accumulate_without_double_counting() {
        let mut t = Timekeeper::from_saved(Some(1000));
        let mut secs = t.elapsed_secs(Some(1000), 0);
        for i in 1..=40u64 {
            let mono = i * 250;
            secs += t.elapsed_secs(Some(1000 + mono / 1000), mono);
        }
        assert_eq!(secs, 10);
    }

    #[test]
    fn suspend_counts_via_wall_clock() {
        let mut t = Timekeeper::from_saved(Some(100));
        t.elapsed_secs(Some(100), 5_000);
        // Suspended for an hour: ticktimer advanced 1 s, wall 3601 s.
        assert_eq!(t.elapsed_secs(Some(3701), 6_000), 3601);
    }

    #[test]
    fn time_away_from_saved_reading() {
        let mut t = Timekeeper::from_saved(Some(1_000));
        // App starts after a reboot: no monotonic baseline yet.
        assert_eq!(t.elapsed_secs(Some(87_400), 10), 86_400);
    }

    #[test]
    fn clock_set_backwards_uses_monotonic() {
        let mut t = Timekeeper::from_saved(Some(5_000));
        t.elapsed_secs(Some(5_000), 0);
        assert_eq!(t.elapsed_secs(Some(10), 3_000), 3);
        assert_eq!(t.last_wall_s(), Some(10));
    }

    #[test]
    fn unset_then_set_does_not_double_count() {
        let mut t = Timekeeper::from_saved(Some(100));
        t.elapsed_secs(Some(100), 0);
        assert_eq!(t.elapsed_secs(None, 2_000), 2);
        // Wall clock comes back: baseline only, no jump from the old reading.
        assert_eq!(t.elapsed_secs(Some(50_000), 3_000), 1);
    }
}
