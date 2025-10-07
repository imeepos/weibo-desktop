# Quickstart: 微博关键字增量爬取

**Feature**: 003- | **Date**: 2025-10-07
**Purpose**: 端到端验收测试场景,验证所有功能需求

---

## 测试环境准备

### 前置条件

1. **Redis运行**:
   ```bash
   docker compose up redis -d
   redis-cli ping  # 验证连接
   ```

2. **Playwright server运行**:
   ```bash
   cd playwright
   pnpm install
   pnpm run server  # 启动WebSocket服务器 (ws://localhost:9223)
   ```

3. **已有有效cookies** (通过001-cookies功能):
   ```bash
   # 假设已有测试账号的cookies
   redis-cli HGET weibo:cookies:1234567890 screen_name
   # 输出: "测试用户"
   ```

4. **Tauri应用编译并运行**:
   ```bash
   cd src-tauri
   cargo build
   cargo tauri dev
   ```

---

## 场景1: 创建爬取任务

### 目标
验证FR-001至FR-005 (任务管理)

### 步骤

1. **打开应用,进入爬取页面**

2. **点击"新建任务"按钮**

3. **填写任务表单**:
   - 关键字: `国庆`
   - 事件开始时间: `2025-10-01 00:00:00`
   - 选择账号: `测试用户 (1234567890)`

4. **点击"创建"按钮**

### 预期结果

- ✅ 任务创建成功,显示任务ID
- ✅ 任务列表中出现新任务
- ✅ 任务状态为 `Created`
- ✅ Redis中存在任务记录:
  ```bash
  redis-cli HGET crawl:task:{task_id} status
  # 输出: "Created"
  ```

### 边界测试

- [ ] 关键字为空时,显示错误提示
- [ ] 事件开始时间是未来时间时,显示错误提示
- [ ] 选择的账号cookies过期时,显示"请重新登录"提示

---

## 场景2: 启动历史回溯并监听进度

### 目标
验证FR-006至FR-009 (历史回溯)、FR-019至FR-020 (进度追踪)

### 步骤

1. **在任务列表中选择刚创建的任务**

2. **点击"开始爬取"按钮**

3. **观察进度展示**:
   - 当前时间范围
   - 当前页码
   - 已爬取帖子数
   - 任务状态变化

4. **等待至少3个分页完成** (约1分钟)

### 预期结果

- ✅ 任务状态转换: `Created` → `HistoryCrawling`
- ✅ 前端实时接收 `crawl-progress` 事件
- ✅ 进度条正常更新
- ✅ Redis中存在检查点:
  ```bash
  redis-cli HGETALL crawl:checkpoint:{task_id}
  # 输出:
  # shard_start_time: "1696118400"
  # current_page: "3"
  # direction: "Backward"
  ```
- ✅ Redis中存在帖子数据:
  ```bash
  redis-cli ZCARD crawl:posts:{task_id}
  # 输出: 60 (假设每页20条,3页共60条)
  ```
- ✅ 帖子ID已去重:
  ```bash
  redis-cli SCARD crawl:post_ids:{task_id}
  # 输出: 60 (假设无重复)
  ```

### 边界测试

- [ ] 检测到50页限制时,自动进行时间分片
- [ ] 某时间范围无结果时,正确跳过该分片
- [ ] 日志记录时间分片操作

---

## 场景3: 暂停并恢复任务

### 目标
验证FR-004 (暂停/恢复)、FR-013至FR-015 (断点续爬)

### 步骤

1. **在任务运行中点击"暂停"按钮**

2. **验证暂停状态**:
   - 任务状态为 `Paused`
   - 停止接收进度事件
   - 显示检查点信息 (当前时间范围、当前页码)

3. **等待30秒**

4. **点击"恢复"按钮**

5. **观察恢复后的行为**:
   - 从检查点的下一页继续爬取
   - 不重复已爬取的数据

### 预期结果

- ✅ 暂停后任务状态: `HistoryCrawling` → `Paused`
- ✅ 恢复后任务状态: `Paused` → `HistoryCrawling`
- ✅ Redis检查点保存正确:
  ```bash
  redis-cli HGET crawl:checkpoint:{task_id} current_page
  # 输出: "15" (暂停时的页码)
  ```
