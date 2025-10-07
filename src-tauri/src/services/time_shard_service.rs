use chrono::{DateTime, Utc};
use std::boxed::Box;
use std::pin::Pin;
use std::future::Future;

use crate::utils::time_utils::{ceil_to_hour, floor_to_hour};

/// 时间分片服务
///
/// 职责：智能拆分时间范围，突破微博API的50页限制。
///
/// # 核心算法
///
/// 采用递归二分策略：
/// 1. 尝试估算时间范围内的结果数
/// 2. 若结果数 > 1000 (50页 × 20条/页)，二分时间范围
/// 3. 递归处理左右子范围，直到满足限制或达到最小粒度(1小时)
///
/// # 边界处理
///
/// - 所有时间自动取整到小时边界（微博API限制）
/// - 最小分片粒度：1小时
/// - 若1小时内仍>1000条，记录警告并返回该范围
///
/// # 示例
///
/// ```no_run
/// use chrono::Utc;
/// use time_shard_service::TimeShardService;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let service = TimeShardService::new();
///
/// let start = Utc::now() - chrono::Duration::days(30);
/// let end = Utc::now();
///
/// // 自动拆分为多个时间分片
/// let shards = service.split_time_range_if_needed(start, end, "国庆").await?;
///
/// for (shard_start, shard_end) in shards {
///     // 每个分片保证结果数 ≤ 1000
///     println!("爬取时间段: {} - {}", shard_start, shard_end);
/// }
/// # Ok(())
/// # }
/// ```
pub struct TimeShardService;

impl TimeShardService {
    /// 创建时间分片服务实例
    pub fn new() -> Self {
        Self
    }

    /// 拆分时间范围（如有必要）
    ///
    /// # 参数
    ///
    /// - `start`: 起始时间（将被向下取整到小时）
    /// - `end`: 结束时间（将被向上取整到小时）
    /// - `keyword`: 搜索关键字（用于估算结果数）
    ///
    /// # 返回值
    ///
    /// 子时间范围列表，每个范围内结果数 ≤ 1000（或已达最小粒度）
    ///
    /// # 算法说明
    ///
    /// 1. 时间取整到小时边界
    /// 2. 估算结果数（当前为mock实现，将来调用爬虫）
    /// 3. 若 ≤ 1000，直接返回原范围
    /// 4. 若 > 1000 且范围 > 1小时，二分递归
    /// 5. 若 > 1000 但已是1小时，记录警告并返回
    pub async fn split_time_range_if_needed(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        keyword: &str,
    ) -> Result<Vec<(DateTime<Utc>, DateTime<Utc>)>, String> {
        let start_aligned = floor_to_hour(start);
        let end_aligned = ceil_to_hour(end);

        self.split_recursive(start_aligned, end_aligned, keyword)
            .await
    }

    /// 递归拆分时间范围
    ///
    /// 核心算法实现，递归终止条件：
    /// - 结果数 ≤ 1000
    /// - 时间范围 ≤ 1小时
    #[allow(clippy::type_complexity)]
    fn split_recursive<'a>(
        &'a self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        keyword: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(DateTime<Utc>, DateTime<Utc>)>, String>> + Send + 'a>> {
        Box::pin(async move {
        const MAX_RESULTS: usize = 1000;
        const MIN_SHARD_HOURS: i64 = 1;

        // 估算结果数
        let total_results = self.estimate_total_results(start, end, keyword).await?;

        // 情况1: 结果数可接受，无需分片
        if total_results <= MAX_RESULTS {
            tracing::debug!(
                keyword,
                start = %start,
                end = %end,
                total_results,
                "时间范围无需分片"
            );
            return Ok(vec![(start, end)]);
        }

        // 情况2: 已达最小粒度，无法再分
        let duration_hours = end.signed_duration_since(start).num_hours();
        if duration_hours <= MIN_SHARD_HOURS {
            tracing::warn!(
                keyword,
                start = %start,
                end = %end,
                total_results,
                "时间范围仅{}小时但结果数超过限制，将跳过部分数据",
                duration_hours
            );
            return Ok(vec![(start, end)]);
        }

        // 情况3: 二分时间范围
        let mid = start + (end - start) / 2;
        let mid_aligned = floor_to_hour(mid);

        tracing::debug!(
            keyword,
            start = %start,
            end = %end,
            mid = %mid_aligned,
            total_results,
            "二分时间范围"
        );

            let left_shards = self.split_recursive(start, mid_aligned, keyword).await?;
            let right_shards = self.split_recursive(mid_aligned, end, keyword).await?;

            Ok([left_shards, right_shards].concat())
        })
    }

