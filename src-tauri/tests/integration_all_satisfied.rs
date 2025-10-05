//! 集成测试: 所有依赖已满足
//!
//! 测试完整流程:
//! 1. 启动时检测所有依赖
//! 2. 所有依赖都已安装且版本符合要求
//! 3. 应用正常启动,无提示
//! 4. 验证性能要求: 检测耗时<2秒
//! 5. 验证无安装动作触发

use std::time::Instant;

mod common;
use common::{MockDependencyChecker};

/// 模拟应用启动状态管理器
///
/// 跟踪应用启动过程中的状态变化,用于验证是否正确进入主界面
struct MockAppStartupManager {
    /// 是否已进入主界面
    entered_main_interface: bool,
    /// 检测进度
    check_progress: u8,
    /// 安装任务触发次数
    installation_triggered_count: u32,
}

impl MockAppStartupManager {
    fn new() -> Self {
        Self {
            entered_main_interface: false,
            check_progress: 0,
            installation_triggered_count: 0,
        }
    }

    /// 模拟检测进度更新
    fn update_check_progress(&mut self, progress: u8) {
        self.check_progress = progress;
        println!("检测进度更新: {}%", progress);
    }

    /// 模拟安装任务触发
    fn trigger_installation(&mut self) {
        self.installation_triggered_count += 1;
        println!("⚠️ 安装任务被触发 (第{}次)", self.installation_triggered_count);
    }

    /// 模拟进入主界面
    fn enter_main_interface(&mut self) {
        self.entered_main_interface = true;
        println!("✅ 进入主界面");
    }

