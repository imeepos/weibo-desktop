/**
 * 浏览器实例管理
 *
 * 职责:
 * - 统一管理全局Browser实例
 * - 确保Browser生命周期
 */

import { chromium, Browser } from 'playwright';

let globalBrowser: Browser | null = null;

/**
 * 确保Browser实例存在
 */
export async function ensureBrowser(): Promise<Browser> {
  if (globalBrowser && globalBrowser.isConnected()) {
    return globalBrowser;
  }

  globalBrowser = await chromium.launch({
    headless: true,
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-dev-shm-usage',
    ]
  });

  return globalBrowser;
}

/**
 * 关闭Browser实例
 */
export async function closeBrowser(): Promise<void> {
  if (globalBrowser) {
    await globalBrowser.close();
    globalBrowser = null;
    console.log('浏览器已关闭');
  }
}
