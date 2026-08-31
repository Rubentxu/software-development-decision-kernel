//! Date/time formatting utilities.
//!
//! Format module for RFC 3339 timestamps across the workspace. Centralized here
//! so that `sddk-storage`, `sddk-cli`, and any future crate share a single
//! implementation of Howard Hinnant's `civil_from_days` algorithm.
//!
//! ## Why no `chrono`?
//!
//! The kernel wants zero external dependencies for date math. The Hinnant
//! algorithm is ~30 lines, deterministic, and produces the same RFC 3339 output
//! as `chrono::Utc::now().to_rfc3339()` for any properly-encoded Unix epoch.
//!
//! ## Verification
//!
//! Pinned-value tests in `sddk-storage::graph_store::tests` exercise:
//! - Unix epoch: `0` → `"1970-01-01T00:00:00Z"`
//! - Mid-cycle: `1_787_142_896` → `"2026-08-19T12:34:56Z"`
//! - Leap year: `1_709_164_800` → `"2024-02-29T00:00:00Z"`

/// Format `epoch_secs` (Unix seconds, UTC) as RFC 3339: `YYYY-MM-DDTHH:MM:SSZ`.
///
/// Howard Hinnant's algorithm (https://howardhinnant.github.io/date_algorithms.html).
/// No external dependencies. Use this instead of `chrono::Utc::now().to_rfc3339()`
/// or `format!("1970-01-01T00:00:00+00:00 (epoch {})")` (the cycle-2 bug).
///
/// # Examples
///
/// ```
/// use sddk_domain::format::format_rfc3339_utc;
/// assert_eq!(format_rfc3339_utc(0), "1970-01-01T00:00:00Z");
/// ```
pub fn format_rfc3339_utc(epoch_secs: u64) -> String {
    // Days since 1970-01-01 (Unix epoch)
    let z = (epoch_secs / 86_400) as i64;
    let secs_of_day = epoch_secs % 86_400;

    // Hinnant's `civil_from_days(z)` shifts `z` by 719468 to align day 0
    // with 0000-03-01 (so the year boundary falls inside the era).
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };

    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hour, minute, second
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zeros() {
        assert_eq!(format_rfc3339_utc(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn mid_cycle_2026() {
        // 2026-08-19T12:34:56Z: 20_454 days (1970-2026) + 230 days (Aug 19)
        // = 20_684 days × 86_400 + 45_296 s = 1_787_142_896
        assert_eq!(format_rfc3339_utc(1_787_142_896), "2026-08-19T12:34:56Z");
    }

    #[test]
    fn leap_year_feb_29() {
        // 2024-02-29T00:00:00Z
        // 1970-01-01..2024-01-01 = 54 yrs × 365 + 13 leap days = 19_723 days
        // 2024-01-01..2024-02-29 = 59 days
        // 19_723 + 59 = 19_782 days × 86_400 = 1_709_164_800
        assert_eq!(format_rfc3339_utc(1_709_164_800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn one_second_after_epoch() {
        assert_eq!(format_rfc3339_utc(1), "1970-01-01T00:00:01Z");
    }

    #[test]
    fn day_boundary() {
        // 1970-01-02T00:00:00Z = 86_400 seconds
        assert_eq!(format_rfc3339_utc(86_400), "1970-01-02T00:00:00Z");
    }
}
