# 前端UI架构说明

## 目录结构

```
src/
├── types/              # 类型定义
│   └── weibo.ts       # 微博登录相关类型
├── components/         # 可复用组件
│   ├── QrcodeDisplay.tsx   # 二维码显示组件
│   └── LoginStatus.tsx     # 登录状态组件
├── pages/              # 页面组件
│   ├── LoginPage.tsx       # 主登录页面
│   └── CookiesListPage.tsx # Cookies管理页面
├── App.tsx            # 应用入口
└── main.tsx           # React挂载点
```

## 组件职责

### 类型层 (types/weibo.ts)
与Rust后端完全对应的TypeScript类型定义,确保类型安全。

### 组件层
- **QrcodeDisplay**: 展示二维码,追踪倒计时,反馈状态
- **LoginStatus**: 将登录事件翻译成人类可读的提示

### 页面层
- **LoginPage**: 编排登录流程,管理状态机,自动轮询
- **CookiesListPage**: 管理已保存的Cookies,支持查看和删除

## 设计原则

### 存在即合理
每个组件都有明确的单一职责,没有冗余代码。

### 优雅即简约
- 使用函数组件 + Hooks
- 避免不必要的React.FC类型
- 纯TypeScript类型推导

### 性能即艺术
- useCallback防止不必要的重渲染
- 倒计时使用setInterval,组件卸载时清理

### 错误处理
- try-catch捕获异步错误
- 用户友好的错误提示
- 状态分离(loading/error/data)

## 状态机设计

LoginPage的状态流转:
```
[初始] -> 生成二维码 -> [等待扫码]
                              |
                              v
                         [已扫描] -> [确认登录]
                              |           |
                              v           v
                         [过期]      [成功/失败]
```

## TailwindCSS样式规范

- 使用语义化颜色: blue(信息), yellow(警告), green(成功), red(错误)
- 渐变背景: `bg-gradient-to-br from-blue-50 to-indigo-100`
- 阴影层次: `shadow` -> `shadow-lg`
- 响应式: `grid-cols-1 md:grid-cols-2`

## Tauri集成

通过 `@tauri-apps/api/tauri` 的 `invoke` 函数调用Rust命令:

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// 生成二维码
const response = await invoke<GenerateQrcodeResponse>('generate_qrcode');

// 轮询状态
const poll = await invoke<PollStatusResponse>('poll_login_status', {
  qrId: 'xxx',
});
```

## 启动开发服务器

```bash
pnpm run dev
```

## 构建生产版本

```bash
pnpm run build
```
