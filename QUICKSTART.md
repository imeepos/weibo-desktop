# 快速启动指南

## 前置条件

1. **后端服务运行中**
   ```bash
   cd /workspace/desktop
   docker compose up -d
   ```

2. **依赖已安装**
   ```bash
   pnpm install
   ```

3. **Playwright浏览器已安装** ⚡ 重要
   ```bash
   cd playwright
   pnpm exec playwright install chromium
   cd ..
   ```

   > ⚠️  **必须执行**: Playwright依赖包安装不会自动下载浏览器二进制文件。
   > 如果跳过此步骤,二维码生成功能将失败。

## 开发模式

### 启动Vite开发服务器
```bash
pnpm run dev
```

浏览器访问: `http://localhost:5173`

### 启动Tauri桌面应用
```bash
pnpm run tauri dev
```

## 生产构建

### 构建前端资源
```bash
pnpm run build
```

### 构建Tauri应用
```bash
pnpm run tauri build
```

## 功能验证

### 1. 生成二维码
1. 点击「生成二维码」按钮
2. 等待后端返回二维码图片
3. 观察倒计时 (300秒)

### 2. 扫码登录
1. 使用微博App扫描二维码
2. 在手机上点击「确认登录」
3. 观察状态变化:
   - 等待扫码 -> 已扫描 -> 确认成功 -> 验证成功

### 3. 查看Cookies
1. 登录成功后,Cookies自动保存到Redis
2. 切换到Cookies管理页面 (需要添加路由)
3. 查看已保存的UID和Cookies详情

## 调试技巧

### 查看Tauri日志
```bash
# Rust后端日志
tail -f src-tauri/target/debug/weibo-login-desktop.log

# 或者在终端查看实时输出
pnpm run tauri dev
```

### 查看React DevTools
安装React DevTools浏览器插件,在开发模式下查看组件状态。

### 查看后端服务日志
```bash
docker compose logs -f server
```

## 常见问题

### Q: 二维码生成失败?
A: 检查后端服务是否运行:
```bash
docker compose ps
curl http://localhost:3000/health
```

### Q: 轮询一直等待?
A: 检查后端日志,确认Playwright正常运行:
```bash
docker compose logs playwright
```

### Q: TypeScript类型错误?
A: 确保前后端类型同步:
```bash
# 重新构建检查
pnpm run build
```

## 项目结构

```
desktop/
├── src/                    # 前端源码
│   ├── types/             # TypeScript类型
│   ├── components/        # React组件
│   ├── pages/             # 页面组件
│   └── App.tsx            # 应用入口
├── src-tauri/             # Tauri后端
│   ├── src/
│   │   └── main.rs       # Rust主程序
│   └── Cargo.toml
├── dist/                  # Vite构建输出
└── package.json
```

## 下一步

完成Phase 7后,建议进行:
1. **Phase 8**: 端到端集成测试
2. 添加路由 (React Router)
3. 添加状态管理 (Zustand/Jotai)
4. 完善错误处理
5. 添加日志系统

---

**提示**: 这是一个Tauri应用,前端UI运行在Webview中,通过invoke调用Rust后端命令。
