//! 二维码会话管理器
//!
//! 职责: 确保同一时间只有一个活跃的二维码监控任务
//! 策略: 单例模式 - 新会话启动时自动终止旧会话

use tokio::sync::Mutex;
use tokio::task::AbortHandle;

/// 会话管理器
///
/// 存在即合理: 防止资源泄露的唯一看守者
/// - 跟踪当前活跃的监控任务
/// - 在新任务启动前终止旧任务
pub struct SessionManager {
    /// 当前活跃的会话 (qr_id, abort_handle)
    current_session: Mutex<Option<(String, AbortHandle)>>,
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        Self {
            current_session: Mutex::new(None),
        }
    }

    /// 设置新的活跃会话,自动取消旧会话
    ///
    /// # 参数
    /// - `qr_id`: 新二维码ID
    /// - `abort_handle`: 新任务的取消句柄
    ///
    /// # 副作用
    /// - 如果存在旧会话,将调用其abort()终止任务
    /// - WebSocket连接会随着任务终止而关闭
    pub async fn set_current_session(&self, qr_id: String, abort_handle: AbortHandle) {
        let mut guard = self.current_session.lock().await;

        // 取消旧会话
        if let Some((old_qr_id, old_handle)) = guard.take() {
            tracing::info!(
                旧二维码ID = %old_qr_id,
                新二维码ID = %qr_id,
                "取消旧会话,启动新会话"
            );
            old_handle.abort();
        } else {
            tracing::info!(
                二维码ID = %qr_id,
                "启动首个会话"
            );
        }

        // 设置新会话
        *guard = Some((qr_id, abort_handle));
    }

    /// 取消当前活跃会话
    ///
    /// 用于应用退出或手动清理场景
    pub async fn cancel_current_session(&self) {
        let mut guard = self.current_session.lock().await;

        if let Some((qr_id, handle)) = guard.take() {
            tracing::info!(二维码ID = %qr_id, "手动取消会话");
            handle.abort();
        }
    }

    /// 获取当前活跃会话的二维码ID (仅用于调试)
    #[allow(dead_code)]
    pub async fn current_qr_id(&self) -> Option<String> {
        self.current_session
            .lock()
            .await
            .as_ref()
            .map(|(id, _)| id.clone())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_replacement() {
        let manager = SessionManager::new();

        // 首次设置
        let (tx1, _rx1) = tokio::sync::oneshot::channel::<()>();
        let handle1 = tokio::spawn(async move {
            let _ = tx1;
            tokio::time::sleep(tokio::time::Duration::from_secs(100)).await;
        })
        .abort_handle();

        manager
            .set_current_session("qr1".to_string(), handle1)
            .await;
        assert_eq!(manager.current_qr_id().await, Some("qr1".to_string()));

        // 替换会话
        let (tx2, _rx2) = tokio::sync::oneshot::channel::<()>();
        let handle2 = tokio::spawn(async move {
            let _ = tx2;
            tokio::time::sleep(tokio::time::Duration::from_secs(100)).await;
        })
        .abort_handle();

        manager
            .set_current_session("qr2".to_string(), handle2)
            .await;
        assert_eq!(manager.current_qr_id().await, Some("qr2".to_string()));
    }

    #[tokio::test]
    async fn test_cancel_current_session() {
        let manager = SessionManager::new();

        let handle = tokio::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_secs(100)).await;
        })
        .abort_handle();

        manager.set_current_session("qr1".to_string(), handle).await;
        assert!(manager.current_qr_id().await.is_some());

        manager.cancel_current_session().await;
        assert!(manager.current_qr_id().await.is_none());
    }
}
