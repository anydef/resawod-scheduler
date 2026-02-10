use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};

/// Returns the next occurrence of the given weekday strictly after `from`.
/// If `from` is already that weekday, it returns the *next* week's occurrence.
pub fn next_weekday(from: NaiveDate, target: Weekday) -> NaiveDate {
    let from_weekday = from.weekday().num_days_from_monday();
    let target_weekday = target.num_days_from_monday();
    let days_ahead = if target_weekday > from_weekday {
        target_weekday - from_weekday
    } else {
        7 - (from_weekday - target_weekday)
    };
    from + Duration::days(days_ahead as i64)
}

/// Parse a day name (e.g. "monday") into a chrono Weekday.
pub fn parse_weekday(day: &str) -> Option<Weekday> {
    match day.to_lowercase().as_str() {
        "monday" => Some(Weekday::Mon),
        "tuesday" => Some(Weekday::Tue),
        "wednesday" => Some(Weekday::Wed),
        "thursday" => Some(Weekday::Thu),
        "friday" => Some(Weekday::Fri),
        "saturday" => Some(Weekday::Sat),
        "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Compute start and end UNIX timestamps for a given date.
/// Start = 00:00:00, End = 22:00:00 on the given date.
pub fn day_timestamps(date: NaiveDate) -> (i64, i64) {
    let start = date
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .and_utc()
        .timestamp();
    let end = date
        .and_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap())
        .and_utc()
        .timestamp();
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_next_weekday_same_day() {
        // If today is Wednesday, next Wednesday should be 7 days later
        let wed = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(); // Wednesday
        let next = next_weekday(wed, Weekday::Wed);
        assert_eq!(next, NaiveDate::from_ymd_opt(2024, 1, 10).unwrap());
    }

    #[test]
    fn test_next_weekday_future_day() {
        let mon = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Monday
        let next = next_weekday(mon, Weekday::Fri);
        assert_eq!(next, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
    }

    #[test]
    fn test_next_weekday_past_day() {
        let fri = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap(); // Friday
        let next = next_weekday(fri, Weekday::Mon);
        assert_eq!(next, NaiveDate::from_ymd_opt(2024, 1, 8).unwrap());
    }

    #[test]
    fn test_parse_weekday() {
        assert_eq!(parse_weekday("monday"), Some(Weekday::Mon));
        assert_eq!(parse_weekday("FRIDAY"), Some(Weekday::Fri));
        assert_eq!(parse_weekday("invalid"), None);
    }

    #[test]
    fn test_day_timestamps() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let (start, end) = day_timestamps(date);
        assert_eq!(end - start, 22 * 3600);
    }
}
