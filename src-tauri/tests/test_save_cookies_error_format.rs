use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 测试用的SaveCookiesError (扁平化版本)
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error")]
pub enum SaveCookiesError {
    #[error("个人资料API调用失败 (状态码 {status}): {message}")]
    ProfileApiFailed { status: u16, message: String },

    #[error("缺少必需的cookie字段: {cookie_name}")]
    MissingCookie { cookie_name: String },

    #[error("Redis连接失败: {message}")]
    RedisConnectionFailed { message: String },

    #[error("数据序列化失败: {message}")]
    SerializationError { message: String },

    #[error("UID不匹配: 期望 {expected}, 实际 {actual}")]
    UidMismatch { expected: String, actual: String },
}

#[test]
fn test_profile_api_failed_serialization() {
    let err = SaveCookiesError::ProfileApiFailed {
        status: 401,
        message: "Invalid credentials".to_string(),
    };

    let json = serde_json::to_string_pretty(&err).unwrap();
    println!("ProfileApiFailed serialization:\n{}", json);

    // 期望的契约格式:
    // {
    //   "error": "ProfileApiFailed",
    //   "status": 401,
    //   "message": "Invalid credentials"
    // }

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["error"], "ProfileApiFailed");
    assert_eq!(value["status"], 401);
    assert_eq!(value["message"], "Invalid credentials");
}

#[test]
fn test_missing_cookie_serialization() {
    let err = SaveCookiesError::MissingCookie {
        cookie_name: "SUB".to_string(),
    };

    let json = serde_json::to_string_pretty(&err).unwrap();
    println!("MissingCookie serialization:\n{}", json);

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["error"], "MissingCookie");
    assert_eq!(value["cookie_name"], "SUB");
}

#[test]
fn test_redis_connection_failed_serialization() {
    let err = SaveCookiesError::RedisConnectionFailed {
        message: "pool timeout".to_string(),
    };

    let json = serde_json::to_string_pretty(&err).unwrap();
    println!("RedisConnectionFailed serialization:\n{}", json);

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["error"], "RedisConnectionFailed");
    assert_eq!(value["message"], "pool timeout");
}

#[test]
fn test_uid_mismatch_serialization() {
    let err = SaveCookiesError::UidMismatch {
        expected: "123".to_string(),
        actual: "456".to_string(),
    };

    let json = serde_json::to_string_pretty(&err).unwrap();
    println!("UidMismatch serialization:\n{}", json);

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["error"], "UidMismatch");
    assert_eq!(value["expected"], "123");
    assert_eq!(value["actual"], "456");
}