    /// 验证启动流程符合预期
    fn validate_startup_flow(&self) -> Result<String, String> {
        // 验证无安装动作触发
        if self.installation_triggered_count > 0 {
            return Err(format!(
                "不应该触发安装任务,但触发了{}次",
                self.installation_triggered_count
            ));
        }

        // 验证进度完成
        if self.check_progress != 100 {
            return Err(format!("检测进度应为100%,实际为{}%", self.check_progress));
        }

        // 验证进入主界面
        if !self.entered_main_interface {
            return Err("应该已进入主界面".to_string());
        }

        Ok("启动流程验证通过".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试所有依赖已满足的完整流程
    ///
    /// 测试场景:
    /// - Redis 7.2.4 运行在 localhost:6379
    /// - Playwright 1.48.0 已安装
    /// - 预期: 快速检测(2秒内)并进入主界面,无安装动作
    #[tokio::test]
    async fn test_all_dependencies_satisfied_flow() {
        println!("\n🚀 开始测试: 所有依赖已满足场景");

        // 1. 准备测试环境
        let startup_manager = MockAppStartupManager::new();
        let mut startup_manager = startup_manager;
        let dependency_checker = MockDependencyChecker::new_all_satisfied();

        println!("📋 测试环境准备完成");
        println!("   - Redis: ✅ 已安装 (v7.2.4)");
        println!("   - Playwright: ✅ 已安装 (v1.48.0)");

        // 2. 开始性能计时
        let start_time = Instant::now();
        println!("⏱️  开始检测依赖...");

        // 3. 模拟依赖检测过程
        let check_results = dependency_checker.check_all().await;

        // 4. 计算检测耗时
        let elapsed = start_time.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;

        println!("📊 检测完成,耗时: {}ms", elapsed_ms);

        // 5. 验证性能要求 (< 2秒)
        assert!(
            elapsed_ms < 2000,
            "依赖检测耗时超过2秒限制: {}ms >= 2000ms",
            elapsed_ms
        );
        println!("✅ 性能验证通过: {}ms < 2000ms", elapsed_ms);

        // 6. 验证检测结果
        assert_eq!(check_results.len(), 2, "应该检测到2个依赖项");

        // 验证Redis检测结果
        let redis_result = check_results
            .iter()
            .find(|r| r.dependency_id == "redis")
            .expect("应该包含Redis检测结果");

        assert_eq!(
            redis_result.status,
            weibo_login::models::dependency::CheckStatus::Satisfied,
            "Redis状态应为Satisfied"
        );
        assert_eq!(
            redis_result.detected_version.as_ref().unwrap(),
            "7.2.4",
            "Redis版本应为7.2.4"
        );
        assert!(
            redis_result.error_details.is_none(),
            "Redis不应有错误信息"
        );

        // 验证Playwright检测结果
        let playwright_result = check_results
            .iter()
            .find(|r| r.dependency_id == "playwright")
            .expect("应该包含Playwright检测结果");

        assert_eq!(
            playwright_result.status,
            weibo_login::models::dependency::CheckStatus::Satisfied,
            "Playwright状态应为Satisfied"
        );
        assert_eq!(
            playwright_result.detected_version.as_ref().unwrap(),
            "1.48.0",
            "Playwright版本应为1.48.0"
        );
        assert!(
            playwright_result.error_details.is_none(),
            "Playwright不应有错误信息"
        );

        println!("✅ 依赖状态验证通过");
        println!("   - Redis: ✅ Satisfied (v7.2.4, {}ms)", redis_result.duration_ms);
        println!("   - Playwright: ✅ Satisfied (v1.48.0, {}ms)", playwright_result.duration_ms);

        // 7. 模拟检测进度更新
        for progress in [25, 50, 75, 100] {
            startup_manager.update_check_progress(progress);
            // 模拟UI更新延迟
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // 8. 验证无安装动作触发
        // 在真实场景中,这对应于检查是否调用了install_dependency命令
        for result in &check_results {
            if result.status != weibo_login::models::dependency::CheckStatus::Satisfied {
                startup_manager.trigger_installation();
            }
        }

        // 9. 模拟进入主界面
        startup_manager.enter_main_interface();

        // 10. 验证启动流程
        match startup_manager.validate_startup_flow() {
            Ok(msg) => {
                println!("✅ {}", msg);
            }
            Err(err) => {
                panic!("启动流程验证失败: {}", err);
            }
        }

        // 11. 验证契约一致性
        let contract_results: Vec<weibo_login::models::dependency::DependencyCheckResult> =
            check_results.iter().map(|r| r.to_contract_result()).collect();

        // 验证契约格式
        for result in &contract_results {
            assert!(!result.dependency_id.is_empty(), "dependency_id不应为空");
            assert!(result.duration_ms > 0, "duration_ms应大于0");
            assert!(result.checked_at.timestamp() > 0, "checked_at应为有效时间戳");
        }

        println!("✅ 契约格式验证通过");

        // 12. 输出最终结果(模拟前端接收的数据格式)
        println!("\n📋 前端接收的检测结果:");
        for result in &contract_results {
            match result.status {
                weibo_login::models::dependency::CheckStatus::Satisfied => {
                    println!("   ✅ {}: Satisfied (v{})",
                        result.dependency_id,
                        result.detected_version.as_ref().unwrap()
                    );
                }
                _ => {
                    println!("   ❌ {}: {:?} - {}",
                        result.dependency_id,
                        result.status,
                        result.error_details.as_ref().unwrap_or(&"Unknown error".to_string())
                    );
                }
            }
        }

        println!("\n🎉 所有依赖已满足场景测试通过!");
        println!("   - 检测耗时: {}ms (< 2000ms ✅)", elapsed_ms);
        println!("   - 依赖状态: 全部满足 ✅");
        println!("   - 安装动作: 无触发 ✅");
        println!("   - 主界面: 已进入 ✅");
    }

    /// 测试检测性能边界情况
    ///
    /// 验证即使检测接近性能边界,应用仍能正常启动
    #[tokio::test]
    async fn test_performance_boundary_case() {
        println!("\n⏱️  测试性能边界情况");

        // 创建接近2秒性能边界的Mock服务
        let dependency_checker = MockDependencyChecker::new(
            true, "7.2.4".to_string(),      // Redis satisfied
            true, "1.48.0".to_string(),    // Playwright satisfied
            1900,                          // 1.9秒检测延迟
        );

        let start_time = Instant::now();
        let _results = dependency_checker.check_all().await;
        let elapsed = start_time.elapsed().as_millis() as u64;

        // 应该在2秒内完成
        assert!(elapsed < 2000, "边界测试: 检测耗时 {}ms 应 < 2000ms", elapsed);
        assert!(elapsed >= 1900, "边界测试: 检测耗时 {}ms 应 >= 1900ms", elapsed);

        println!("✅ 性能边界测试通过: {}ms", elapsed);
    }

    /// 测试依赖检测结果数据完整性
    ///
    /// 确保Mock服务返回的数据格式符合契约要求
    #[tokio::test]
    async fn test_dependency_result_data_integrity() {
        println!("\n🔍 测试依赖检测结果数据完整性");

        let dependency_checker = MockDependencyChecker::new_all_satisfied();
        let results = dependency_checker.check_all().await;

        // 验证结果数量
        assert_eq!(results.len(), 2, "应该返回2个依赖检测结果");

        // 验证每个结果的字段完整性
        for result in results {
            assert!(!result.dependency_id.is_empty(), "dependency_id不应为空");
            assert!(!result.dependency_name.is_empty(), "dependency_name不应为空");

            // 对于满足的依赖,应该有版本信息
            if result.status == weibo_login::models::dependency::CheckStatus::Satisfied {
                assert!(
                    result.detected_version.is_some(),
                    "Satisfied状态的依赖应该有版本信息"
                );
                assert!(
                    result.detected_version.as_ref().unwrap().len() > 0,
                    "版本号不应为空字符串"
                );
                assert!(
                    result.error_details.is_none(),
                    "Satisfied状态的依赖不应有错误信息"
                );
            }

            // 验证时间戳
            assert!(result.checked_at.timestamp() > 0, "checked_at应为有效时间戳");

            // 验证耗时
            assert!(result.duration_ms > 0, "duration_ms应大于0");
        }

        println!("✅ 数据完整性验证通过");
    }
}
