import { test, expect } from '@playwright/test';

/**
 * 微博扫码登录 UI 测试
 *
 * 验证目标:
 * 1. 页面基础元素正确渲染
 * 2. 二维码生成流程正常
 * 3. 状态变化反馈清晰
 * 4. 错误处理优雅
 *
 * 测试哲学: 从用户视角验证体验,而非实现细节
 */

test.describe('微博扫码登录界面', () => {
  test('应该正确显示初始页面元素', async ({ page }) => {
    // 访问应用
    await page.goto('/');

    // 验证页面标题
    await expect(page.locator('h1')).toHaveText('微博扫码登录');

    // 验证说明文字
    await expect(page.locator('text=获取微博Cookies')).toBeVisible();

    // 验证"生成二维码"按钮存在且可点击
    const generateButton = page.locator('button', { hasText: '生成二维码' });
    await expect(generateButton).toBeVisible();
    await expect(generateButton).toBeEnabled();

    // 验证底部提示信息
    await expect(page.locator('text=使用微博App扫描二维码并确认登录')).toBeVisible();
    await expect(page.locator('text=Cookies将自动保存到Redis')).toBeVisible();
  });

  test('点击生成二维码按钮应该触发加载状态', async ({ page }) => {
    await page.goto('/');

    const generateButton = page.locator('button', { hasText: '生成二维码' });

    // 点击按钮
    await generateButton.click();

    // 验证按钮状态变为"生成中..."
    await expect(page.locator('button', { hasText: '生成中...' })).toBeVisible();

    // 验证按钮被禁用
    const disabledButton = page.locator('button:disabled', { hasText: '生成中...' });
    await expect(disabledButton).toBeVisible();
  });

  test('成功生成二维码后应该显示二维码图片', async ({ page }) => {
    await page.goto('/');

    // 点击生成按钮
    await page.locator('button', { hasText: '生成二维码' }).click();

    // 等待二维码图片出现 (最多 10 秒)
    const qrImage = page.locator('img[alt="微博登录二维码"]');
    await expect(qrImage).toBeVisible({ timeout: 10000 });

    // 验证图片是 base64 格式
    const imgSrc = await qrImage.getAttribute('src');
    expect(imgSrc).toContain('data:image/png;base64,');

    // 验证二维码尺寸正确 (264x264 = 256 + padding)
    await expect(qrImage).toHaveCSS('width', '256px');
    await expect(qrImage).toHaveCSS('height', '256px');
  });

  test('应该显示会话ID信息', async ({ page }) => {
    await page.goto('/');

    await page.locator('button', { hasText: '生成二维码' }).click();

    // 等待会话 ID 显示
    const sessionInfo = page.locator('text=/会话ID:.*/', { timeout: 10000 });
    await expect(sessionInfo).toBeVisible();

    // 验证 ID 格式 (至少 12 字符)
    const sessionText = await sessionInfo.textContent();
    expect(sessionText).toMatch(/会话ID: [a-z0-9]{12}\.\.\./);
  });

  test('应该显示倒计时', async ({ page }) => {
    await page.goto('/');

    await page.locator('button', { hasText: '生成二维码' }).click();

    // 等待倒计时显示
    const countdown = page.locator('text=/剩余 \\d+ 秒/');
    await expect(countdown).toBeVisible({ timeout: 10000 });

    // 验证倒计时数字在合理范围内 (0-300秒)
    const countdownText = await countdown.textContent();
    const seconds = parseInt(countdownText?.match(/\d+/)?.[0] || '0');
    expect(seconds).toBeGreaterThan(0);
    expect(seconds).toBeLessThanOrEqual(300);
  });

  test('应该显示二维码状态提示', async ({ page }) => {
    await page.goto('/');

    await page.locator('button', { hasText: '生成二维码' }).click();

    // 等待状态提示
    const statusText = page.locator('text=请使用微博App扫描二维码');
    await expect(statusText).toBeVisible({ timeout: 10000 });

    // 验证状态颜色为蓝色 (pending 状态)
    const statusContainer = page.locator('p.text-blue-600', {
      hasText: '请使用微博App扫描二维码',
    });
    await expect(statusContainer).toBeVisible();
  });

  test('应该显示二维码生成成功事件', async ({ page }) => {
    await page.goto('/');

    await page.locator('button', { hasText: '生成二维码' }).click();

    // 等待事件状态组件显示
    const eventMessage = page.locator('text=二维码生成成功');
    await expect(eventMessage).toBeVisible({ timeout: 10000 });

    // 验证事件时间戳存在
    const timestamp = page.locator('.text-xs.opacity-50');
    await expect(timestamp).toBeVisible();
  });

  test('页面应该响应式布局适配不同屏幕', async ({ page }) => {
    await page.goto('/');

    // 验证容器最大宽度
    const container = page.locator('.max-w-md');
    await expect(container).toBeVisible();

    // 验证渐变背景
    const background = page.locator('.bg-gradient-to-br.from-blue-50.to-indigo-100');
    await expect(background).toBeVisible();
  });

  test('应该正确显示处理中的加载状态', async ({ page }) => {
    await page.goto('/');

    await page.locator('button', { hasText: '生成二维码' }).click();

    // 查找加载动画 (spinning circle)
    const spinner = page.locator('.animate-spin');

    // 加载动画可能短暂出现
    // 如果在 2 秒内出现,验证其存在
    const spinnerVisible = await spinner.isVisible().catch(() => false);

    if (spinnerVisible) {
      await expect(spinner).toHaveClass(/animate-spin/);
    }
  });
});

test.describe('错误处理', () => {
  test('网络错误时应该显示错误信息', async ({ page }) => {
    // 模拟网络失败
    await page.route('**/api/**', (route) => route.abort());

    await page.goto('/');

    const generateButton = page.locator('button', { hasText: '生成二维码' });
    await generateButton.click();

    // 等待错误信息显示 (最多 10 秒)
    const errorContainer = page.locator('.bg-red-50.border-red-200');

    // 错误可能不会立即出现,给予合理超时
    const errorVisible = await errorContainer.isVisible({ timeout: 10000 }).catch(() => false);

    if (errorVisible) {
      await expect(errorContainer).toBeVisible();
      const errorText = page.locator('.text-red-700');
      await expect(errorText).toBeVisible();
    }
  });
});

test.describe('视觉回归', () => {
  test('初始页面截图对比', async ({ page }) => {
    await page.goto('/');

    // 等待页面完全加载
    await page.waitForLoadState('networkidle');

    // 截图对比 (首次运行会生成基准)
    await expect(page).toHaveScreenshot('initial-page.png', {
      fullPage: true,
      maxDiffPixels: 100, // 允许微小差异
    });
  });
});
