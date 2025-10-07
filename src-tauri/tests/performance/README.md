# 性能测试

Redis性能验证测试套件，用于验证T039任务的性能指标。

## 测试环境要求

1. Redis 7.x 运行中
2. Rust 1.75+
3. 足够的内存和磁盘空间

## 快速开始

### 1. 启动Redis

```bash
# 使用Docker Compose
docker compose up redis -d

# 验证Redis运行状态
docker ps | grep redis
```

### 2. 配置Redis连接

编辑测试文件中的Redis URL：

```rust
// 根据环境选择合适的URL
let redis_url = "redis://localhost:6379";        // 本地Redis
// let redis_url = "redis://172.28.0.10:6379";   // Docker network
```

### 3. 运行测试

```bash
cd /home/ubuntu/worktrees/desktop/src-tauri

# 运行所有性能测试
cargo test --test perf_redis_performance -- --nocapture --ignored

# 运行特定测试
cargo test --test perf_redis_performance test_batch_write_performance -- --nocapture --ignored
```

## 测试用例

### 1. 批量写入性能 (`test_batch_write_performance`)
- **数据量**: 10万条
- **期望吞吐量**: > 10,000 posts/s
- **预计时间**: ~10秒

### 2. 时间范围查询性能 (`test_time_range_query_performance`)
- **数据量**: 100万条
- **测试范围**: 1小时、1天、7天、30天
- **期望查询时间**: < 100ms
- **预计时间**: ~10分钟

### 3. 导出性能 (`test_export_performance`)
- **数据量**: 100万条
- **格式**: JSON
- **期望总时间**: < 30秒
- **预计时间**: ~10分钟

### 4. 内存占用 (`test_redis_memory_usage`)
- **数据量**: 100万条
- **期望内存增长**: < 2GB
- **预计时间**: ~10分钟

### 5. Pipeline优化 (`test_pipeline_optimization`)
- **对比**: Pipeline批量 vs 逐条写入
- **期望提升**: > 10x
- **预计时间**: ~30秒

### 6. 端到端性能 (`test_end_to_end_performance`)
- **场景**: 7天历史回溯
- **数据量**: 50万条
- **预计时间**: ~5分钟

## 性能指标

| 指标 | 目标 | 测试用例 |
|------|------|---------|
| Redis内存占用 | < 2GB (100万条) | test_redis_memory_usage |
| 时间范围查询 | < 100ms | test_time_range_query_performance |
| 导出性能 | < 30秒 (100万条) | test_export_performance |
| 批量写入吞吐量 | > 10,000 posts/s | test_batch_write_performance |
| Pipeline提升 | > 10x | test_pipeline_optimization |

## 常见问题

### Q: Redis连接被拒绝

**症状**: `Connection refused (os error 111)`

**解决方案**:
1. 确认Redis运行: `docker ps | grep redis`
2. 尝试不同的Redis URL:
   - `redis://localhost:6379`
   - `redis://127.0.0.1:6379`
   - `redis://172.28.0.10:6379` (Docker network)
3. 检查端口映射: `docker port desktop-redis`

### Q: 测试运行很慢

**原因**: 100万条数据写入需要时间

**建议**:
- 先运行小数据量测试
- 使用 `--test-threads=1` 避免并发冲突
- 调整测试数据量（修改 `total_count` 变量）

### Q: 内存不足

**症状**: OOM或Redis内存警告

**解决方案**:
1. 调整Redis maxmemory配置
2. 减少测试数据量
3. 运行测试前清空Redis: `docker exec desktop-redis redis-cli FLUSHALL`

## 清理测试数据

```bash
# 清空所有Redis数据（谨慎使用！）
docker exec desktop-redis redis-cli FLUSHALL

# 清空特定任务的数据
docker exec desktop-redis redis-cli DEL "crawl:posts:perf_test_*"
docker exec desktop-redis redis-cli DEL "crawl:post_ids:perf_test_*"
```

## 详细报告

参见 [PERFORMANCE_TEST_REPORT.md](./PERFORMANCE_TEST_REPORT.md) 了解：
- 测试设计原理
- 性能优化建议
- 环境配置详解
- 故障排除指南
