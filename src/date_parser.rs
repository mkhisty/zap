use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use regex::Regex;

use crate::todo::Priority;

/// Parse priority markers from input text.
/// Returns (remaining text, priority) after removing [priority:LEVEL] or [p:LEVEL] markers.
/// Levels: low, medium, high, max (case-insensitive)
pub fn parse_priority(input: &str) -> (String, Priority) {
    let text = input.to_string();
    let lower = text.to_lowercase();

    // Check for [priority:LEVEL] or [p:LEVEL] patterns (case insensitive)
    let patterns = [
        // Full form
        ("[priority:max]", Priority::Max),
        ("[priority:high]", Priority::High),
        ("[priority:medium]", Priority::Medium),
        ("[priority:low]", Priority::Low),
        // Short form
        ("[p:max]", Priority::Max),
        ("[p:high]", Priority::High),
        ("[p:medium]", Priority::Medium),
        ("[p:low]", Priority::Low),
    ];

    for (marker, p) in patterns {
        if let Some(pos) = lower.find(marker) {
            // Remove the marker from text (use original case positions)
            let before = &text[..pos];
            let after = &text[pos + marker.len()..];
            let result = format!("{}{}", before, after);
            let result = result.split_whitespace().collect::<Vec<_>>().join(" ");
            return (result, p);
        }
    }

    (text, Priority::None)
}

/// Parse date from input text using [date:...] or [d:...] syntax.
/// Returns (remaining text, parsed date) if a date pattern is found.
/// Supported formats inside brackets:
/// - today, tod, tomorrow, tom, yesterday
/// - weekday names (mon, monday, tue, etc.)
/// - next <weekday>
/// - month day (jan 15, january 15)
/// - relative (+3, 3d)
/// - mm/dd, mm/dd/yy, mm/dd/yyyy
pub fn parse_date(input: &str) -> (String, Option<NaiveDate>) {
    let input = input.trim();
    let today = Local::now().date_naive();

    // Match [date:...] or [d:...] pattern (case insensitive)
    let re = Regex::new(r"(?i)\[(date|d):([^\]]+)\]").unwrap();

    if let Some(caps) = re.captures(input) {
        let full_match = caps.get(0).unwrap();
        let date_str = caps.get(2).unwrap().as_str().trim().to_lowercase();

        if let Some(date) = try_parse_date(&date_str, today) {
            // Remove the marker from text
            let before = &input[..full_match.start()];
            let after = &input[full_match.end()..];
            let result = format!("{}{}", before, after);
            let result = result.split_whitespace().collect::<Vec<_>>().join(" ");
            return (result, Some(date));
        }
    }

    (input.to_string(), None)
}

fn try_parse_date(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    match s {
        "today" | "tod" => Some(today),
        "tomorrow" | "tom" => Some(today + Duration::days(1)),
        "yesterday" => Some(today - Duration::days(1)),
        _ => {
            // Try "next <weekday>"
            if let Some(rest) = s.strip_prefix("next ") {
                if let Some(weekday) = parse_weekday(rest) {
                    return Some(next_weekday(today, weekday, true));
                }
            }

            // Try weekday names
            if let Some(weekday) = parse_weekday(s) {
                return Some(next_weekday(today, weekday, false));
            }

            // Try "jan 15" or "january 15" format
            if let Some(date) = parse_month_day(s, today) {
                return Some(date);
            }

            // Try relative days like "+3" or "3d"
            if let Some(days) = parse_relative_days(s) {
                return Some(today + Duration::days(days));
            }

            // Try mm/dd, m/dd, mm/d, m/d formats (with optional /yy or /yyyy)
            if let Some(date) = parse_slash_date(s, today) {
                return Some(date);
            }

            None
        }
    }
}

/// Parse dates in mm/dd, m/dd, mm/d, m/d format with optional /yy or /yyyy
fn parse_slash_date(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split('/').collect();

    match parts.len() {
        2 => {
            // mm/dd or m/d format
            let month: u32 = parts[0].parse().ok()?;
            let day: u32 = parts[1].parse().ok()?;

            if month < 1 || month > 12 || day < 1 || day > 31 {
                return None;
            }

            // Use current year, but if the date has passed, use next year
            let mut year = today.year();
            if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                if date < today {
                    year += 1;
                }
                NaiveDate::from_ymd_opt(year, month, day)
            } else {
                None
            }
        }
        3 => {
            // mm/dd/yy or mm/dd/yyyy format
            let month: u32 = parts[0].parse().ok()?;
            let day: u32 = parts[1].parse().ok()?;
            let year_part: i32 = parts[2].parse().ok()?;

            if month < 1 || month > 12 || day < 1 || day > 31 {
                return None;
            }

            // Handle 2-digit vs 4-digit year
            let year = if year_part < 100 {
                // 2-digit year: assume 2000s for 00-99
                2000 + year_part
            } else {
                year_part
            };

            NaiveDate::from_ymd_opt(year, month, day)
        }
        _ => None,
    }
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tues" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thurs" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

fn next_weekday(from: NaiveDate, target: Weekday, skip_this_week: bool) -> NaiveDate {
    let current = from.weekday().num_days_from_monday();
    let target_num = target.num_days_from_monday();

    let mut days_ahead = (target_num as i64) - (current as i64);

    if days_ahead <= 0 || skip_this_week {
        days_ahead += 7;
    }

    from + Duration::days(days_ahead)
}

