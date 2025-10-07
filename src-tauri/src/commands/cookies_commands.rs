use crate::models::{CookiesData, StorageError, ValidationError};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;
use thiserror::Error;

/// 保存Cookies错误
///
/// 契约定义: specs/001-cookies/contracts/save_cookies.md:100
/// 扁平化错误结构,确保序列化格式符合契约要求
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error")]
pub enum SaveCookiesError {
    /// 个人资料API调用失败 (Cookies无效)
    #[error("个人资料API调用失败 (状态码 {status}): {message}")]
    ProfileApiFailed { status: u16, message: String },

    /// 缺少必需的cookie字段
    #[error("缺少必需的cookie字段: {cookie_name}")]
    MissingCookie { cookie_name: String },

    /// Playwright执行失败
    #[error("Playwright脚本执行失败: {message}")]
    PlaywrightFailed { message: String },

    /// Cookies格式无效
    #[error("Cookies格式无效: {message}")]
    InvalidFormat { message: String },

    /// UID提取失败
    #[error("无法提取用户UID: {message}")]
    UidExtractionFailed { message: String },

    /// Redis连接失败
    #[error("Redis连接失败: {message}")]
    RedisConnectionFailed { message: String },

    /// 指定UID的Cookies未找到
    #[error("未找到UID {uid} 的Cookies")]
    NotFound { uid: String },

    /// 序列化/反序列化失败
    #[error("数据序列化失败: {message}")]
    SerializationError { message: String },

    /// Redis操作超时
    #[error("Redis操作超时: {message}")]
    OperationTimeout { message: String },

    /// Redis命令执行失败
    #[error("Redis命令执行失败: {message}")]
    CommandFailed { message: String },

    /// UID不匹配
    #[error("UID不匹配: 期望 {expected}, 实际 {actual}")]
    UidMismatch { expected: String, actual: String },
}

/// 从 ValidationError 转换为 SaveCookiesError
impl From<ValidationError> for SaveCookiesError {
    fn from(err: ValidationError) -> Self {
        match err {
            ValidationError::ProfileApiFailed { status, message } => {
                SaveCookiesError::ProfileApiFailed { status, message }
            }
            ValidationError::MissingCookie(cookie_name) => {
                SaveCookiesError::MissingCookie { cookie_name }
            }
            ValidationError::PlaywrightFailed(message) => {
                SaveCookiesError::PlaywrightFailed { message }
            }
            ValidationError::InvalidFormat(message) => SaveCookiesError::InvalidFormat { message },
            ValidationError::UidExtractionFailed(message) => {
                SaveCookiesError::UidExtractionFailed { message }
            }
        }
    }
}

/// 从 StorageError 转换为 SaveCookiesError
impl From<StorageError> for SaveCookiesError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::RedisConnectionFailed(message) => {
                SaveCookiesError::RedisConnectionFailed { message }
            }
            StorageError::NotFound(uid) => SaveCookiesError::NotFound { uid },
            StorageError::SerializationError(message) => {
                SaveCookiesError::SerializationError { message }
            }
            StorageError::OperationTimeout(message) => {
                SaveCookiesError::OperationTimeout { message }
            }
            StorageError::CommandFailed(message) => SaveCookiesError::CommandFailed { message },
        }
    }
}

/// 保存Cookies响应
///
/// 向前端反馈操作结果的完整画像:
/// - success: 布尔值,最直接的结果
/// - redis_key: 存储位置,可用于调试
/// - validation_duration_ms: 性能指标,优化依据
/// - is_overwrite: 行为说明,UI展示差异
#[derive(Debug, Serialize)]
pub struct SaveCookiesResponse {
    pub success: bool,
    pub redis_key: String,
    pub validation_duration_ms: u64,
    pub is_overwrite: bool,
}

/// 保存Cookies命令
///
/// 契约定义: specs/001-cookies/contracts/save_cookies.md:31
/// 参数扁平化,直接接收 uid, cookies, screen_name。
///
/// 完整的验证-保存流程:
/// 1. 验证cookies有效性 (Playwright调用微博API)
/// 2. 确保UID匹配 (安全检查)
/// 3. 保存到Redis (持久化)
///
/// 返回:
/// - 成功: SaveCookiesResponse
/// - 失败: SaveCookiesError (Validation, Storage, UidMismatch)
#[tauri::command]
pub async fn save_cookies(
    uid: String,
    cookies: HashMap<String, String>,
    screen_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<SaveCookiesResponse, SaveCookiesError> {
    tracing::info!(
        用户ID = %uid,
        Cookies数量 = %cookies.len(),
        "调用save_cookies命令"
    );

    let start = std::time::Instant::now();

    // 验证cookies
    let (validated_uid, validated_screen_name) = state.validator.validate_cookies(&cookies).await?;

    // 确保UID匹配 - 安全性的基石
    if validated_uid != uid {
        return Err(SaveCookiesError::UidMismatch {
            expected: uid,
            actual: validated_uid,
        });
    }

    // 创建CookiesData
    let mut cookies_data = CookiesData::new(validated_uid, cookies);
    cookies_data = cookies_data.with_screen_name(screen_name.unwrap_or(validated_screen_name));

    // 验证CookiesData结构
    cookies_data.validate()?;

    // 保存到Redis
    let is_overwrite = state.redis.save_cookies(&cookies_data).await?;

    let validation_duration = start.elapsed();

    tracing::info!(
        用户ID = %cookies_data.uid,
        Redis键 = %cookies_data.redis_key,
        验证耗时毫秒 = %validation_duration.as_millis(),
        是否覆盖 = %is_overwrite,
        "Cookies保存成功"
    );

    Ok(SaveCookiesResponse {
        success: true,
        redis_key: cookies_data.redis_key,
        validation_duration_ms: validation_duration.as_millis() as u64,
        is_overwrite,
    })
}

/// 查询Cookies请求
#[derive(Debug, Deserialize)]
pub struct QueryCookiesRequest {
    pub uid: String,
}

/// 查询Cookies命令
///
/// 根据UID检索已保存的cookies。
/// 简单直接 - 输入UID,输出完整的CookiesData或错误。
///
/// 前端可用此命令:
/// - 检查cookies是否存在
/// - 获取完整cookies用于API调用
/// - 展示用户昵称 (screen_name)
#[tauri::command]
pub async fn query_cookies(
    request: QueryCookiesRequest,
    state: State<'_, AppState>,
) -> Result<CookiesData, String> {
    tracing::debug!(用户ID = %request.uid, "调用query_cookies命令");

    state
        .redis
        .query_cookies(&request.uid)
        .await
        .map_err(|e| format!("Query failed: {}", e))
}

/// 删除Cookies命令
///
/// 用户登出或cookies过期时调用。
/// 彻底清除Redis中的数据,不留痕迹。
///
/// 幂等性保证: 删除不存在的UID不会报错,
/// 因为结果一致 - "该UID的cookies不存在"。
#[tauri::command]
pub async fn delete_cookies(uid: String, state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!(用户ID = %uid, "调用delete_cookies命令");

    state
        .redis
        .delete_cookies(&uid)
        .await
        .map_err(|e| format!("Delete failed: {}", e))
}

/// 列出所有已保存的UIDs
///
/// 用于前端展示账号列表,支持多账号管理。
/// 返回所有存储的UID,前端可据此:
/// - 显示账号选择界面
/// - 批量查询每个UID的详细信息
/// - 统计已登录账号数量
#[tauri::command]
pub async fn list_all_uids(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    tracing::debug!("调用list_all_uids命令");

    state
        .redis
        .list_all_uids()
        .await
        .map_err(|e| format!("List failed: {}", e))
}
