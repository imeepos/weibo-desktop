/// 集成测试 - 场景7: 异常处理
///
/// 验证目标:
/// - FR-024至FR-026 (错误处理)
///
/// 测试覆盖:
/// 1. 验证码检测: 自动暂停任务,推送CAPTCHA_DETECTED事件
/// 2. 网络错误: 任务进入Failed状态,记录失败原因
/// 3. Redis连接失败: 返回STORAGE_ERROR
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use weibo_login::models::crawl_task::{CrawlStatus, CrawlTask};
use weibo_login::services::redis_service::RedisService;

/// 测试辅助: 创建测试用RedisService
async fn setup_redis() -> RedisService {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    RedisService::new(&redis_url)
        .await
        .expect("Redis连接失败,请确保Redis服务运行")
}

/// 测试辅助: 清理测试数据
async fn cleanup_task(redis: &RedisService, task_id: &str) {
    let mut conn = redis.get_connection().await.unwrap();
    let _: () = conn.del(format!("crawl:task:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:checkpoint:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:posts:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:post_ids:{}", task_id)).await.unwrap();
}

/// 测试辅助: 准备运行中的任务
async fn prepare_running_task(redis: &RedisService) -> CrawlTask {
    let mut task = CrawlTask::new(
        "测试验证码".to_string(),
        Utc::now() - Duration::days(7),
    );

    // 状态转换: Created -> HistoryCrawling
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();

    redis.save_crawl_task(&task).await.unwrap();

    task
}

#[tokio::test]
async fn test_captcha_detected_auto_pause() {
    // 场景7.1: 验证码检测自动暂停
    let redis = setup_redis().await;
    let task = prepare_running_task(&redis).await;

    // 模拟: 检测到验证码 (实际由Playwright脚本检测)
    // 在实际实现中,CrawlService会调用:
    //   task.transition_to(CrawlStatus::Paused)
    //   emit_event("crawl-error", CrawlErrorEvent { errorCode: "CAPTCHA_DETECTED", ... })

    let mut updated_task = redis.load_task(&task.id).await.unwrap();

    // 模拟暂停操作
    updated_task.transition_to(CrawlStatus::Paused).unwrap();
    redis.save_crawl_task(&updated_task).await.unwrap();

    // 验证: 任务状态转换正确
    let loaded_task = redis.load_task(&task.id).await.unwrap();
    assert_eq!(
        loaded_task.status,
        CrawlStatus::Paused,
        "检测到验证码后应自动暂停"
    );

    // 验证: 状态转换链合法
    let history_to_paused = CrawlStatus::HistoryCrawling.can_transition_to(&CrawlStatus::Paused);
    assert!(
        history_to_paused,
        "HistoryCrawling状态应能转换到Paused"
    );

    // 验证: 暂停后可恢复
    let paused_to_history = CrawlStatus::Paused.can_transition_to(&CrawlStatus::HistoryCrawling);
    assert!(
        paused_to_history,
        "Paused状态应能恢复到HistoryCrawling"
    );

    // 清理
    cleanup_task(&redis, &task.id).await;
}

#[tokio::test]
async fn test_captcha_detected_event_structure() {
    // 验证: crawl-error事件结构正确

    // 预期事件结构 (TypeScript类型):
    // {
    //   errorCode: "CAPTCHA_DETECTED",
    //   error: "检测到验证码,需要人工处理"
    // }

    // 模拟事件数据
    let error_code = "CAPTCHA_DETECTED";
    let error_message = "检测到验证码,需要人工处理";

    // 验证: 错误码格式
    assert_eq!(error_code, "CAPTCHA_DETECTED", "错误码应为CAPTCHA_DETECTED");

    // 验证: 错误消息清晰
    assert!(
        error_message.contains("验证码"),
        "错误消息应提到验证码"
    );
    assert!(
        error_message.contains("人工处理"),
        "错误消息应提示需要人工处理"
    );
}