- ✅ 恢复后从第16页继续爬取
- ✅ 帖子数量持续增加,无重复

### 边界测试

- [ ] 在第1页暂停,恢复后从第2页开始
- [ ] 在最后一页暂停,恢复后正确完成任务
- [ ] 多次暂停恢复,每次都从正确位置继续

---

## 场景4: 完成历史回溯

### 目标
验证FR-008 (历史回溯完成标记)

### 步骤

1. **等待历史回溯完成** (或手动设置较短的时间范围加速测试)

2. **观察状态变化**

3. **验证数据完整性**

### 预期结果

- ✅ 任务状态转换: `HistoryCrawling` → `HistoryCompleted`
- ✅ 前端接收 `crawl-completed` 事件
- ✅ Redis任务数据更新:
  ```bash
  redis-cli HGET crawl:task:{task_id} status
  # 输出: "HistoryCompleted"

  redis-cli HGET crawl:task:{task_id} min_post_time
  # 输出: "1696118400" (接近event_start_time)
  ```
- ✅ 帖子时间范围正确:
  ```bash
  # 最小帖子时间应接近event_start_time
  redis-cli ZRANGE crawl:posts:{task_id} 0 0 WITHSCORES
  # 输出: ... "1696118400"

  # 最大帖子时间应接近任务创建时间
  redis-cli ZRANGE crawl:posts:{task_id} -1 -1 WITHSCORES
  # 输出: ... "1696204800"
  ```

### 边界测试

- [ ] 事件开始时间到现在只有1小时,任务正常完成
- [ ] 某时间范围内无任何帖子,任务仍能完成

---

## 场景5: 启动增量更新

### 目标
验证FR-010至FR-012 (增量更新)

### 步骤

1. **在历史回溯完成后,点击"启动增量更新"按钮**

2. **等待10秒** (模拟新帖子产生)

3. **手动在微博发布包含关键字的帖子** (可选,测试环境可跳过)

4. **观察增量爬取行为**

### 预期结果

- ✅ 任务状态转换: `HistoryCompleted` → `IncrementalCrawling`
- ✅ 爬取方向为 `Forward`
- ✅ 仅爬取最大时间后的帖子:
  ```bash
  # 检查新帖子的时间戳大于max_post_time
  redis-cli ZRANGE crawl:posts:{task_id} -10 -1 WITHSCORES
  # 输出: 最新10条帖子,时间戳应大于之前的max_post_time
  ```
- ✅ 日志显示: `增量爬取: 从{max_post_time}到{now}`

### 边界测试

- [ ] 最大时间到现在无新帖子,正常完成
- [ ] 增量爬取中检测到验证码,暂停任务

---

## 场景6: 导出数据

### 目标
验证FR-021至FR-023 (数据导出)

### 步骤

1. **在任务列表中选择已完成的任务**

2. **点击"导出数据"按钮**

3. **选择导出格式**:
   - 测试JSON格式
   - 测试CSV格式

4. **指定时间范围** (可选):
   - 开始时间: `2025-10-01 00:00:00`
   - 结束时间: `2025-10-03 23:59:59`

5. **点击"确认导出"**

### 预期结果

- ✅ 显示导出成功消息
- ✅ 返回文件路径
- ✅ 文件保存到下载目录:
  ```bash
  ls ~/Downloads/weibo_*
  # 输出: weibo_550e8400_1696204800.json
  ```
- ✅ JSON格式正确:
  ```bash
  cat ~/Downloads/weibo_550e8400_1696204800.json | jq '.posts | length'
  # 输出: 12345
  ```
- ✅ CSV格式正确:
  ```bash
  wc -l ~/Downloads/weibo_550e8400_1696204800.csv
  # 输出: 12346 (包含标题行)
  ```
- ✅ 时间范围过滤生效 (如果指定)

### 边界测试

- [ ] 无数据时导出,显示错误提示
- [ ] 导出超大数据集 (100万条),不超时,不内存溢出
- [ ] CSV中包含特殊字符 (换行、逗号、引号),正确转义

