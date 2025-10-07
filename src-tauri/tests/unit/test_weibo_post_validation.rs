//! WeiboPost模型验证单元测试
//!
//! 覆盖:
//! - validate()方法的所有验证规则
//! - JSON序列化/反序列化
//! - 边界情况和异常输入

use chrono::{DateTime, Duration, Utc};
use weibo_login::models::weibo_post::WeiboPost;

// ============================================================
// 1. validate()方法测试
// ============================================================

#[test]
fn test_validate_success() {
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    assert!(post.validate().is_ok());
}

#[test]
fn test_validate_empty_id() {
    let now = Utc::now();
    let post = WeiboPost::new(
        "".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    let result = post.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "帖子ID不能为空");
}

#[test]
fn test_validate_whitespace_id() {
    let now = Utc::now();
    let post = WeiboPost::new(
        "   ".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    let result = post.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "帖子ID不能为空");
}

#[test]
fn test_validate_empty_author_uid() {
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now - Duration::hours(1),
        "".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    let result = post.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "作者UID不能为空");
}

#[test]
fn test_validate_whitespace_author_uid() {
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now - Duration::hours(1),
        "   ".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    let result = post.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "作者UID不能为空");
}

#[test]
fn test_validate_created_at_after_crawled_at() {
    let now = Utc::now();
    let mut post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    // 手动设置created_at晚于crawled_at
    post.created_at = post.crawled_at + Duration::hours(1);

    let result = post.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "帖子发布时间不能晚于爬取时间");
}

#[test]
fn test_validate_created_at_equals_crawled_at() {
    let now = Utc::now();
    let mut post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now,
        "1234567890".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    // 设置相同时间 (边界情况)
    post.created_at = post.crawled_at;

    assert!(post.validate().is_ok());
}

#[test]
fn test_validate_allows_empty_text() {
    // 根据data-model.md，text字段没有非空验证
    // 但现实中微博不太可能完全空白
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "".to_string(), // 空文本
        now - Duration::hours(1),
        "1234567890".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    assert!(post.validate().is_ok());
}

#[test]
fn test_validate_allows_empty_author_screen_name() {
    // author_screen_name没有验证规则
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "".to_string(), // 空昵称
        100,
        50,
        200,
    );

    assert!(post.validate().is_ok());
}

#[test]
fn test_validate_allows_zero_interaction_counts() {
    // 互动数据为0是合法的 (新帖子)
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "测试用户".to_string(),
        0, // 0转发
        0, // 0评论
        0, // 0点赞
    );

    assert!(post.validate().is_ok());
}

// ============================================================
// 2. JSON序列化/反序列化测试
// ============================================================

#[test]
fn test_json_serialization_deserialization_roundtrip() {
    let now = Utc::now();
    let original = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "这是一条测试微博".to_string(),
        now - Duration::hours(2),
        "1234567890".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    // 序列化
    let json = original.to_json().expect("序列化失败");

    // 反序列化
    let restored = WeiboPost::from_json(&json).expect("反序列化失败");

    // 验证所有字段一致
    assert_eq!(restored.id, original.id);
    assert_eq!(restored.task_id, original.task_id);
    assert_eq!(restored.text, original.text);
    assert_eq!(restored.created_at, original.created_at);
    assert_eq!(restored.author_uid, original.author_uid);
    assert_eq!(restored.author_screen_name, original.author_screen_name);
    assert_eq!(restored.reposts_count, original.reposts_count);
    assert_eq!(restored.comments_count, original.comments_count);
    assert_eq!(restored.attitudes_count, original.attitudes_count);
    assert_eq!(restored.crawled_at, original.crawled_at);
}

#[test]
fn test_json_deserialization_missing_field() {
    // 缺少必需字段的JSON应该失败
    let invalid_json = r#"{"id":"5008471234567890","task_id":"task123"}"#;

    let result = WeiboPost::from_json(invalid_json);
    assert!(result.is_err());
}

#[test]
fn test_json_deserialization_invalid_type() {
    // 字段类型错误应该失败
    let invalid_json = r#"{
        "id": "5008471234567890",
        "task_id": "task123",
        "text": "测试",
        "created_at": "not-a-date",
        "author_uid": "123",
        "author_screen_name": "用户",
        "reposts_count": 100,
        "comments_count": 50,
        "attitudes_count": 200,
        "crawled_at": "2025-10-07T00:00:00Z"
    }"#;

    let result = WeiboPost::from_json(invalid_json);
    assert!(result.is_err());
}

#[test]
fn test_json_deserialization_invalid_number() {
    // 数字字段为负数应该失败 (u64不支持负数)
    let invalid_json = r#"{
        "id": "5008471234567890",
        "task_id": "task123",
        "text": "测试",
        "created_at": "2025-10-07T00:00:00Z",
        "author_uid": "123",
        "author_screen_name": "用户",
        "reposts_count": -100,
        "comments_count": 50,
        "attitudes_count": 200,
        "crawled_at": "2025-10-07T00:00:00Z"
    }"#;

    let result = WeiboPost::from_json(invalid_json);
    assert!(result.is_err());
}