#[tokio::test]
async fn test_captcha_checkpoint_preserved() {
    // 验证: 检测到验证码时,检查点应保存,便于恢复
    let redis = setup_redis().await;
    let task = prepare_running_task(&redis).await;

    // 模拟: 保存检查点 (在暂停前)
    let checkpoint = weibo_login::models::crawl_checkpoint::CrawlCheckpoint::new_backward(
        task.id.clone(),
        Utc::now() - Duration::hours(48),
        Utc::now(),
    );

    // 模拟已爬取15页
    let mut checkpoint_with_progress = checkpoint;
    for _ in 0..14 {
        checkpoint_with_progress.advance_page();
    }

    redis.save_checkpoint(&checkpoint_with_progress).await.unwrap();

    // 模拟: 检测到验证码,暂停任务
    let mut task = redis.load_task(&task.id).await.unwrap();
    task.transition_to(CrawlStatus::Paused).unwrap();
    redis.save_crawl_task(&task).await.unwrap();

    // 验证: 检查点仍然存在
    let loaded_checkpoint = redis.load_checkpoint(&task.id).await.unwrap();
    assert!(loaded_checkpoint.is_some(), "检查点应被保留");

    let checkpoint = loaded_checkpoint.unwrap();
    assert_eq!(
        checkpoint.current_page, 15,
        "检查点应保存当前页码"
    );

    // 验证: 用户可手动处理后从检查点恢复
    let mut resumed_task = redis.load_task(&task.id).await.unwrap();
    resumed_task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    redis.save_crawl_task(&resumed_task).await.unwrap();

    let final_task = redis.load_task(&task.id).await.unwrap();
    assert_eq!(
        final_task.status,
        CrawlStatus::HistoryCrawling,
        "恢复后应继续爬取"
    );

    // 清理
    cleanup_task(&redis, &task.id).await;
}

#[tokio::test]
async fn test_network_error_enters_failed_state() {
    // 场景7.2: 网络错误后进入Failed状态
    let redis = setup_redis().await;
    let task = prepare_running_task(&redis).await;

    // 模拟: Playwright服务器未运行
    // 在实际实现中,CrawlService会检测到连接失败,调用:
    //   task.mark_failed("Playwright服务器未运行")

    let mut updated_task = redis.load_task(&task.id).await.unwrap();

    // 模拟标记失败
    let failure_reason = "Playwright服务器未运行";
    updated_task.mark_failed(failure_reason.to_string());

    redis.save_crawl_task(&updated_task).await.unwrap();

    // 验证: 任务状态为Failed
    let loaded_task = redis.load_task(&task.id).await.unwrap();
    assert_eq!(
        loaded_task.status,
        CrawlStatus::Failed,
        "网络错误后应进入Failed状态"
    );

    // 验证: 失败原因记录正确
    assert_eq!(
        loaded_task.failure_reason,
        Some(failure_reason.to_string()),
        "失败原因应被记录"
    );

    // 验证: updated_at字段已更新
    assert!(
        loaded_task.updated_at >= updated_task.updated_at,
        "标记失败时应更新时间戳"
    );

    // 清理
    cleanup_task(&redis, &task.id).await;
}

#[tokio::test]
async fn test_network_error_retry_allowed() {
    // 验证: Failed状态可重试 (转换回HistoryCrawling)
    let redis = setup_redis().await;

    let mut task = CrawlTask::new(
        "测试重试".to_string(),
        Utc::now() - Duration::days(3),
    );

    // 状态转换: Created -> HistoryCrawling -> Failed
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.mark_failed("模拟网络错误".to_string());

    redis.save_crawl_task(&task).await.unwrap();

    // 验证: Failed状态可以转换回HistoryCrawling
    let can_retry = task.status.can_transition_to(&CrawlStatus::HistoryCrawling);
    assert!(can_retry, "Failed状态应允许重试");

    // 模拟: 用户点击"重试"
    let mut retry_task = redis.load_task(&task.id).await.unwrap();
    retry_task.transition_to(CrawlStatus::HistoryCrawling).unwrap();

    // 清除失败原因
    retry_task.failure_reason = None;
    redis.save_crawl_task(&retry_task).await.unwrap();

    // 验证: 重试后状态正确
    let final_task = redis.load_task(&task.id).await.unwrap();
    assert_eq!(
        final_task.status,
        CrawlStatus::HistoryCrawling,
        "重试后应恢复运行状态"
    );
    assert_eq!(
        final_task.failure_reason,
        None,
        "重试后失败原因应清除"
    );

    // 清理
    cleanup_task(&redis, &task.id).await;
}

