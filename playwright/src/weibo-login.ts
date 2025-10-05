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
  });

  const context = await browser.newContext({
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
  });

  const page = await context.newPage();

  await page.goto('https://weibo.com/login', {
    waitUntil: 'networkidle',
    timeout: 30000,
  });

  /// 尝试点击"扫码登录"
  try {
    const scanTabSelector = 'text=扫码登录';
    await page.waitForSelector(scanTabSelector, { timeout: 3000 });
    await page.click(scanTabSelector);
  } catch {
    /// 可能已经在扫码页面,继续
  }

  /// 等待二维码加载
  const qrSelector = '.login-qrcode img, .qrcode img, img[src*="qrcode"]';
  await page.waitForSelector(qrSelector, { timeout: 10000 });

  const qrImageElement = page.locator(qrSelector).first();
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
 * 2. 检测登录状态(pending/scanned/confirmed/expired)
 * 3. 登录成功时提取cookies和用户信息
 */
async function checkStatus(sessionId: string): Promise<StatusResponse> {
  const sessionData = loadSession(sessionId);
  if (!sessionData) {
    return { status: 'expired' };
  }

  if (Date.now() > sessionData.expires_at) {
    return { status: 'expired' };
  }

  const browser = await chromium.launch({
    headless: true,
  });

  const context = await browser.newContext({
    storageState: sessionData.context_state_path,
  });

  const page = await context.newPage();

  /// 访问首页检查登录状态
  await page.goto('https://weibo.com/', {
    waitUntil: 'networkidle',
    timeout: 15000,
  });

  const currentUrl = page.url();

  /// 检查是否已登录
  const isLoggedIn = currentUrl.includes('/home') ||
                    currentUrl === 'https://weibo.com/' &&
                    await page.locator('[usercard], .gn_name, .name').count() > 0;

  if (isLoggedIn) {
    const cookies = await context.cookies();
    const cookiesMap: Record<string, string> = {};
    cookies.forEach(cookie => {
      cookiesMap[cookie.name] = cookie.value;
    });

    /// 提取用户信息
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

      /// 备选方案
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

  /// 检查是否回到登录页(已扫码但未确认)
  if (currentUrl.includes('/login')) {
    const pageContent = await page.content();
    const hasScanned = pageContent.includes('已扫描') ||
                      pageContent.includes('扫描成功') ||
                      pageContent.includes('请在手机上确认');

    await browser.close();

    if (hasScanned) {
      return { status: 'scanned' };
    }
  }

  await browser.close();

  return { status: 'pending' };
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
