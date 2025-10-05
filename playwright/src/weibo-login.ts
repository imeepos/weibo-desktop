/**
 * 微博扫码登录服务 (Playwright实现)
 *
 * 存在即合理:
 * - 移除对微博OAuth2 API的依赖,使用真实登录页面
 * - 每个函数都有不可替代的职责
 * - 无冗余代码,无无意义注释
 *
 * 使用方式:
 * ```bash
 * node dist/weibo-login.js generate
 * node dist/weibo-login.js check <session_id>
 * ```
 */

import { chromium, Browser, BrowserContext, Page } from 'playwright';
import * as fs from 'fs';
import * as path from 'path';

/// 会话存储目录
const SESSIONS_DIR = path.join(__dirname, '../.sessions');

/// 二维码生成响应
interface GenerateResponse {
  session_id: string;
  qr_image: string;
  expires_in: number;
}

/// 登录状态响应
interface StatusResponse {
  status: 'pending' | 'scanned' | 'confirmed' | 'expired';
  cookies?: Record<string, string>;
  uid?: string;
  screen_name?: string;
}

/// 会话数据
interface SessionData {
  browser_ws_endpoint?: string;
  context_state_path: string;
  created_at: number;
  expires_at: number;
}

function ensureSessionsDir(): void {
  if (!fs.existsSync(SESSIONS_DIR)) {
    fs.mkdirSync(SESSIONS_DIR, { recursive: true });
  }
}

