# 测试文档

本目录包含微博扫码登录功能的所有测试。

## 测试分类

### 1. 契约测试 (Contract Tests)
验证Tauri命令符合API契约定义。

**文件**:
- `contract_save_cookies.rs` - save_cookies命令测试
- `contract_query_cookies.rs` - query_cookies命令测试

**运行**:
```bash
cargo test --test contract_save_cookies
cargo test --test contract_query_cookies
```

### 2. 单元测试 (Unit Tests)
测试数据模型的业务逻辑。

**文件**:
- `models_test.rs` - LoginSession, CookiesData等模型测试

**运行**:
```bash
cargo test --test models_test
```

### 3. 集成测试 (Integration Tests)
测试完整的端到端流程。

**文件**:
- `integration_test.rs` - 完整登录流程测试

**测试场景**:
- ✅ 完整登录流程 (生成二维码 -> 扫码 -> 验证 -> 保存)
- ✅ 二维码过期场景
- ✅ 多账户并发登录
- ✅ 网络中断恢复
- ✅ Redis故障恢复
- ✅ 登录会话状态转换
- ✅ Cookies验证失败场景
- ✅ 数据序列化和反序列化

**运行**:
```bash
cargo test --test integration_test
```

### 4. 性能测试 (Performance Tests)
验证性能指标。

**文件**:
- `performance_test.rs` - Redis操作性能、并发性能

**性能指标**:
- Redis保存: < 100ms (P95)
- Redis查询: < 50ms (P95)
- Cookies验证: < 2s (P95)
- 序列化/反序列化: < 10ms (P95)
- 并发支持: 50个并发请求在5秒内完成
- 大规模并发: 200个并发请求在10秒内完成

**运行**:
```bash
cargo test --test performance_test
```

## 运行所有测试

```bash
# 运行所有测试
cargo test

# 显示输出
cargo test -- --nocapture

# 运行特定测试
cargo test test_complete_login_flow

# 并行运行
cargo test -- --test-threads=4

# 运行单个测试文件
cargo test --test integration_test -- --nocapture
```

## Mock服务

所有测试使用Mock服务,无需外部依赖:
- `MockRedisService` - 内存Redis
  - 支持: SET/GET, HSET/HGET/HGETALL, EXISTS, DEL
  - 故障模拟: set_fail_mode()
  - 数据清理: clear()

- `MockValidationService` - Playwright验证Mock
  - 成功模式: new_success()
  - 失败模式: new_failure()
  - 自定义模式: new(succeed, uid, screen_name)

## 测试覆盖率

生成覆盖率报告 (需要安装tarpaulin):
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## 性能基准

参考 `specs/001-cookies/quickstart.md` 中的性能要求:
- 二维码生成: < 500ms
- 轮询延迟: < 1s
- Cookies验证: < 2s
- Redis操作: < 100ms (P95)

## 测试策略

### 契约测试
- 验证API签名正确
- 验证输入验证逻辑
- 验证返回值格式

### 单元测试
- 验证数据模型验证逻辑
- 验证字段必需性
- 验证序列化/反序列化

### 集成测试
- 验证完整业务流程
- 验证错误恢复机制
- 验证并发场景
- 验证数据一致性

### 性能测试
- 验证P95延迟
- 验证并发能力
- 验证内存使用
- 验证序列化性能

## 测试原则

遵循 `.specify/memory/constitution.md` 中的宪章原则:

1. **存在即合理**: 每个测试验证真实场景,无冗余测试
2. **优雅即简约**: 测试代码清晰易懂,自解释
3. **性能即艺术**: 性能测试验证真实性能指标
4. **错误处理哲学**: 测试错误恢复和容错机制

## 快速开始

```bash
# 1. 运行所有测试
cargo test

# 2. 查看性能测试结果
cargo test --test performance_test -- --nocapture

# 3. 运行集成测试
cargo test --test integration_test -- --nocapture

# 4. 使用测试脚本 (推荐)
bash run_tests.sh
```

## 常见问题

**Q: 为什么测试不需要真实的Redis?**
A: 我们使用MockRedisService,它在内存中模拟Redis,速度快且无外部依赖。

**Q: 性能测试是否准确?**
A: 性能测试使用Mock服务,主要测试业务逻辑性能。真实环境需要集成真实Redis。

**Q: 如何调试失败的测试?**
A: 使用 `-- --nocapture` 参数查看println输出,或使用Rust调试器。

**Q: 测试覆盖率如何?**
A: 使用 `cargo tarpaulin` 生成覆盖率报告,目标覆盖率 > 80%。

## 测试目录结构

```
tests/
├── README.md                    # 本文档
├── common/
│   └── mod.rs                   # Mock服务和测试工具
├── contract_save_cookies.rs     # save_cookies契约测试
├── contract_query_cookies.rs    # query_cookies契约测试
├── models_test.rs               # 数据模型单元测试
├── integration_test.rs          # 端到端集成测试
└── performance_test.rs          # 性能测试
```

## 持续集成

在CI/CD流程中,建议:
1. 运行所有测试: `cargo test`
2. 生成覆盖率报告: `cargo tarpaulin`
3. 验证性能指标: `cargo test --test performance_test`
4. 验证契约: `cargo test --test contract_*`

## 贡献指南

添加新测试时:
1. 遵循现有测试风格
2. 使用Mock服务避免外部依赖
3. 添加清晰的文档注释
4. 验证测试覆盖新场景
5. 确保测试可重复执行

## 参考资料

- [Rust测试文档](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio异步测试](https://tokio.rs/tokio/topics/testing)
- [项目规范文档](../specs/001-cookies/quickstart.md)
