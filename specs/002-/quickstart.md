# Quickstart: 启动时依赖检测与自动安装 - 集成测试

**功能**: 002-依赖检测与自动安装
**测试目的**: 验证完整用户场景的端到端功能
**执行环境**: Desktop应用(Tauri)
**测试数据**: 基于 `data-model.md` 和 `contracts/`

---

## 测试准备

### 前置条件

1. **测试环境配置**
   ```bash
   # 克隆项目
   cd /workspace/desktop

   # 安装依赖
   pnpm install
   cd src-tauri && cargo build
   ```

2. **启动Redis**(用于检测结果缓存)
   ```bash
   docker run -d -p 6379:6379 redis:7-alpine
   ```

3. **配置测试依赖清单**

   创建 `src-tauri/tests/fixtures/test_dependencies.toml`:
   ```toml
   [[dependencies]]
   id = "redis"
   name = "Redis Server"
   version_requirement = ">=7.0.0"
   description = "内存数据库"
   level = "required"
   auto_installable = false
   install_priority = 1

   [dependencies.check_method]
   type = "service"
   host = "localhost"
   port = 6379

   [[dependencies]]
   id = "playwright"
   name = "Playwright"
   version_requirement = ">=1.40.0"
   description = "浏览器自动化"
   level = "optional"
   auto_installable = true
   install_priority = 5
   install_command = "pnpm install playwright"

   [dependencies.check_method]
   type = "executable"
   name = "npx"
   version_args = ["playwright", "--version"]
   ```

---

## 测试场景

### 场景1: 所有依赖满足 - 快速启动

**用户故事**: 作为用户,当所有必需依赖都已安装且版本兼容时,我希望应用快速启动并进入主界面,无需等待安装过程。

**前置条件**:
- Redis 7.2.x 运行在 localhost:6379
- Playwright 1.40+ 已安装在 node_modules

**执行步骤**:

1. **启动应用**
   ```bash
   pnpm tauri dev
   ```

2. **观察启动界面**
   - 应显示"正在检测依赖..."文本
   - 进度条显示 0% → 100%
   - 检测项目显示:
     - ✅ Redis Server (v7.2.4)
     - ✅ Playwright (v1.48.0)

3. **验证事件流**(前端控制台)
   ```typescript
   // 预期看到4个事件
   [1/2] Redis Server: Checking
   [1/2] Redis Server: Completed
   [2/2] Playwright: Checking
   [2/2] Playwright: Completed
   ```

4. **验证最终结果**
   - 检测耗时 < 2秒
   - 自动跳转到主界面
   - 无错误提示

**预期输出**:

```json
// check_dependencies 返回结果
[
  {
    "dependency_id": "redis",
    "checked_at": "2025-10-05T10:30:15.123Z",
    "status": "satisfied",
    "detected_version": "7.2.4",
    "duration_ms": 45
  },
  {
    "dependency_id": "playwright",
    "checked_at": "2025-10-05T10:30:15.168Z",
    "status": "satisfied",
    "detected_version": "1.48.0",
    "duration_ms": 32
  }
]
```

**成功标准**:
- ✅ 进度条流畅更新
- ✅ 检测时间 < 2秒
- ✅ 无错误日志
- ✅ 自动进入主界面

---

### 场景2: 缺失可自动安装依赖 - 自动安装

**用户故事**: 作为用户,当缺失可选依赖Playwright时,我希望应用自动下载安装,并通过进度条展示安装进度,安装成功后自动继续启动。

**前置条件**:
- Redis 7.2.x 运行中
- Playwright **未安装** (删除 node_modules/playwright)

**执行步骤**:

1. **删除Playwright**
   ```bash
   rm -rf node_modules/playwright
   ```

2. **启动应用**
   ```bash
   pnpm tauri dev
   ```

3. **观察检测阶段**
   - 进度条显示"正在检测依赖 [1/2]"
   - Redis: ✅ 已安装
   - Playwright: ❌ 缺失

4. **观察安装阶段**
   - 界面切换到"正在准备运行环境"
   - 进度条显示"正在安装 Playwright"
   - 进度: 0% → 15%(下载) → 65%(安装) → 100%(完成)

