use chrono::{Datelike, NaiveDate, Timelike};
use weibo_login::utils::time_utils::{
    ceil_to_hour, floor_to_hour, format_weibo_time, parse_weibo_time,
};

// ============================================================================
// floor_to_hour 测试
// ============================================================================

#[test]
fn test_floor_to_hour_整点时间保持不变() {
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();

    let floored = floor_to_hour(dt);

    assert_eq!(floored, dt);
}

#[test]
fn test_floor_to_hour_非整点向下取整() {
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
fn test_floor_to_hour_边界_0分0秒() {
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();

    let floored = floor_to_hour(dt);

    assert_eq!(floored.hour(), 12);
    assert_eq!(floored.minute(), 0);
    assert_eq!(floored.second(), 0);
}

#[test]
fn test_floor_to_hour_边界_59分59秒() {
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(12, 59, 59)
        .unwrap()
        .and_utc();

    let floored = floor_to_hour(dt);

    assert_eq!(floored.hour(), 12);
    assert_eq!(floored.minute(), 0);
    assert_eq!(floored.second(), 0);
}

// ============================================================================
// ceil_to_hour 测试
// ============================================================================

#[test]
fn test_ceil_to_hour_整点时间保持不变() {
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();

    let ceiled = ceil_to_hour(dt);

    assert_eq!(ceiled, dt);
}

#[test]
fn test_ceil_to_hour_非整点向上取整() {
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
fn test_ceil_to_hour_边界_0分1秒() {
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(12, 0, 1)
        .unwrap()
        .and_utc();

    let ceiled = ceil_to_hour(dt);

    assert_eq!(ceiled.hour(), 13);
    assert_eq!(ceiled.minute(), 0);
    assert_eq!(ceiled.second(), 0);
}

#[test]
fn test_ceil_to_hour_边界_59分59秒() {
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(12, 59, 59)
        .unwrap()
        .and_utc();

    let ceiled = ceil_to_hour(dt);

    assert_eq!(ceiled.hour(), 13);
    assert_eq!(ceiled.minute(), 0);
    assert_eq!(ceiled.second(), 0);
}

#[test]
fn test_ceil_to_hour_边界_跨天() {
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(23, 30, 0)
        .unwrap()
        .and_utc();

    let ceiled = ceil_to_hour(dt);

    assert_eq!(ceiled.day(), 8);
    assert_eq!(ceiled.hour(), 0);
    assert_eq!(ceiled.minute(), 0);
    assert_eq!(ceiled.second(), 0);
}

// ============================================================================
// parse_weibo_time 测试
// ============================================================================

#[test]
fn test_parse_weibo_time_标准格式() {
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
fn test_parse_weibo_time_时区_正零时区() {
    let result = parse_weibo_time("Tue Oct 07 12:00:00 +0000 2025");

    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.hour(), 12);
}

#[test]
fn test_parse_weibo_time_时区_负时区() {
    let result = parse_weibo_time("Tue Oct 07 12:00:00 -0500 2025");

    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.hour(), 17); // UTC = -0500 + 5
}

#[test]
fn test_parse_weibo_time_时区_东八区() {
    let result = parse_weibo_time("Mon Oct 06 12:00:00 +0800 2025");

    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.hour(), 4); // UTC = +0800 - 8
}

#[test]
fn test_parse_weibo_time_错误格式_空字符串() {
    let result = parse_weibo_time("");
    assert!(result.is_err());
}

#[test]
fn test_parse_weibo_time_错误格式_无效字符串() {
    let result = parse_weibo_time("invalid time string");
    assert!(result.is_err());
}

#[test]
fn test_parse_weibo_time_错误格式_错误格式() {
    let result = parse_weibo_time("2025-10-07 12:34:56");
    assert!(result.is_err());
}

// ============================================================================
// format_weibo_time 测试
// ============================================================================

#[test]
fn test_format_weibo_time_标准格式() {
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(12, 34, 56)
        .unwrap()
        .and_utc();

    let formatted = format_weibo_time(dt);

    assert_eq!(formatted, "20251007120000");
}

#[test]
fn test_format_weibo_time_零填充_单数月日() {
    let dt = NaiveDate::from_ymd_opt(2025, 1, 5)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap()
        .and_utc();

    let formatted = format_weibo_time(dt);

    assert_eq!(formatted, "20250105090000");
}

#[test]
fn test_format_weibo_time_零填充_单数小时() {
    let dt = NaiveDate::from_ymd_opt(2025, 12, 31)
        .unwrap()
        .and_hms_opt(3, 0, 0)
        .unwrap()
        .and_utc();

    let formatted = format_weibo_time(dt);

    assert_eq!(formatted, "20251231030000");
}

#[test]
fn test_format_weibo_time_utc_conversion() {
    // UTC时间应直接格式化，不做时区转换
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let formatted = format_weibo_time(dt);

    assert_eq!(formatted, "20251007000000");
}

#[test]
fn test_format_weibo_time_忽略分秒() {
    // 验证分钟和秒被强制为00
    let dt = NaiveDate::from_ymd_opt(2025, 10, 7)
        .unwrap()
        .and_hms_opt(12, 59, 59)
        .unwrap()
        .and_utc();

    let formatted = format_weibo_time(dt);

    assert_eq!(formatted, "20251007120000");
    assert!(formatted.ends_with("0000"));
}

// ============================================================================
// 集成测试 - 往返一致性
// ============================================================================

#[test]
fn test_往返_floor_ceil一致性() {
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
fn test_往返_parse_format一致性() {
    let original = "Mon Oct 06 12:00:00 +0800 2025";
    let parsed = parse_weibo_time(original).unwrap();
    let formatted = format_weibo_time(parsed);

    // UTC: 2025-10-06 04:00:00
    assert_eq!(formatted, "20251006040000");
}
