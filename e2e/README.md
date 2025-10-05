# E2E 测试目录

这个目录包含使用 Playwright 编写的端到端测试,用于验证 Tauri 应用的 UI 功能。

## 测试文件

### login.spec.ts
微博扫码登录界面的主要测试套件:
- 初始页面渲染验证
- 二维码生成流程
- 状态变化和倒计时
- 错误处理
- 视觉回归测试

### accessibility.spec.ts
可访问性测试套件:
- WCAG 2.1 标准合规性验证
- 键盘导航支持
- 屏幕阅读器兼容性
- 语义化 HTML 验证

## 运行测试

```bash
# 快速运行所有测试
pnpm test:e2e

# 查看详细指南
cat ../E2E_TESTING_GUIDE.md
```

## 编写新测试

### 测试结构

```typescript
import { test, expect } from '@playwright/test';

test.describe('功能模块名称', () => {
  test('应该验证某个具体行为', async ({ page }) => {
    // 1. 导航到页面
    await page.goto('/');

    // 2. 执行操作
    await page.locator('button').click();

    // 3. 验证结果
    await expect(page.locator('.result')).toBeVisible();
  });
});
```

### 选择器优先级

从高到低:

1. **角色选择器**: `page.getByRole('button', { name: '登录' })`
2. **文本选择器**: `page.locator('text=生成二维码')`
3. **测试 ID**: `page.locator('[data-testid="qrcode"]')`
4. **CSS 选择器**: `page.locator('button.primary')` (最后选择)

### 等待策略

```typescript
// ✅ 好: 明确等待
await page.locator('.element').waitFor({ state: 'visible' });

// ❌ 差: 固定延迟
await page.waitForTimeout(3000);
```

### 断言风格

```typescript
// ✅ 好: 语义化断言
await expect(page.locator('h1')).toHaveText('微博扫码登录');

// ❌ 差: 过度具体
const text = await page.locator('h1').textContent();
expect(text).toBe('微博扫码登录');
```

## 测试原则

1. **独立性**: 每个测试可以单独运行
2. **确定性**: 测试结果可重复
3. **清晰性**: 测试意图一目了然
4. **简洁性**: 只测试必要的行为

## 配置

测试配置位于根目录的 `playwright.config.ts`。

关键配置:
- 超时: 60 秒
- 重试: CI 环境 2 次
- 截图: 失败时自动
- 追踪: 失败时保留
- 无头模式: Docker 环境强制启用

## 调试

```bash
# 调试模式 (逐步执行)
pnpm test:e2e:debug

# 查看失败追踪
pnpm exec playwright show-trace test-results/.../trace.zip

# 更新截图基准
pnpm exec playwright test --update-snapshots
```

---

**编写测试时思考**: 这个测试验证的是不可或缺的用户体验吗?
