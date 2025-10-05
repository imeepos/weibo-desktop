#!/bin/bash
# 运行所有测试的便捷脚本
#
# 用法: bash run_tests.sh
#
# 遵循优雅即简约原则,提供清晰的测试执行流程和结果输出

set -e

echo "🧪 运行微博扫码登录测试套件"
echo "=============================="
echo ""

# 切换到 src-tauri 目录
cd "$(dirname "$0")/src-tauri"

echo "📍 当前目录: $(pwd)"
echo ""

# 1. 契约测试
echo "1️⃣  契约测试 (Contract Tests)"
echo "------------------------------"
echo "验证 Tauri 命令 API 契约..."
cargo test --test contract_save_cookies --quiet
cargo test --test contract_query_cookies --quiet
echo "✅ 契约测试通过"
echo ""

# 2. 单元测试
echo "2️⃣  单元测试 (Unit Tests)"
echo "------------------------------"
echo "验证数据模型业务逻辑..."
cargo test --test models_test --quiet
echo "✅ 单元测试通过"
echo ""

# 3. 集成测试
echo "3️⃣  集成测试 (Integration Tests)"
echo "------------------------------"
echo "验证端到端业务流程..."
cargo test --test integration_test --quiet
echo "✅ 集成测试通过"
echo ""

# 4. 性能测试
echo "4️⃣  性能测试 (Performance Tests)"
echo "------------------------------"
echo "验证性能指标 (显示详细输出)..."
cargo test --test performance_test -- --nocapture
echo "✅ 性能测试通过"
echo ""

# 5. 运行所有集成测试
echo "5️⃣  完整测试套件"
echo "------------------------------"
echo "运行所有集成测试 (不含文档测试)..."
cargo test --tests --quiet
echo "✅ 完整测试套件通过"
echo ""

echo "=============================="
echo "🎉 所有测试通过!"
echo ""
echo "📊 测试统计:"
echo "  - 契约测试: 2 个文件 (20+ 测试)"
echo "  - 单元测试: 1 个文件 (35 个测试)"
echo "  - 集成测试: 1 个文件 (18 个测试)"
echo "  - 性能测试: 1 个文件 (18 个测试)"
echo "  - 总计: 91+ 测试场景"
echo ""
echo "📖 查看详细文档: src-tauri/tests/README.md"
echo ""
echo "💡 提示:"
echo "  - 运行单个测试: cargo test test_complete_login_flow"
echo "  - 查看详细输出: cargo test -- --nocapture"
echo "  - 运行特定文件: cargo test --test integration_test"
