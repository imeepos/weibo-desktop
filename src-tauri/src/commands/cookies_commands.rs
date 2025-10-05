use crate::models::CookiesData;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

/// 保存Cookies请求
///
/// 契约定义: specs/001-cookies/contracts/save_cookies.md
/// 三个字段,三个核心信息:
/// - uid: 身份标识,存储的键
/// - cookies: 凭证本身,价值所在
/// - screen_name: 人性化展示,非必需但重要
#[derive(Debug, Deserialize)]
pub struct SaveCookiesRequest {
    pub uid: String,
    pub cookies: HashMap<String, String>,
    pub screen_name: Option<String>,
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
/// 完整的验证-保存流程:
/// 1. 验证cookies有效性 (Playwright调用微博API)
/// 2. 确保UID匹配 (安全检查)
/// 3. 保存到Redis (持久化)
///
/// # 错误处理即为人处世
/// 验证失败 -> 友善提示"Cookies无效,请重新登录"
/// UID不匹配 -> 明确指出问题所在
/// 存储失败 -> 说明"存储服务不可用"
///
/// 每个错误都是与用户的对话,而非技术性甩锅。
#[tauri::command]
pub async fn save_cookies(
    request: SaveCookiesRequest,
    state: State<'_, AppState>,
) -> Result<SaveCookiesResponse, String> {
    tracing::info!(
        uid = %request.uid,
        cookies_count = %request.cookies.len(),
        "save_cookies command called"
    );

    let start = std::time::Instant::now();

    // 验证cookies
    let (validated_uid, validated_screen_name) = state
        .validator
        .validate_cookies(&request.cookies)
        .await
        .map_err(|e| format!("Validation failed: {}", e))?;

    // 确保UID匹配 - 安全性的基石
    if validated_uid != request.uid {
        return Err(format!(
            "UID mismatch: expected {}, got {}",
            request.uid, validated_uid
        ));
    }

    // 创建CookiesData
    let mut cookies_data = CookiesData::new(validated_uid, request.cookies);
    cookies_data = cookies_data
        .with_screen_name(request.screen_name.unwrap_or(validated_screen_name));

    // 保存到Redis
    let is_overwrite = state
        .redis
        .save_cookies(&cookies_data)
        .await
        .map_err(|e| format!("Failed to save: {}", e))?;

    let validation_duration = start.elapsed();

    tracing::info!(
        uid = %cookies_data.uid,
        redis_key = %cookies_data.redis_key,
        validation_duration_ms = %validation_duration.as_millis(),
        is_overwrite = %is_overwrite,
        "Cookies saved successfully"
    );

    Ok(SaveCookiesResponse {
        success: true,
        redis_key: cookies_data.redis_key,
        validation_duration_ms: validation_duration.as_millis() as u64,
        is_overwrite,
    })
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
    uid: String,
    state: State<'_, AppState>,
) -> Result<CookiesData, String> {
    tracing::debug!(uid = %uid, "query_cookies command called");

    state
        .redis
        .query_cookies(&uid)
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
pub async fn delete_cookies(
    uid: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(uid = %uid, "delete_cookies command called");

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
pub async fn list_all_uids(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    tracing::debug!("list_all_uids command called");

    state
        .redis
        .list_all_uids()
        .await
        .map_err(|e| format!("List failed: {}", e))
}