function generateSessionId(): string {
  return `qr_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
}

function saveSession(sessionId: string, data: SessionData): void {
  ensureSessionsDir();
  const filePath = path.join(SESSIONS_DIR, `${sessionId}.json`);
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
}

function loadSession(sessionId: string): SessionData | null {
  const filePath = path.join(SESSIONS_DIR, `${sessionId}.json`);
  if (!fs.existsSync(filePath)) {
    return null;
  }
  const content = fs.readFileSync(filePath, 'utf-8');
  return JSON.parse(content);
}

/**
 * 生成二维码
 *
 * 职责:
 * 1. 启动浏览器访问微博登录页
 * 2. 提取二维码图片(base64)
 * 3. 持久化浏览器会话
 */
async function generateQrcode(): Promise<GenerateResponse> {
  const browser = await chromium.launch({
    headless: true,
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-dev-shm-usage',
    ]
  });

  const context = await browser.newContext({
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
  });

  const page = await context.newPage();

  await page.goto('https://passport.weibo.com/sso/signin?entry=miniblog&source=miniblog&disp=popup&url=https%3A%2F%2Fweibo.com%2Fnewlogin%3Ftabtype%3Dweibo%26gid%3D102803%26openLoginLayer%3D0%26url%3Dhttps%253A%252F%252Fweibo.com%252F&from=weibopro', {
    waitUntil: 'domcontentloaded',
    timeout: 60000,
  });

  /// 等待页面加载完成
  await page.waitForTimeout(3000);

  /// 保存页面快照用于调试
  const pageContent = await page.content();
  console.error('DEBUG: Page loaded, title:', await page.title());
  console.error('DEBUG: URL:', page.url());

  /// 尝试点击"扫码登录"
  try {
    const scanTabSelector = 'text=扫码登录';
    await page.waitForSelector(scanTabSelector, { timeout: 3000 });
    await page.click(scanTabSelector);
    await page.waitForTimeout(2000);
  } catch {
    /// 可能已经在扫码页面,继续
    console.error('DEBUG: Skip scan tab click');
  }

  /// 尝试多种二维码选择器
  const qrSelectors = [
    '.login-qrcode img',
    '.qrcode img',
    'img[src*="qrcode"]',
    'img[alt*="二维码"]',
    'canvas',
    '[class*="qrcode"] img',
    '[class*="qr-code"] img'
  ];

  let qrImageElement = null;
  let foundSelector = '';

  for (const selector of qrSelectors) {
    try {
      await page.waitForSelector(selector, { timeout: 5000, state: 'visible' });
      qrImageElement = page.locator(selector).first();
      foundSelector = selector;
      console.error(`DEBUG: Found QR code with selector: ${selector}`);
      break;
    } catch {
      console.error(`DEBUG: Selector not found: ${selector}`);
    }
  }

  if (!qrImageElement) {
    /// 保存HTML用于诊断
    const fs = require('fs');
    const debugPath = '/workspace/desktop/playwright/.sessions/debug-page.html';
    fs.writeFileSync(debugPath, pageContent);
    console.error(`DEBUG: Page HTML saved to ${debugPath}`);
    throw new Error('QR code element not found with any selector');
  }

  const qrImageSrc = await qrImageElement.getAttribute('src');

  if (!qrImageSrc) {
    throw new Error('Failed to extract QR code image source');
  }

  /// 转换为base64
  let qrImageBase64: string;
  if (qrImageSrc.startsWith('data:image')) {
    qrImageBase64 = qrImageSrc.split(',')[1];
  } else {
    const absoluteUrl = new URL(qrImageSrc, page.url()).href;
    const response = await page.goto(absoluteUrl);
    if (!response) {
      throw new Error('Failed to fetch QR code image');
    }
    const imageBuffer = await response.body();
    qrImageBase64 = imageBuffer.toString('base64');
  }

  const sessionId = generateSessionId();
  const contextStatePath = path.join(SESSIONS_DIR, `${sessionId}_state.json`);

  /// 保存浏览器上下文状态
  await context.storageState({ path: contextStatePath });

  saveSession(sessionId, {
    context_state_path: contextStatePath,
    created_at: Date.now(),
    expires_at: Date.now() + 180 * 1000, // 3分钟
  });

  /// 关闭浏览器(后续通过storageState恢复)
  await browser.close();

  return {
    session_id: sessionId,
    qr_image: qrImageBase64,
    expires_in: 180,
  };
}

/**
 * 检查登录状态
 *
 * 职责:
 * 1. 恢复浏览器会话
 * 2. 在二维码页面监听状态变化
 * 3. 处理过期/扫码/确认状态
 * 4. 自动刷新过期二维码
 */
async function checkStatus(sessionId: string): Promise<StatusResponse> {
  const sessionData = loadSession(sessionId);
  if (!sessionData) {
    return { status: 'expired' };
  }

  const browser = await chromium.launch({
    headless: true,
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-dev-shm-usage',
    ]
  });

  const context = await browser.newContext({
    storageState: sessionData.context_state_path,
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
  });

  const page = await context.newPage();

  try {
    /// 访问登录页面
    await page.goto('https://passport.weibo.com/sso/signin?entry=miniblog&source=miniblog&disp=popup&url=https%3A%2F%2Fweibo.com%2Fnewlogin%3Ftabtype%3Dweibo%26gid%3D102803%26openLoginLayer%3D0%26url%3Dhttps%253A%252F%252Fweibo.com%252F&from=weibopro', {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForTimeout(2000);

    /// 检查是否已跳转到首页(登录成功)
    const currentUrl = page.url();
    if (currentUrl === 'https://weibo.com/' || currentUrl.includes('/home')) {
      const cookies = await context.cookies();
      const cookiesMap: Record<string, string> = {};
      cookies.forEach(cookie => {
        cookiesMap[cookie.name] = cookie.value;
      });

      const userInfo = await page.evaluate(() => {
        const userElement = document.querySelector('[usercard]');
        if (userElement) {
          const usercard = userElement.getAttribute('usercard');
          const uidMatch = usercard?.match(/id=(\d+)/);
          return {
            uid: uidMatch?.[1],
            screen_name: userElement.textContent?.trim(),
          };
        }

        const nameElement = document.querySelector('.gn_name, .name');
        return {
          uid: undefined,
          screen_name: nameElement?.textContent?.trim(),
        };
      });

      await browser.close();

      return {
        status: 'confirmed',
        cookies: cookiesMap,
        uid: userInfo.uid,
        screen_name: userInfo.screen_name,
      };
    }

    /// 确保在扫码页面
    try {
      const scanTabSelector = 'text=扫码登录';
      await page.waitForSelector(scanTabSelector, { timeout: 3000 });
      await page.click(scanTabSelector);
      await page.waitForTimeout(1000);
    } catch {
      // 已在扫码页面
    }

    /// 状态检测选择器
    const expiredSelector = 'text=该二维码已过期';
    const scannedSelector = 'text=成功扫描，请在手机点击确认以登录';
    const refreshButtonSelector = 'text=点击刷新';

    /// 检查过期状态
    const expiredElement = await page.locator(expiredSelector).count();
    if (expiredElement > 0) {
      /// 尝试刷新二维码
      const refreshButton = await page.locator(refreshButtonSelector).count();
      if (refreshButton > 0) {
        await page.click(refreshButtonSelector);
        await page.waitForTimeout(2000);

        /// 更新会话过期时间
        sessionData.expires_at = Date.now() + 180 * 1000;
        saveSession(sessionId, sessionData);

        await browser.close();
        return { status: 'pending' };
      }

      await browser.close();
      return { status: 'expired' };
    }

    /// 检查扫码成功状态
    const scannedElement = await page.locator(scannedSelector).count();
    if (scannedElement > 0) {
      await browser.close();
      return { status: 'scanned' };
    }

    /// 等待URL变化(登录确认)
    const navigationPromise = page.waitForURL(url => url.includes('weibo.com/') && !url.includes('/sso/'), {
      timeout: 2000,
    }).catch(() => null);

    if (await navigationPromise) {
      const cookies = await context.cookies();
      const cookiesMap: Record<string, string> = {};
      cookies.forEach(cookie => {
        cookiesMap[cookie.name] = cookie.value;
      });

      const userInfo = await page.evaluate(() => {
        const userElement = document.querySelector('[usercard]');
        if (userElement) {
          const usercard = userElement.getAttribute('usercard');
          const uidMatch = usercard?.match(/id=(\d+)/);
          return {
            uid: uidMatch?.[1],
            screen_name: userElement.textContent?.trim(),
          };
        }

        const nameElement = document.querySelector('.gn_name, .name');
        return {
          uid: undefined,
          screen_name: nameElement?.textContent?.trim(),
        };
      });

      await browser.close();

      return {
        status: 'confirmed',
        cookies: cookiesMap,
        uid: userInfo.uid,
        screen_name: userInfo.screen_name,
      };
    }

    /// 默认pending状态
    await browser.close();
    return { status: 'pending' };

  } catch (error) {
    await browser.close();
    throw error;
  }
}

/**
 * 主入口
 *
 * 命令:
 * - generate: 生成二维码
 * - check <session_id>: 检查登录状态
 */
async function main(): Promise<void> {
  const command = process.argv[2];

  try {
    if (command === 'generate') {
      const response = await generateQrcode();
      console.log(JSON.stringify(response));
      process.exit(0);
    } else if (command === 'check') {
      const sessionId = process.argv[3];
      if (!sessionId) {
        throw new Error('Session ID required for check command');
      }
      const response = await checkStatus(sessionId);
      console.log(JSON.stringify(response));
      process.exit(0);
    } else {
      throw new Error(`Unknown command: ${command}. Use 'generate' or 'check <session_id>'`);
    }
  } catch (error) {
    console.error(JSON.stringify({
      error: error instanceof Error ? error.message : String(error),
    }));
    process.exit(1);
  }
}

main();
