use chrono::{DateTime, Timelike, Utc};

/// 向下取整到小时边界
///
/// 例: 2025-10-07 12:34:56 → 2025-10-07 12:00:00
pub fn floor_to_hour(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.date_naive()
        .and_hms_opt(dt.hour(), 0, 0)
        .expect("有效的小时值")
        .and_utc()
}

/// 向上取整到小时边界
///
/// 例: 2025-10-07 12:34:56 → 2025-10-07 13:00:00
/// 例: 2025-10-07 12:00:00 → 2025-10-07 12:00:00 (已对齐则保持不变)
pub fn ceil_to_hour(dt: DateTime<Utc>) -> DateTime<Utc> {
    let floored = floor_to_hour(dt);
    if floored == dt {
        dt
    } else {
        floored + chrono::Duration::hours(1)
    }
}

/// 解析微博时间字符串
///
/// 微博API返回格式: "Mon Oct 07 12:34:56 +0800 2025"
pub fn parse_weibo_time(time_str: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_str(time_str, "%a %b %d %H:%M:%S %z %Y")
        .map(|dt| dt.with_timezone(&Utc))
}

/// 格式化为微博API时间参数
///
/// 输出格式: YYYYMMDDhhmmss (例: 20251007120000)
/// 微博API仅支持小时精度，秒和分钟会被忽略
pub fn format_weibo_time(dt: DateTime<Utc>) -> String {
    dt.format("%Y%m%d%H0000").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_floor_to_hour_removes_minutes_and_seconds() {
        let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap()
            .and_utc();

        let floored = floor_to_hour(dt);

        assert_eq!(floored.hour(), 12);
        assert_eq!(floored.minute(), 0);
        assert_eq!(floored.second(), 0);
    }

    #[test]
    fn test_floor_to_hour_already_aligned() {
        let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc();

        let floored = floor_to_hour(dt);

        assert_eq!(floored, dt);
    }

    #[test]
    fn test_ceil_to_hour_rounds_up() {
        let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap()
            .and_utc();

        let ceiled = ceil_to_hour(dt);

        assert_eq!(ceiled.hour(), 13);
        assert_eq!(ceiled.minute(), 0);
        assert_eq!(ceiled.second(), 0);
    }

    #[test]
    fn test_ceil_to_hour_already_aligned() {
        let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc();

        let ceiled = ceil_to_hour(dt);

        assert_eq!(ceiled, dt);
    }

    #[test]
    fn test_ceil_to_hour_day_boundary() {
        let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
            .unwrap()
            .and_hms_opt(23, 30, 0)
            .unwrap()
            .and_utc();

        let ceiled = ceil_to_hour(dt);

        assert_eq!(ceiled.day(), 8);
        assert_eq!(ceiled.hour(), 0);
    }

    #[test]
    fn test_parse_weibo_time_valid() {
        let result = parse_weibo_time("Tue Oct 07 12:34:56 +0800 2025");
        assert!(result.is_ok());

        let dt = result.unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 10);
        assert_eq!(dt.day(), 7);
        assert_eq!(dt.hour(), 4); // UTC = +0800 - 8
        assert_eq!(dt.minute(), 34);
        assert_eq!(dt.second(), 56);
    }

    #[test]
    fn test_parse_weibo_time_invalid() {
        let result = parse_weibo_time("invalid time string");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_weibo_time() {
        let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap()
            .and_utc();

        let formatted = format_weibo_time(dt);

        assert_eq!(formatted, "20251007120000");
    }

    #[test]
    fn test_format_weibo_time_single_digit_month_and_day() {
        let dt = NaiveDate::from_ymd_opt(2025, 1, 5)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_utc();

        let formatted = format_weibo_time(dt);

        assert_eq!(formatted, "20250105090000");
    }

    #[test]
    fn test_round_trip_floor_ceil() {
        let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap()
            .and_utc();

        let floored = floor_to_hour(dt);
        let ceiled = ceil_to_hour(dt);

        assert_eq!(
            ceiled.signed_duration_since(floored).num_hours(),
            1,
            "向上和向下取整应相差1小时"
        );
    }

    #[test]
    fn test_parse_and_format_consistency() {
        let original = "Mon Oct 06 12:00:00 +0800 2025";
        let parsed = parse_weibo_time(original).unwrap();
        let formatted = format_weibo_time(parsed);

        // UTC: 2025-10-06 04:00:00
        assert_eq!(formatted, "20251006040000");
    }
}