    /// 估算时间范围内的结果数
    ///
    /// # 实现说明
    ///
    /// 当前为mock实现，将来需要：
    /// 1. 调用Playwright爬取第一页
    /// 2. 从响应中提取总结果数提示
    /// 3. 处理网络错误和验证码
    ///
    /// # 参数
    ///
    /// - `start`: 开始时间
    /// - `end`: 结束时间
    /// - `keyword`: 搜索关键字
    ///
    /// # 返回值
    ///
    /// 估算的结果总数
    pub async fn estimate_total_results(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        keyword: &str,
    ) -> Result<usize, String> {
        // TODO: 集成Playwright爬虫
        // 当前为mock实现，基于时间范围长度粗略估算
        let duration_hours = end.signed_duration_since(start).num_hours();
        let estimated = (duration_hours as usize) * 50; // 假设每小时50条

        tracing::trace!(
            keyword,
            start = %start,
            end = %end,
            duration_hours,
            estimated,
            "估算结果数（mock实现）"
        );

        Ok(estimated)
    }
}

impl Default for TimeShardService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Timelike};

    fn create_datetime(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, 0, 0)
            .unwrap()
            .and_utc()
    }

    #[tokio::test]
    async fn test_no_split_when_results_below_limit() {
        let service = TimeShardService::new();

        // 1小时范围，估算50条，无需分片
        let start = create_datetime(2025, 10, 7, 12);
        let end = create_datetime(2025, 10, 7, 13);

        let shards = service
            .split_time_range_if_needed(start, end, "冷门关键字")
            .await
            .unwrap();

        assert_eq!(shards.len(), 1);
        assert_eq!(shards[0], (start, end));
    }

    #[tokio::test]
    async fn test_split_when_results_exceed_limit() {
        let service = TimeShardService::new();

        // 30小时范围，估算1500条，需要分片
        let start = create_datetime(2025, 10, 7, 0);
        let end = create_datetime(2025, 10, 8, 6);

        let shards = service
            .split_time_range_if_needed(start, end, "热门关键字")
            .await
            .unwrap();

        // 应该被拆分为多个分片
        assert!(shards.len() > 1);

        // 验证分片连续性
        for i in 0..shards.len() - 1 {
            assert_eq!(shards[i].1, shards[i + 1].0, "分片应该连续");
        }
    }

    #[tokio::test]
    async fn test_time_alignment_to_hour_boundary() {
        let service = TimeShardService::new();

        // 带有分钟的时间
        let start = NaiveDate::from_ymd_opt(2025, 10, 7)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap()
            .and_utc();
        let end = NaiveDate::from_ymd_opt(2025, 10, 7)
            .unwrap()
            .and_hms_opt(13, 45, 12)
            .unwrap()
            .and_utc();

        let shards = service
            .split_time_range_if_needed(start, end, "测试")
            .await
            .unwrap();

        // 验证时间已被取整
        assert_eq!(shards[0].0.minute(), 0);
        assert_eq!(shards[0].0.second(), 0);
        assert_eq!(shards[0].1.minute(), 0);
        assert_eq!(shards[0].1.second(), 0);
    }

    #[tokio::test]
    async fn test_minimum_shard_is_one_hour() {
        let service = TimeShardService::new();

        // 即使结果数超限，1小时也不会再分
        let start = create_datetime(2025, 10, 7, 12);
        let end = create_datetime(2025, 10, 7, 13);

        let shards = service
            .split_time_range_if_needed(start, end, "超热门")
            .await
            .unwrap();

        // 应该保持1个分片（虽然可能超过1000条）
        assert_eq!(shards.len(), 1);
    }

    #[tokio::test]
    async fn test_empty_keyword() {
        let service = TimeShardService::new();

        let start = create_datetime(2025, 10, 7, 12);
        let end = create_datetime(2025, 10, 7, 15);

        // 空关键字也应该正常处理
        let result = service.split_time_range_if_needed(start, end, "").await;

        assert!(result.is_ok());
    }
}