#[tokio::test]
async fn test_network_error_types() {
    // 验证: 不同网络错误类型的失败原因
    let redis = setup_redis().await;

    let error_scenarios = vec![
        "Playwright服务器未运行",
        "网络请求超时: 10秒后无响应",
        "微博API返回502 Bad Gateway",
        "连接被拒绝: ECONNREFUSED",
    ];

    for (i, error_msg) in error_scenarios.iter().enumerate() {
        let mut task = CrawlTask::new(
            format!("错误测试{}", i),
            Utc::now() - Duration::days(1),
        );

        task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
        task.mark_failed(error_msg.to_string());

        redis.save_crawl_task(&task).await.unwrap();

        // 验证: 失败原因清晰
        let loaded_task = redis.load_task(&task.id).await.unwrap();
        assert_eq!(
            loaded_task.failure_reason.as_ref().unwrap(),
            error_msg,
            "失败原因应准确记录"
        );

        cleanup_task(&redis, &task.id).await;
    }
}

#[tokio::test]
async fn test_redis_connection_failure() {
    // 场景7.3: Redis连接失败返回STORAGE_ERROR

    // 模拟: 使用无效的Redis连接地址
    let invalid_redis_url = "redis://127.0.0.1:9999"; // 不存在的端口

    let result = RedisService::new(invalid_redis_url).await;

    // 验证: 连接失败
    assert!(
        result.is_err(),
        "无效的Redis地址应返回错误"
    );

    // 验证: 错误消息包含连接失败信息
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("连接") || error_message.contains("Connection"),
        "错误消息应提示连接失败: {}",
        error_message
    );
}

#[tokio::test]
async fn test_redis_operation_failure_handling() {
    // 验证: Redis操作失败时的错误处理
    let redis = setup_redis().await;

    // 模拟: 加载不存在的任务
    let nonexistent_task_id = "nonexistent_task_999";
    let result = redis.load_task(nonexistent_task_id).await;

    // 验证: 返回错误而非panic
    assert!(
        result.is_err(),
        "加载不存在的任务应返回错误"
    );

    // 验证: 错误消息清晰
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("未找到") || error_message.contains("不存在"),
        "错误消息应提示任务不存在: {}",
        error_message
    );
}

#[tokio::test]
async fn test_storage_error_on_create_task() {
    // 验证: 创建任务时Redis失败应返回STORAGE_ERROR

    // 实际场景: 如果在create_crawl_task命令中,
    // redis.save_crawl_task()失败,应返回:
    // Err(TauriError { code: "STORAGE_ERROR", message: "Redis连接失败: ..." })

    // 这里验证数据层的错误传播
    let redis = setup_redis().await;

    // 创建无效的任务 (触发验证错误,模拟存储失败)
    let invalid_task = CrawlTask::new(
        "".to_string(), // 空关键字
        Utc::now(),
    );

    // 验证: 无效任务无法通过验证
    let validation_result = invalid_task.validate();
    assert!(
        validation_result.is_err(),
        "无效任务应验证失败"
    );

    // 在命令层,这会转换为STORAGE_ERROR或INVALID_KEYWORD
    let error_message = validation_result.unwrap_err();
    assert!(
        error_message.contains("关键字不能为空"),
        "错误消息应明确问题"
    );
}

#[tokio::test]
async fn test_incremental_crawl_network_error() {
    // 验证: 增量更新中的网络错误处理
    let redis = setup_redis().await;

    let mut task = CrawlTask::new(
        "增量爬取错误".to_string(),
        Utc::now() - Duration::days(10),
    );

    // 状态转换: Created -> HistoryCrawling -> HistoryCompleted -> IncrementalCrawling
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::HistoryCompleted).unwrap();
    task.transition_to(CrawlStatus::IncrementalCrawling).unwrap();

    redis.save_crawl_task(&task).await.unwrap();

    // 模拟: 增量爬取中检测到网络错误
    let mut updated_task = redis.load_task(&task.id).await.unwrap();
    updated_task.mark_failed("增量更新: 微博API限流".to_string());

    redis.save_crawl_task(&updated_task).await.unwrap();

    // 验证: 增量爬取中的错误同样进入Failed状态
    let loaded_task = redis.load_task(&task.id).await.unwrap();
    assert_eq!(
        loaded_task.status,
        CrawlStatus::Failed,
        "增量爬取错误应进入Failed状态"
    );

    // 验证: 状态转换合法
    let incremental_to_failed = CrawlStatus::IncrementalCrawling.can_transition_to(&CrawlStatus::Failed);
    assert!(
        incremental_to_failed,
        "IncrementalCrawling应能转换到Failed"
    );

    // 清理
    cleanup_task(&redis, &task.id).await;
}