fn parse_month_day(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }

    let month = match parts[0] {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    };

    let day: u32 = parts[1].parse().ok()?;

    // Use current year, but if the date has passed, use next year
    let mut year = today.year();
    if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
        if date < today {
            year += 1;
        }
        NaiveDate::from_ymd_opt(year, month, day)
    } else {
        None
    }
}

fn parse_relative_days(s: &str) -> Option<i64> {
    // Handle "+3" format
    if let Some(num) = s.strip_prefix('+') {
        return num.parse().ok();
    }

    // Handle "3d" format
    if let Some(num) = s.strip_suffix('d') {
        return num.parse().ok();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Date parsing tests with [date:...] syntax
    #[test]
    fn test_parse_today() {
        let (text, date) = parse_date("Buy groceries [date:today]");
        assert_eq!(text, "Buy groceries");
        assert!(date.is_some());
    }

    #[test]
    fn test_parse_tomorrow() {
        let (text, date) = parse_date("Call mom [date:tomorrow]");
        assert_eq!(text, "Call mom");
        assert!(date.is_some());
    }

    #[test]
    fn test_no_date() {
        let (text, date) = parse_date("Just a regular task");
        assert_eq!(text, "Just a regular task");
        assert!(date.is_none());
    }

    #[test]
    fn test_parse_slash_date_mmdd() {
        let (text, date) = parse_date("Buy groceries [date:1/15]");
        assert_eq!(text, "Buy groceries");
        assert!(date.is_some());
        let d = date.unwrap();
        assert_eq!(d.month(), 1);
        assert_eq!(d.day(), 15);
    }

    #[test]
    fn test_parse_slash_date_with_year() {
        let (text, date) = parse_date("Pay taxes [date:4/15/25]");
        assert_eq!(text, "Pay taxes");
        assert!(date.is_some());
        let d = date.unwrap();
        assert_eq!(d.month(), 4);
        assert_eq!(d.day(), 15);
        assert_eq!(d.year(), 2025);
    }

    #[test]
    fn test_parse_slash_date_full_year() {
        let (text, date) = parse_date("Event [date:12/25/2026]");
        assert_eq!(text, "Event");
        assert!(date.is_some());
        let d = date.unwrap();
        assert_eq!(d.month(), 12);
        assert_eq!(d.day(), 25);
        assert_eq!(d.year(), 2026);
    }

    #[test]
    fn test_date_short_alias() {
        let (text, date) = parse_date("Task [d:tomorrow]");
        assert_eq!(text, "Task");
        assert!(date.is_some());
    }

    #[test]
    fn test_date_weekday() {
        let (text, date) = parse_date("Meeting [date:monday]");
        assert_eq!(text, "Meeting");
        assert!(date.is_some());
    }

    #[test]
    fn test_date_next_weekday() {
        let (text, date) = parse_date("Meeting [date:next friday]");
        assert_eq!(text, "Meeting");
        assert!(date.is_some());
    }

    #[test]
    fn test_date_month_day() {
        let (text, date) = parse_date("Birthday [date:jan 15]");
        assert_eq!(text, "Birthday");
        assert!(date.is_some());
        let d = date.unwrap();
        assert_eq!(d.month(), 1);
        assert_eq!(d.day(), 15);
    }

    #[test]
    fn test_date_relative() {
        let (text, date) = parse_date("Reminder [date:+3]");
        assert_eq!(text, "Reminder");
        assert!(date.is_some());
    }

    #[test]
    fn test_date_relative_days() {
        let (text, date) = parse_date("Reminder [date:5d]");
        assert_eq!(text, "Reminder");
        assert!(date.is_some());
    }

    // Priority parsing tests
    #[test]
    fn test_priority_and_date_combined() {
        // Test priority parsing
        let (text, priority) = parse_priority("Buy groceries [priority:high] [date:tomorrow]");
        assert_eq!(text, "Buy groceries [date:tomorrow]");
        assert_eq!(priority, Priority::High);

        // Then date parsing on the result
        let (final_text, date) = parse_date(&text);
        assert_eq!(final_text, "Buy groceries");
        assert!(date.is_some());
    }

    #[test]
    fn test_priority_and_date_short_aliases() {
        let (text, priority) = parse_priority("Task [p:high] [d:tomorrow]");
        assert_eq!(text, "Task [d:tomorrow]");
        assert_eq!(priority, Priority::High);

        let (final_text, date) = parse_date(&text);
        assert_eq!(final_text, "Task");
        assert!(date.is_some());
    }

    #[test]
    fn test_priority_low() {
        let (text, priority) = parse_priority("Low priority task [priority:low]");
        assert_eq!(text, "Low priority task");
        assert_eq!(priority, Priority::Low);
    }

    #[test]
    fn test_priority_max() {
        let (text, priority) = parse_priority("[priority:max] Urgent task");
        assert_eq!(text, "Urgent task");
        assert_eq!(priority, Priority::Max);
    }

    #[test]
    fn test_priority_case_insensitive() {
        let (text, priority) = parse_priority("Task [PRIORITY:HIGH]");
        assert_eq!(text, "Task");
        assert_eq!(priority, Priority::High);
    }

    #[test]
    fn test_priority_short_alias() {
        let (text, priority) = parse_priority("Task [p:medium]");
        assert_eq!(text, "Task");
        assert_eq!(priority, Priority::Medium);
    }

    #[test]
    fn test_no_priority() {
        let (text, priority) = parse_priority("Regular task");
        assert_eq!(text, "Regular task");
        assert_eq!(priority, Priority::None);
    }
}