#[test]
fn test_json_deserialization_malformed() {
    // 格式错误的JSON
    let invalid_json = "not a valid json {{{";

    let result = WeiboPost::from_json(invalid_json);
    assert!(result.is_err());
}

#[test]
fn test_json_serialization_structure() {
    let now = DateTime::parse_from_rfc3339("2025-10-07T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let mut post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "测试微博".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    // 固定crawled_at以便验证JSON结构
    post.crawled_at = now;

    let json = post.to_json().expect("序列化失败");

    // 验证JSON包含所有必需字段
    assert!(json.contains("\"id\":\"5008471234567890\""));
    assert!(json.contains("\"task_id\":\"task123\""));
    assert!(json.contains("\"text\":\"测试微博\""));
    assert!(json.contains("\"author_uid\":\"1234567890\""));
    assert!(json.contains("\"author_screen_name\":\"测试用户\""));
    assert!(json.contains("\"reposts_count\":100"));
    assert!(json.contains("\"comments_count\":50"));
    assert!(json.contains("\"attitudes_count\":200"));
}

// ============================================================
// 3. 边界情况测试
// ============================================================

#[test]
fn test_boundary_large_interaction_counts() {
    // 测试极大的互动数据 (爆款微博)
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "病毒式传播".to_string(),
        now - Duration::days(7),
        "1234567890".to_string(),
        "大V账号".to_string(),
        u64::MAX, // 最大u64值
        u64::MAX,
        u64::MAX,
    );

    assert!(post.validate().is_ok());

    // 验证序列化不会溢出
    let json = post.to_json();
    assert!(json.is_ok());
}

#[test]
fn test_boundary_unicode_text() {
    // Unicode字符 (emoji, 多语言)
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "测试微博 🎉🎊 Test Post مرحبا 你好 こんにちは 🌟".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "多语言用户 🌍".to_string(),
        100,
        50,
        200,
    );

    assert!(post.validate().is_ok());

    // 验证序列化和反序列化保持Unicode完整性
    let json = post.to_json().expect("序列化失败");
    let restored = WeiboPost::from_json(&json).expect("反序列化失败");
    assert_eq!(restored.text, post.text);
    assert_eq!(restored.author_screen_name, post.author_screen_name);
}

#[test]
fn test_boundary_special_characters() {
    // 特殊字符: 换行, 引号, 反斜杠
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "第一行\n第二行\t制表符\r回车符\\反斜杠\"引号\"".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "特殊\"用户\\名".to_string(),
        100,
        50,
        200,
    );

    assert!(post.validate().is_ok());

    // 验证JSON转义正确
    let json = post.to_json().expect("序列化失败");
    let restored = WeiboPost::from_json(&json).expect("反序列化失败");
    assert_eq!(restored.text, post.text);
    assert_eq!(restored.author_screen_name, post.author_screen_name);
}

#[test]
fn test_boundary_very_long_text() {
    // 微博长文本 (可能有字数限制，但模型不限制)
    let long_text = "微博".repeat(5000); // 10000个字符
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        long_text.clone(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "话唠用户".to_string(),
        100,
        50,
        200,
    );

    assert!(post.validate().is_ok());

    // 验证序列化大文本
    let json = post.to_json();
    assert!(json.is_ok());

    // 验证反序列化保持文本完整
    let restored = WeiboPost::from_json(&json.unwrap()).expect("反序列化失败");
    assert_eq!(restored.text, long_text);
}

#[test]
fn test_boundary_very_old_post() {
    // 非常久远的微博 (2009年微博上线)
    let very_old = DateTime::parse_from_rfc3339("2009-08-14T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "task123".to_string(),
        "微博元年".to_string(),
        very_old,
        "1234567890".to_string(),
        "元老用户".to_string(),
        100,
        50,
        200,
    );

    assert!(post.validate().is_ok());
}

#[test]
fn test_boundary_task_id_uuid() {
    // 验证UUID格式的task_id
    let now = Utc::now();
    let post = WeiboPost::new(
        "5008471234567890".to_string(),
        "550e8400-e29b-41d4-a716-446655440000".to_string(), // UUID v4
        "测试微博".to_string(),
        now - Duration::hours(1),
        "1234567890".to_string(),
        "测试用户".to_string(),
        100,
        50,
        200,
    );

    assert!(post.validate().is_ok());
}

#[test]
fn test_boundary_minimal_valid_post() {
    // 最小化合法帖子 (所有可选内容为空或0)
    let now = Utc::now();
    let post = WeiboPost::new(
        "1".to_string(), // 最短ID
        "1".to_string(), // 最短task_id
        "".to_string(),  // 空文本
        now - Duration::seconds(1),
        "1".to_string(), // 最短author_uid
        "".to_string(),  // 空昵称
        0,
        0,
        0,
    );

    assert!(post.validate().is_ok());
}
