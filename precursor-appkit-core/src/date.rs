//! Proleptic Gregorian calendar dates in pure integer math.
//!
//! Conversions to and from a day count use Howard Hinnant's `days_from_civil`
//! / `civil_from_days` algorithms, which are exact for every year in range.
//!
//! Shared by planner and budget. Enable this crate's `serde` feature to store
//! [`Date`] in records. Get today's date on the device from `llio`'s local
//! time (see planner `src/app.rs`), falling back to the last date shown when
//! the clock isn't set.

use core::fmt;


/// Earliest and latest years accepted (keeps all arithmetic in range
/// and keeps four-digit display).
pub const MIN_YEAR: i32 = 1970;
pub const MAX_YEAR: i32 = 9999;

const MS_PER_DAY: u64 = 86_400_000;

const MONTH_NAMES: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

pub fn is_leap(year: i32) -> bool { (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 }

/// Days in `month` (1..=12) of `year`; 0 for an invalid month.
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Short English month name for 1..=12, `"???"` otherwise.
pub fn month_name(month: u8) -> &'static str {
    MONTH_NAMES.get((month as usize).wrapping_sub(1)).copied().unwrap_or("???")
}

/// A calendar month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    pub year: i32,
    pub month: u8,
}

impl YearMonth {
    pub fn new(year: i32, month: u8) -> Option<Self> {
        ((MIN_YEAR..=MAX_YEAR).contains(&year) && (1..=12).contains(&month)).then_some(Self { year, month })
    }

    /// Next month, rolling December into January of the next year. Saturates at `MAX_YEAR`-12.
    pub fn next(self) -> Self {
        match self.month {
            12 if self.year < MAX_YEAR => Self { year: self.year + 1, month: 1 },
            12 => self,
            m => Self { year: self.year, month: m + 1 },
        }
    }

    /// Previous month, rolling January into December of the previous year. Saturates at `MIN_YEAR`-01.
    pub fn prev(self) -> Self {
        match self.month {
            1 if self.year > MIN_YEAR => Self { year: self.year - 1, month: 12 },
            1 => self,
            m => Self { year: self.year, month: m - 1 },
        }
    }

    pub fn first_day(self) -> Date { Date { year: self.year, month: self.month, day: 1 } }

    pub fn days(self) -> u8 { days_in_month(self.year, self.month) }

    pub fn contains(self, d: Date) -> bool { d.year == self.year && d.month == self.month }
}

impl fmt::Display for YearMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{} {}", month_name(self.month), self.year) }
}

