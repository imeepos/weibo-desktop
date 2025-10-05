# 微博 Cookies 验证器

使用 Playwright 验证微博 cookies 有效性的自动化脚本。

## 功能

- 验证微博 cookies 是否有效
- 提取用户 UID 和昵称
- JSON 输入/输出,易于集成
- Headless 浏览器,10秒超时

## 安装

```bash
# 安装依赖
pnpm install

# 构建 TypeScript
npm run build

# 安装 Playwright 浏览器 (首次运行)
npx playwright install chromium
npx playwright install-deps chromium  # 可能需要 sudo
```

## 使用方法

```bash
# 基本用法
node dist/validate-cookies.js '{"SUB":"your_sub_value","SUBP":"your_subp_value"}'

# 输出格式 (成功)
{"valid":true,"uid":"1234567890","screen_name":"用户昵称"}

# 输出格式 (失败)
{"valid":false,"error":"Not logged in or cookies invalid"}
```

## 退出码

- `0`: Cookies 有效
- `1`: Cookies 无效或发生错误

## WSL2 环境注意事项

在 WSL2 中,Chromium 可能遇到 ICU 数据问题。解决方案:

1. **使用完整 Chromium** (不使用 headless shell):
   ```typescript
   const browser = await chromium.launch({
     headless: true,  // 使用常规 headless 模式
     channel: 'chromium',  // 使用系统 Chromium
   });
   ```

2. **使用 Docker 容器**:
   ```dockerfile
   FROM mcr.microsoft.com/playwright:v1.40.0-jammy
   ```

3. **手动安装系统依赖**:
   ```bash
   sudo apt-get update
   sudo apt-get install -y libnss3 libatk1.0-0 libatk-bridge2.0-0 \
     libcups2 libdrm2 libxkbcommon0 libxcomposite1 libxdamage1 \
     libxfixes3 libxrandr2 libgbm1 libasound2
   ```

## API 端点说明

当前使用 `https://m.weibo.cn/profile/info` (移动端页面)。

如果微博更新页面结构,可能需要调整:
- 改用 Web API: `https://weibo.com/ajax/profile/info`
- 或其他稳定的 API 端点

## 开发

```bash
# 监听模式编译
npm run watch

# 运行测试
./test-validate.sh
```

## 架构

```
validate-cookies.ts
├── convertToCookies()    # 转换 cookies 格式
├── validateCookies()     # 验证核心逻辑
└── main()                # CLI 入口
```

## 错误处理

所有错误都转换为结构化 JSON 输出:
- HTTP 错误
- 解析错误
- 超时错误
- 浏览器启动失败

## 依赖

- `playwright`: ^1.40.0 - 浏览器自动化
- `typescript`: ^5.0.0 - 类型安全
- `@types/node`: ^20.0.0 - Node.js 类型定义
