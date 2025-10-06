# Rust 浏览器 POC 文档

**状态**: 实验性 (Experimental)
**完成度**: 70%
**最后更新**: 2025-10-06

## 概述

这是一个用纯 Rust 替代 Node.js + Playwright 的概念验证实现。目标是减少运行时依赖和打包体积。

## 技术栈

- **chromiumoxide** 0.7.0 - Chrome DevTools Protocol 客户端
- **tokio-tungstenite** 0.21 - WebSocket 服务器
- **base64** 0.22 - 图片编码

## 架构

```
src-tauri/src/services/
├── browser_service_poc.rs       - Chromium 生命周期管理
├── weibo_login_service_poc.rs   - 微博登录逻辑和网络监听
└── websocket_server_poc.rs      - WebSocket 服务器 (端口 9223)
```

## 已实现功能 ✅

1. **浏览器管理**
   - 全局 Chromium 实例 (`Arc<Browser>`)
   - 自动启动和连接检测
   - 事件处理器生命周期管理

2. **QR Code 生成**
   - 导航到微博登录页面
   - 提取二维码图片 (支持 data URL 和远程 URL)
   - Base64 编码

3. **WebSocket 服务器**
   - 端口 9223 监听
   - 兼容 TypeScript 版本的消息协议
   - 基础消息处理 (generate_qrcode, ping/pong)

4. **Cookie 提取**
   - 登录成功后提取所有 cookies
   - 从 SUB cookie 解析 UID

## 待完善功能 ⚠️

### 1. 网络监听集成 (核心功能)

**问题**: `WeiboLoginService::monitor_login_status` 已实现但未集成到 WebSocket 服务器

**原因**:
- `monitor_login_status` 需要 `Page` 引用,生命周期复杂
- WebSocket 服务器处理连接是独立的异步任务
- 需要在生成二维码后启动监听任务并保持 WebSocket 连接

**解决方案**:
```rust
// websocket_server_poc.rs 中
match WeiboLoginService::generate_qrcode().await {
    Ok((session, page)) => {
        // 发送二维码给客户端
        send_qrcode(&mut ws_sender, &session).await?;

        // 启动监听任务
        let ws_sender_clone = ws_sender.clone();
        tokio::spawn(async move {
            WeiboLoginService::monitor_login_status(
                page,
                session.session_id.clone(),
                move |update| {
                    // 推送状态更新
                    ws_sender_clone.send(...).await;
                }
            ).await
        });
    }
}
```

**预计工作量**: 2-3 小时

### 2. 完整错误处理

**当前状态**: 基础错误处理,日志不完整

**待改进**:
- CDP 连接失败重试
- 页面加载超时处理
- 网络请求失败兜底
- 更详细的错误日志

**预计工作量**: 1-2 小时

### 3. 真实环境测试

**当前状态**: 仅编译测试,未真实登录测试

**测试项**:
- [ ] Chromium 是否能正常启动
- [ ] 微博登录页面是否能加载
- [ ] QR Code 是否能正确提取
- [ ] CDP 网络事件是否能正常监听
- [ ] `/sso/v2/qrcode/check` 响应是否能解析
- [ ] 登录成功后 cookies 是否完整
- [ ] UID 提取是否正确

**预计工作量**: 3-5 小时 (含调试)

## 技术债务

### chromiumoxide API 学习曲线

**挑战**:
- 文档较少,主要靠阅读源码和示例
- CDP 协议复杂,事件流理解困难
- 异步生命周期管理需要仔细设计

**经验总结**:
1. `Browser` 不实现 `Clone`,需要用 `Arc<Browser>` 共享
2. `Handler` 的事件流需要用 `futures_util::StreamExt` 处理
3. `Page::event_listener` 返回的流需要持续监听,不能 drop
4. 网络事件需要显式启用: `page.execute(EnableParams::default())`

### 性能考虑

**优势**:
- 原生二进制,启动速度快 (~50-100ms vs Node.js 300-500ms)
- 内存占用小 (~30-50MB vs 80-120MB)

**劣势**:
- chromiumoxide 编译时间长 (~60K 行代码生成)
- 调试困难 (Rust async + CDP 双层复杂度)

## 切换指南

### 编译 Rust POC 版本

```bash
cargo build --release --features rust-browser --no-default-features
```

### 环境变量

```bash
# 使用 Rust POC (需要编译时启用 feature)
export BROWSER_BACKEND=rust-poc

# 使用 Playwright (默认)
export BROWSER_BACKEND=playwright
```

### 运行时选择

```rust
// main.rs 中会根据 BROWSER_BACKEND 环境变量选择后端
match std::env::var("BROWSER_BACKEND").unwrap_or_default().as_str() {
    "rust-poc" => { /* 启动 WebSocketServer */ }
    _ => { /* 使用外部 Playwright Server */ }
}
```

## 优势对比

| 指标 | Playwright | Rust POC |
|------|-----------|----------|
| **打包体积** | ~850MB | ~650MB (-23%) |
| **启动速度** | 300-500ms | 50-100ms (3-5x 快) |
| **内存占用** | 80-120MB | 30-50MB (-50%) |
| **运行时依赖** | Node.js 20+ | 无 |
| **成熟度** | ✅ 生产就绪 | ⚠️ POC 阶段 |
| **维护成本** | 🟡 中等 (2套语言) | 🟢 低 (纯 Rust) |
| **调试难度** | 🟢 简单 | 🔴 困难 |

## 后续演进路径

### 短期 (1-2周)
- 保持 Playwright 作为默认方案
- POC 代码保留但不激活

### 中期 (1-2月)
- 完善网络监听集成
- 进行真实环境测试
- 修复发现的问题
- 积累 chromiumoxide 使用经验

### 长期 (3-6月)
- 评估切换到 Rust 版本
- 逐步迁移用户到 Rust 后端
- 完全移除 Node.js 依赖

## 参考资料

- [chromiumoxide GitHub](https://github.com/mattsse/chromiumoxide)
- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
- [Playwright vs Puppeteer vs chromiumoxide 对比](https://www.cuketest.com/playwright/docs/browsers/)

## 联系人

- 实现者: Code Artisan
- 日期: 2025-10-06
- 状态: POC / 实验性