/// A calendar date. Field order makes the derived `Ord` chronological.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    /// A validated date, or `None` if it doesn't exist (e.g. 2025-02-29).
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        let ym = YearMonth::new(year, month)?;
        (day >= 1 && day <= ym.days()).then_some(Self { year, month, day })
    }

    pub fn is_valid(&self) -> bool { Date::new(self.year, self.month, self.day).is_some() }

    pub fn year_month(&self) -> YearMonth { YearMonth { year: self.year, month: self.month } }

    /// Days since 1970-01-01 (negative before).
    pub fn to_days(&self) -> i64 {
        let y = self.year as i64 - if self.month <= 2 { 1 } else { 0 };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = self.month as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.day as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` after 1970-01-01, clamped to the supported year range.
    pub fn from_days(days: i64) -> Self {
        let lo = Date { year: MIN_YEAR, month: 1, day: 1 };
        let hi = Date { year: MAX_YEAR, month: 12, day: 31 };
        if days <= lo.to_days() {
            return lo;
        }
        if days >= hi.to_days() {
            return hi;
        }
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let year = (yoe + era * 400 + if month <= 2 { 1 } else { 0 }) as i32;
        Date { year, month, day }
    }

    /// Local date from milliseconds since the epoch (as `llio::LocalTime` reports it).
    pub fn from_epoch_ms(ms: u64) -> Self { Date::from_days((ms / MS_PER_DAY) as i64) }

    pub fn add_days(&self, n: i64) -> Self { Date::from_days(self.to_days() + n) }

    /// Day of week, 0 = Monday … 6 = Sunday (ISO 8601).
    pub fn weekday(&self) -> u8 { (self.to_days() + 3).rem_euclid(7) as u8 }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Two-letter weekday names, Monday first (index = [`Date::weekday`]).
pub const WEEKDAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// Three-letter weekday name for a date.
pub fn weekday_name(d: Date) -> &'static str {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][d.weekday() as usize]
}

/// Calendar grid for a month: 6 weeks x 7 days, Monday first; `None`
/// outside the month.
pub fn month_grid(ym: YearMonth) -> [[Option<u8>; 7]; 6] {
    let mut grid = [[None; 7]; 6];
    let offset = ym.first_day().weekday() as usize;
    for day in 1..=ym.days() {
        let cell = offset + day as usize - 1;
        grid[cell / 7][cell % 7] = Some(day);
    }
    grid
}

impl Date {
    /// The same day in the month `delta` months away, clamped to that
    /// month's length (Jan 31 + 1 month = Feb 28/29).
    pub fn add_months(&self, delta: i32) -> Self {
        let mut ym = self.year_month();
        for _ in 0..delta.unsigned_abs() {
            ym = if delta > 0 { ym.next() } else { ym.prev() };
        }
        Date { year: ym.year, month: ym.month, day: self.day.min(ym.days()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u8, day: u8) -> Date { Date::new(y, m, day).unwrap() }

    #[test]
    fn leap_years() {
        assert!(is_leap(2024));
        assert!(is_leap(2000));
        assert!(!is_leap(1900));
        assert!(!is_leap(2100));
        assert!(!is_leap(2026));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
    }

    #[test]
    fn month_lengths() {
        let lens: Vec<u8> = (1..=12).map(|m| days_in_month(2026, m)).collect();
        assert_eq!(lens, vec![31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]);
        assert_eq!(days_in_month(2026, 0), 0);
        assert_eq!(days_in_month(2026, 13), 0);
    }

    #[test]
    fn invalid_dates_rejected() {
        assert!(Date::new(2025, 2, 29).is_none());
        assert!(Date::new(2024, 2, 29).is_some());
        assert!(Date::new(2026, 4, 31).is_none());
        assert!(Date::new(2026, 13, 1).is_none());
        assert!(Date::new(2026, 1, 0).is_none());
        assert!(Date::new(1969, 12, 31).is_none());
    }

    #[test]
    fn known_day_counts() {
        assert_eq!(d(1970, 1, 1).to_days(), 0);
        assert_eq!(d(2000, 3, 1).to_days(), 11_017);
        assert_eq!(d(2026, 11, 20).to_days(), 20_777);
        assert_eq!(Date::from_days(20_777), d(2026, 11, 20));
    }

    #[test]
    fn round_trip_every_day_1970_to_2100() {
        let mut prev = d(1970, 1, 1);
        for n in 1..=d(2100, 12, 31).to_days() {
            let x = Date::from_days(n);
            assert!(x.is_valid(), "{x}");
            assert_eq!(x.to_days(), n);
            assert!(x > prev);
            prev = x;
        }
    }

    #[test]
    fn epoch_ms() {
        // 2026-11-20T23:59:59.999 local
        assert_eq!(Date::from_epoch_ms(20_777 * 86_400_000 + 86_399_999), d(2026, 11, 20));
        assert_eq!(Date::from_epoch_ms(0), d(1970, 1, 1));
    }

    #[test]
    fn day_steps_cross_month_and_year() {
        assert_eq!(d(2024, 2, 28).add_days(1), d(2024, 2, 29));
        assert_eq!(d(2024, 2, 29).add_days(1), d(2024, 3, 1));
        assert_eq!(d(2025, 2, 28).add_days(1), d(2025, 3, 1));
        assert_eq!(d(2026, 12, 31).add_days(1), d(2027, 1, 1));
        assert_eq!(d(2027, 1, 1).add_days(-1), d(2026, 12, 31));
        assert_eq!(d(2026, 3, 1).add_days(-1), d(2026, 2, 28));
    }

    #[test]
    fn weekdays_of_known_dates() {
        assert_eq!(d(1970, 1, 1).weekday(), 3); // Thursday
        assert_eq!(d(2000, 1, 1).weekday(), 5); // Saturday
        assert_eq!(d(2024, 2, 29).weekday(), 3); // Thursday
        assert_eq!(d(2026, 11, 20).weekday(), 4); // Friday
        assert_eq!(d(2001, 9, 11).weekday(), 1); // Tuesday
    }

    #[test]
    fn month_rollover() {
        let dec = YearMonth::new(2026, 12).unwrap();
        assert_eq!(dec.next(), YearMonth::new(2027, 1).unwrap());
        assert_eq!(dec.next().prev(), dec);
        assert_eq!(YearMonth::new(2026, 1).unwrap().prev(), YearMonth::new(2025, 12).unwrap());
        let floor = YearMonth::new(MIN_YEAR, 1).unwrap();
        assert_eq!(floor.prev(), floor);
        let ceil = YearMonth::new(MAX_YEAR, 12).unwrap();
        assert_eq!(ceil.next(), ceil);
    }

    #[test]
    fn display() {
        assert_eq!(d(2026, 3, 7).to_string(), "2026-03-07");
        assert_eq!(YearMonth::new(2026, 11).unwrap().to_string(), "Nov 2026");
        assert_eq!(month_name(0), "???");
    }

    #[test]
    fn from_days_clamps() {
        assert_eq!(Date::from_days(-5), d(1970, 1, 1));
        assert_eq!(Date::from_days(i64::MAX / 2), d(9999, 12, 31));
    }

    #[test]
    fn grids() {
        // November 2026 starts on a Sunday: day 1 is the last cell of week 1.
        let g = month_grid(YearMonth::new(2026, 11).unwrap());
        assert_eq!(g[0], [None, None, None, None, None, None, Some(1)]);
        assert_eq!(g[1][0], Some(2));
        assert_eq!(g[5][0], Some(30));
        assert_eq!(g[5][1], None);
        // February 2027 starts on Monday and fills exactly four rows.
        let g = month_grid(YearMonth::new(2027, 2).unwrap());
        assert_eq!(g[0][0], Some(1));
        assert_eq!(g[3][6], Some(28));
        assert!(g[4].iter().all(|c| c.is_none()));
        // Every day appears exactly once, in weekday order.
        for (y, m) in [(2024, 2), (2026, 8), (2027, 5)] {
            let ym = YearMonth::new(y, m).unwrap();
            let g = month_grid(ym);
            for day in 1..=ym.days() {
                let pos: Vec<usize> = (0..42).filter(|&i| g[i / 7][i % 7] == Some(day)).collect();
                assert_eq!(pos.len(), 1);
                assert_eq!(pos[0] % 7, Date::new(y, m, day).unwrap().weekday() as usize);
            }
        }
    }

    #[test]
    fn month_steps_clamp_day() {
        assert_eq!(d(2026, 1, 31).add_months(1), d(2026, 2, 28));
        assert_eq!(d(2024, 1, 31).add_months(1), d(2024, 2, 29));
        assert_eq!(d(2026, 3, 31).add_months(-1), d(2026, 2, 28));
        assert_eq!(d(2026, 12, 15).add_months(1), d(2027, 1, 15));
        assert_eq!(d(2026, 1, 15).add_months(-1), d(2025, 12, 15));
        assert_eq!(d(2026, 5, 10).add_months(0), d(2026, 5, 10));
    }

    #[test]
    fn weekday_names() {
        assert_eq!(weekday_name(d(2026, 11, 20)), "Fri");
        assert_eq!(weekday_name(d(2000, 1, 1)), "Sat");
    }
}
