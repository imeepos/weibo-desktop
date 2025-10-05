import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

/**
 * 可访问性测试
 *
 * 验证应用符合 WCAG 2.1 标准
 * - 语义化 HTML
 * - 键盘导航
 * - 屏幕阅读器兼容
 * - 颜色对比度
 *
 * 哲学: 技术应服务所有人,无障碍是基本权利
 */

test.describe('可访问性验证', () => {
  test('初始页面应该通过可访问性检查', async ({ page }) => {
    await page.goto('/');

    // 运行 axe 可访问性扫描
    const accessibilityScanResults = await new AxeBuilder({ page }).analyze();

    // 验证没有严重违规
    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test('二维码页面应该通过可访问性检查', async ({ page }) => {
    await page.goto('/');

    // 生成二维码
    await page.locator('button', { hasText: '生成二维码' }).click();

    // 等待二维码加载
    await page.locator('img[alt="微博登录二维码"]').waitFor({ timeout: 10000 });

    // 运行可访问性扫描
    const accessibilityScanResults = await new AxeBuilder({ page }).analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test('按钮应该支持键盘导航', async ({ page }) => {
    await page.goto('/');

    // 使用 Tab 键聚焦按钮
    await page.keyboard.press('Tab');

    // 验证按钮获得焦点
    const generateButton = page.locator('button', { hasText: '生成二维码' });
    await expect(generateButton).toBeFocused();

    // 使用 Enter 键激活按钮
    await page.keyboard.press('Enter');

    // 验证按钮被触发
    await expect(page.locator('button', { hasText: '生成中...' })).toBeVisible();
  });

  test('图片应该有正确的 alt 文本', async ({ page }) => {
    await page.goto('/');

    await page.locator('button', { hasText: '生成二维码' }).click();

    const qrImage = page.locator('img[alt="微博登录二维码"]');
    await expect(qrImage).toBeVisible({ timeout: 10000 });

    // 验证 alt 属性存在且有意义
    const altText = await qrImage.getAttribute('alt');
    expect(altText).toBeTruthy();
    expect(altText).not.toBe('');
  });
});