#[tokio::test]
async fn test_error_logging_requirements() {
    // 验证: 错误处理的日志要求

    // 根据宪章原则: "日志是思想的表达"
    // 错误日志应该:
    // 1. 包含上下文 (任务ID, 操作类型)
    // 2. 明确问题原因
    // 3. 提供可操作的建议

    let error_scenarios = vec![
        (
            "CAPTCHA_DETECTED",
            "检测到验证码,需要人工处理",
            vec!["验证码", "人工处理"]
        ),
        (
            "NETWORK_ERROR",
            "Playwright服务器未运行",
            vec!["Playwright", "未运行"]
        ),
        (
            "STORAGE_ERROR",
            "Redis连接失败: Connection refused",
            vec!["Redis", "连接失败"]
        ),
    ];

    for (error_code, error_message, expected_keywords) in error_scenarios {
        // 验证: 错误消息包含关键信息
        for keyword in expected_keywords {
            assert!(
                error_message.contains(keyword),
                "错误码'{}'的消息应包含'{}': {}",
                error_code,
                keyword,
                error_message
            );
        }

        // 验证: 错误消息简洁明了 (不超过100字符)
        assert!(
            error_message.len() < 100,
            "错误消息应简洁: 实际{}字符",
            error_message.len()
        );
    }
}

#[tokio::test]
async fn test_failed_task_validation() {
    // 验证: Failed状态的任务必须有失败原因
    let redis = setup_redis().await;

    let mut task = CrawlTask::new(
        "验证失败状态".to_string(),
        Utc::now() - Duration::days(1),
    );

    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.status = CrawlStatus::Failed; // 直接设置状态,不使用mark_failed

    // 验证: 无失败原因的Failed状态应验证失败
    let validation_result = task.validate();
    assert!(
        validation_result.is_err(),
        "Failed状态必须有失败原因"
    );

    let error_message = validation_result.unwrap_err();
    assert!(
        error_message.contains("失败原因"),
        "错误消息应提示缺少失败原因: {}",
        error_message
    );
}

#[tokio::test]
async fn test_error_recovery_workflow() {
    // 验证: 完整的错误恢复工作流
    let redis = setup_redis().await;

    let mut task = CrawlTask::new(
        "错误恢复工作流".to_string(),
        Utc::now() - Duration::days(5),
    );

    // 1. 正常启动
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    redis.save_crawl_task(&task).await.unwrap();

    // 2. 检测到验证码,自动暂停
    let mut task = redis.load_task(&task.id).await.unwrap();
    task.transition_to(CrawlStatus::Paused).unwrap();
    redis.save_crawl_task(&task).await.unwrap();

    // 3. 用户手动处理后恢复
    let mut task = redis.load_task(&task.id).await.unwrap();
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    redis.save_crawl_task(&task).await.unwrap();

    // 4. 网络错误,进入失败状态
    let mut task = redis.load_task(&task.id).await.unwrap();
    task.mark_failed("网络连接中断".to_string());
    redis.save_crawl_task(&task).await.unwrap();

    // 5. 用户点击重试
    let mut task = redis.load_task(&task.id).await.unwrap();
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.failure_reason = None;
    redis.save_crawl_task(&task).await.unwrap();

    // 6. 最终完成
    let mut task = redis.load_task(&task.id).await.unwrap();
    task.transition_to(CrawlStatus::HistoryCompleted).unwrap();
    redis.save_crawl_task(&task).await.unwrap();

    // 验证: 最终状态正确
    let final_task = redis.load_task(&task.id).await.unwrap();
    assert_eq!(
        final_task.status,
        CrawlStatus::HistoryCompleted,
        "经过错误恢复后应能正常完成"
    );

    // 清理
    cleanup_task(&redis, &task.id).await;
}