---

## 场景7: 异常处理

### 目标
验证FR-024至FR-026 (错误处理)

### 7.1 验证码检测

**步骤**:
1. 模拟触发验证码 (通过快速连续请求)
2. 观察系统行为

**预期结果**:
- ✅ 任务自动暂停,状态: `HistoryCrawling` → `Paused`
- ✅ 前端接收 `crawl-error` 事件:
  ```typescript
  {
    errorCode: "CAPTCHA_DETECTED",
    error: "检测到验证码,需要人工处理"
  }
  ```
- ✅ 日志记录验证码截图路径
- ✅ 用户可手动处理后恢复任务

### 7.2 网络错误

**步骤**:
1. 停止Playwright server
2. 尝试启动任务

**预期结果**:
- ✅ 任务状态: `HistoryCrawling` → `Failed`
- ✅ 失败原因记录: `"Playwright服务器未运行"`
- ✅ 用户可在恢复服务后重试

### 7.3 Redis连接失败

**步骤**:
1. 停止Redis服务
2. 尝试创建任务

**预期结果**:
- ✅ 返回错误: `STORAGE_ERROR`
- ✅ 错误消息: `"Redis连接失败: ..."`

---

## 场景8: 清理测试数据

### 步骤

1. **删除测试任务**:
   ```bash
   redis-cli DEL crawl:task:{task_id}
   redis-cli DEL crawl:posts:{task_id}
   redis-cli DEL crawl:post_ids:{task_id}
   redis-cli DEL crawl:checkpoint:{task_id}
   ```

2. **删除导出文件**:
   ```bash
   rm ~/Downloads/weibo_*.json
   rm ~/Downloads/weibo_*.csv
   ```

---

## 性能验证

### 百万级数据存储

**测试**:
1. 创建任务,爬取热门关键字 (如"春节")
2. 运行至少24小时
3. 累积至少100万条帖子

**预期**:
- ✅ Redis内存占用 <2GB
- ✅ 帖子查询按时间范围 <100ms
- ✅ 导出100万条数据 <30秒

### 断点续爬精确性

**测试**:
1. 创建任务并启动
2. 随机在不同页数暂停 (第3页、第20页、第45页)
3. 恢复任务
4. 验证无重复数据

**预期**:
- ✅ 恢复后从正确页码继续
- ✅ 帖子ID去重100%有效

---

## 验收标准

所有场景的"预期结果"必须全部满足,所有边界测试必须通过。

**通过条件**:
- [ ] 场景1至8全部通过
- [ ] 性能验证达标
- [ ] 无未处理的异常或错误日志
- [ ] 用户体验流畅,响应及时

**失败重测**:
- 如果任何场景失败,修复后必须重新执行完整测试流程
- 回归测试必须覆盖所有场景

---

## 自动化测试脚本

可将此quickstart转换为自动化测试:

```rust
#[cfg(test)]
mod quickstart_tests {
    use super::*;

    #[tokio::test]
    async fn test_scenario_1_create_task() {
        // 实现场景1的自动化测试
    }

    #[tokio::test]
    async fn test_scenario_2_start_crawl() {
        // 实现场景2的自动化测试
    }

    // ... 其他场景
}
```

或使用Playwright E2E测试:

```typescript
// tests/e2e/crawl-quickstart.spec.ts
import { test, expect } from '@playwright/test';

test('Scenario 1: Create crawl task', async ({ page }) => {
  // 实现场景1的E2E测试
});

// ... 其他场景
```

---

## 故障排除

### 问题: 任务一直停留在Created状态

**排查**:
1. 检查Playwright server是否运行
2. 检查Redis连接
3. 查看后端日志

### 问题: 进度事件未推送到前端

**排查**:
1. 检查Tauri事件监听器
2. 检查后台任务是否启动成功
3. 查看浏览器控制台日志

### 问题: 帖子数量不增长

**排查**:
1. 检查时间范围内是否有帖子
2. 检查微博搜索API是否返回数据
3. 检查Playwright脚本解析逻辑

---

**测试完成**: 所有场景通过后,功能验收完成,可进入生产环境部署。