5. **验证安装事件流**
   ```typescript
   // installation-progress 事件
   { task_id: "...", status: "downloading", progress_percent: 15 }
   { task_id: "...", status: "installing", progress_percent: 65 }
   { task_id: "...", status: "success", progress_percent: 100 }
   ```

6. **验证重新检测**
   - 安装完成后自动触发重新检测
   - Playwright状态变为 ✅ 已安装

**预期输出**:

```json
// install_dependency 返回的任务
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "dependency_id": "playwright",
  "created_at": "2025-10-05T10:31:00.000Z",
  "started_at": "2025-10-05T10:31:01.000Z",
  "completed_at": "2025-10-05T10:31:45.000Z",
  "status": "success",
  "progress_percent": 100,
  "error_message": null,
  "install_log": [
    "Downloading Playwright v1.48.0...",
    "Installing browsers...",
    "Installation complete"
  ],
  "error_type": null
}
```

**成功标准**:
- ✅ 检测到缺失依赖
- ✅ 自动触发安装(无需用户确认,可选依赖)
- ✅ 进度条实时更新
- ✅ 安装成功后重新检测
- ✅ 最终进入主界面

---

### 场景3: 缺失必需依赖(需手动安装) - 显示安装指引

**用户故事**: 作为用户,当缺失不可自动安装的必需依赖Redis时,我希望看到清晰的安装指引,包括下载链接和安装步骤,并能点击"重新检测"按钮验证安装。

**前置条件**:
- Redis **未运行** (docker stop redis)
- Playwright 已安装

**执行步骤**:

1. **停止Redis**
   ```bash
   docker stop $(docker ps -q --filter ancestor=redis:7-alpine)
   ```

2. **启动应用**
   ```bash
   pnpm tauri dev
   ```

3. **观察检测结果**
   - 进度条完成检测
   - Redis: ❌ 缺失(必需依赖)
   - Playwright: ✅ 已安装

4. **验证安装指引界面**

   界面应显示:
   ```
   ⚠️ 缺少必需依赖

   Redis Server
   用途: 内存数据库,用于存储用户会话和缓存数据

   安装指引:
   ## 安装Redis Server

   ### 方式1: Docker (推荐)
   docker run -d -p 6379:6379 redis:7-alpine

   ### 方式2: 手动安装
   1. 访问 https://redis.io/download
   2. 下载适合您操作系统的版本
   3. 按照官方文档完成安装
   4. 启动Redis服务: redis-server

   [重新检测] [查看详细日志] [退出应用]
   ```

5. **手动启动Redis**
   ```bash
   docker run -d -p 6379:6379 redis:7-alpine
   ```

6. **点击"重新检测"按钮**
   - 触发 `trigger_manual_check` command
   - 进度条重新运行
   - Redis状态更新为 ✅ 已安装

7. **验证进入主界面**

**预期输出**:

```json
// 初次检测结果
{
  "dependency_id": "redis",
  "checked_at": "2025-10-05T10:35:00.000Z",
  "status": "missing",
  "detected_version": null,
  "error_details": "Redis service not reachable at localhost:6379"
}

// InstallationGuide 数据
{
  "dependency_id": "redis",
  "dependency_name": "Redis Server",
  "title": "安装Redis Server",
  "content": "## 安装Redis Server\n\n### 方式1: Docker...",
  "links": [
    { "text": "Redis官网", "url": "https://redis.io/download" }
  ],
  "target_os": [],
  "language": "zh-CN"
}

// 重新检测后
{
  "dependency_id": "redis",
  "checked_at": "2025-10-05T10:36:15.000Z",
  "status": "satisfied",
  "detected_version": "7.2.4"
}
```

**成功标准**:
- ✅ 检测到必需依赖缺失
- ✅ **阻止**进入主界面
- ✅ 显示Markdown格式的安装指引
- ✅ 提供可点击的下载链接
- ✅ "重新检测"按钮可用
- ✅ 手动安装后重新检测成功

---

### 场景4: 权限不足导致安装失败 - 提示管理员权限

**用户故事**: 作为用户,当自动安装因权限不足失败时,我希望看到明确的错误提示,指导我以管理员身份重启应用。

**前置条件**:
- 模拟权限错误(修改安装脚本返回权限错误)
- Playwright缺失

**执行步骤**:

1. **配置权限失败模拟**

   修改测试配置注入权限错误:
   ```rust
   // 测试时设置环境变量
   std::env::set_var("SIMULATE_PERMISSION_ERROR", "true");
   ```

2. **启动应用**

3. **观察安装失败**
   - 安装进度到35%时失败
   - 进度条变红色
   - 显示错误提示

4. **验证错误界面**
   ```
   ❌ 安装失败

   Playwright 安装失败
   错误原因: 权限不足

   解决方案:
   请以管理员身份打开应用:

   Windows: 右键应用图标 → "以管理员身份运行"
   macOS: 使用 sudo 命令启动
   Linux: sudo ./app

   [重启应用] [查看日志] [稍后安装]
   ```

**预期输出**:

```json
{
  "task_id": "...",
  "dependency_id": "playwright",
  "status": "failed",
  "progress_percent": 35,
  "error_message": "Permission denied: cannot write to /usr/local/lib",
  "error_type": "permission_error",
  "install_log": [
    "Downloading Playwright...",
    "Extracting files...",
    "ERROR: Permission denied"
  ]
}
```

**成功标准**:
- ✅ 捕获权限错误
- ✅ 分类为 `PermissionError`
- ✅ 显示平台特定的管理员权限指引
- ✅ 提供"重启应用"按钮
- ✅ 日志记录完整错误堆栈

---

### 场景5: 用户运行期间手动触发检测

**用户故事**: 作为用户,当我在应用运行期间手动安装了新的依赖后,我希望能点击"刷新依赖状态"按钮重新检测,而无需重启应用。

**前置条件**:
- 应用正常运行
- 初始检测显示Playwright缺失

**执行步骤**:

1. **应用已启动**,主界面显示

2. **打开设置页面**
   - 导航到 设置 → 依赖管理

3. **查看当前依赖状态**
   ```
   依赖项列表:
   ✅ Redis Server (v7.2.4)
   ❌ Playwright (未安装)
   ```

4. **在外部终端安装Playwright**
   ```bash
   pnpm install playwright
   ```

5. **点击"刷新依赖状态"按钮**
   - 触发 `trigger_manual_check` command
   - 显示检测进度覆盖层
   - 进度条 0% → 100%

6. **验证状态更新**
   ```
   依赖项列表:
   ✅ Redis Server (v7.2.4)
   ✅ Playwright (v1.48.0) ← 状态已更新
   ```

**预期输出**:

```typescript
// 前端调用
const results = await invoke('trigger_manual_check');

// 返回结果
[
  {
    "dependency_id": "redis",
    "status": "satisfied",
    "detected_version": "7.2.4"
  },
  {
    "dependency_id": "playwright",
    "status": "satisfied",      // 之前是 "missing"
    "detected_version": "1.48.0" // 之前是 null
  }
]
```

**成功标准**:
- ✅ 运行期间可触发检测
- ✅ 检测不阻塞主界面交互
- ✅ 状态实时更新到UI
- ✅ 缓存被正确更新
- ✅ 无需重启应用

---

## 性能验证

### 检测性能

| 场景 | 依赖数量 | 目标耗时 | 实际耗时 | 状态 |
|------|---------|---------|---------|------|
| 所有已安装 | 2 | < 2秒 | _待测_ | ⏳ |
| 部分缺失 | 2 | < 3秒 | _待测_ | ⏳ |
| 全部缺失 | 2 | < 5秒 | _待测_ | ⏳ |

### 安装性能

| 依赖 | 目标耗时 | 实际耗时 | 状态 |
|------|---------|---------|------|
| pnpm | < 30秒 | _待测_ | ⏳ |
| Playwright | < 120秒 | _待测_ | ⏳ |

### 内存占用

| 阶段 | 目标内存 | 实际内存 | 状态 |
|------|---------|---------|------|
| 检测阶段 | < 50MB | _待测_ | ⏳ |
| 并行安装 | < 100MB | _待测_ | ⏳ |

---

## 日志验证

### 检查日志文件

**日志位置**:
- Windows: `%APPDATA%\<app-name>\logs\dependency_check_2025-10-05.log`
- macOS: `~/Library/Application Support/<app-name>/logs/dependency_check_2025-10-05.log`
- Linux: `~/.local/share/<app-name>/logs/dependency_check_2025-10-05.log`

**日志内容验证**:

```bash
# 检查日志文件存在
ls -lh ~/.local/share/desktop-app/logs/

# 查看最新日志
tail -f ~/.local/share/desktop-app/logs/dependency_check_$(date +%Y-%m-%d).log
```

**预期日志格式** (JSON Lines):

```json
{"timestamp":"2025-10-05T10:30:15.123Z","level":"INFO","target":"dependency_checker","fields":{"message":"Starting dependency check","total_dependencies":2}}
{"timestamp":"2025-10-05T10:30:15.150Z","level":"INFO","target":"dependency_checker","fields":{"message":"Dependency check completed","dependency":"redis","status":"satisfied","version":"7.2.4"}}
{"timestamp":"2025-10-05T10:30:15.168Z","level":"WARN","target":"dependency_checker","fields":{"message":"Dependency check completed","dependency":"playwright","status":"missing"}}
{"timestamp":"2025-10-05T10:31:00.000Z","level":"INFO","target":"installer_service","fields":{"message":"Starting installation","dependency":"playwright","task_id":"550e8400-..."}}
{"timestamp":"2025-10-05T10:31:45.000Z","level":"INFO","target":"installer_service","fields":{"message":"Installation succeeded","dependency":"playwright","duration_ms":45000}}
```

**验证点**:
- ✅ 日志永久保留(无自动清理)
- ✅ JSON格式便于解析
- ✅ 包含完整上下文(dependency_id, version, error_type)
- ✅ 时间戳为UTC格式

---

## 自动化测试脚本

### 集成测试运行器

```bash
#!/bin/bash
# scripts/run-quickstart-tests.sh

set -e

echo "🚀 启动Quickstart集成测试..."

# 1. 准备环境
echo "📦 准备测试环境..."
docker run -d --name test-redis -p 6379:6379 redis:7-alpine
pnpm install

# 2. 场景1: 所有依赖满足
echo "✅ 测试场景1: 所有依赖满足"
pnpm tauri test scenario-1-all-satisfied

# 3. 场景2: 自动安装
echo "🔧 测试场景2: 自动安装可选依赖"
rm -rf node_modules/playwright
pnpm tauri test scenario-2-auto-install

# 4. 场景3: 手动安装指引
echo "📖 测试场景3: 显示安装指引"
docker stop test-redis
pnpm tauri test scenario-3-manual-guide

# 5. 清理
echo "🧹 清理测试环境..."
docker rm -f test-redis

echo "✨ 所有测试通过!"
```

---

## 验收标准总结

### 功能完整性

- ✅ FR-001: 启动时自动检测依赖
- ✅ FR-003: 在线自动安装可安装依赖
- ✅ FR-004: 进度条展示检测项目和结果
- ✅ FR-005: 显示手动安装指引
- ✅ FR-007: 日志永久保留
- ✅ FR-009: 运行期间手动触发检测
- ✅ FR-010: 必需依赖满足后进入主界面
- ✅ FR-012: 混合安装策略(必需串行、可选并行)
- ✅ FR-016: 权限不足提示管理员权限

### 用户体验

- ✅ 进度条流畅更新(>= 10 Hz)
- ✅ 无超时限制,等待检测完成
- ✅ 错误提示清晰、可操作
- ✅ 安装指引至少提供中文说明

### 技术质量

- ✅ 与现有技术栈吻合(Tauri/React/Rust)
- ✅ 无新增核心框架
- ✅ 遵循Constitution原则(简约、优雅、性能、错误处理)

---

**测试文档版本**: 1.0.0
**最后更新**: 2025-10-05
**测试覆盖率目标**: 80%+ (5个核心场景)
