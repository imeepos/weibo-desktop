/**
 * 微博登录服务 - WebSocket 架构
 *
 * 职责:
 * - 通过 WebSocket 提供二维码登录服务
 * - 实时推送登录状态变化
 * - 自动管理浏览器生命周期
 *
 * 消息协议:
 * Client -> Server: { type: 'generate_qrcode' } | { type: 'ping' }
 * Server -> Client: { type: 'qrcode_generated' | 'status_update' | 'error' }
 */

import { chromium, Browser, BrowserContext } from 'playwright';
import { WebSocketServer, WebSocket } from 'ws';

const PORT = 9223;
const QR_TIMEOUT_MS = 180000; // 180秒超时

/**
 * 微博VIP中心API响应格式
 */
interface VipCenterResponse {
  code: number;
  data?: {
    uid?: string | number;
    nickname?: string;
  };
  msg?: string;
}

/**
 * 验证cookies并提取用户信息
 *
 * 调用微博VIP中心API获取真实的用户UID和昵称
 */
async function verifyCookiesAndExtractUserInfo(context: BrowserContext): Promise<{ valid: boolean; uid?: string; screen_name?: string; error?: string }> {
  try {
    const response = await context.request.get('https://vip.weibo.com/aj/vipcenter/user', {
      headers: {
        'accept': 'application/json, text/plain, */*',
        'referer': 'https://vip.weibo.com/home',
      },
      timeout: 10000,
    });

    if (!response.ok()) {
      return {
        valid: false,
        error: `HTTP ${response.status()}: Failed to access VIP center`,
      };
    }

    const vipData: VipCenterResponse = await response.json();

    if (vipData.code !== 100000 || !vipData.data?.uid) {
      return {
        valid: false,
        error: vipData.msg || 'Invalid cookies or missing uid',
      };
    }

    return {
      valid: true,
      uid: String(vipData.data.uid),
      screen_name: vipData.data.nickname || 'Unknown',
    };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

let globalBrowser: Browser | null = null;

// 会话管理: 跟踪活跃的 context 和超时定时器
const activeSessions = new Map<string, {
  context: BrowserContext;
  timeout: NodeJS.Timeout;
  heartbeatInterval?: NodeJS.Timeout;
}>();

async function ensureBrowser(): Promise<Browser> {
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

function generateSessionId(): string {
  return `qr_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
}

// 清理会话资源
async function cleanupSession(sessionId: string) {
  const session = activeSessions.get(sessionId);
  if (!session) return;

  clearTimeout(session.timeout);
  if (session.heartbeatInterval) {
    clearInterval(session.heartbeatInterval);
  }
  try {
    await session.context.close();
  } catch (error) {
    console.error(`清理会话失败 ${sessionId}:`, error);
  }
  activeSessions.delete(sessionId);
  console.log(`会话已清理: ${sessionId}`);
}

async function generateQrcode(ws: WebSocket): Promise<void> {
  const browser = await ensureBrowser();

  const context = await browser.newContext({
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
  });

  const page = await context.newPage();

  const sessionId = generateSessionId();
  let lastRetcode: number | null = null;
  let sessionClosed = false;

  // 监听所有网络请求（用于调试）
  page.on('request', (request) => {
    const url = request.url();
    if (url.includes('qrcode') || url.includes('sso')) {
      console.log(`[${sessionId}] 🌐 请求: ${request.method()} ${url}`);
    }
  });

  // 监听所有网络响应（用于调试）
  page.on('response', async (response) => {
    const url = response.url();
    if (url.includes('qrcode') || url.includes('sso')) {
      console.log(`[${sessionId}] 📥 响应: ${response.status()} ${url}`);
    }
  });

  // 设置超时自动清理
  const timeoutHandle = setTimeout(async () => {
    if (!sessionClosed) {
      console.log(`会话超时: ${sessionId}`);
      sessionClosed = true;
      if (ws.readyState === WebSocket.OPEN) {
        console.log(`[${sessionId}] 发送 WebSocket 消息: type=status_update, retcode=50114004 (过期)`);
        ws.send(JSON.stringify({
          type: 'status_update',
          session_id: sessionId,
          retcode: 50114004, // 过期状态码
          msg: 'QR code expired',
          data: null,
          timestamp: Date.now()
        }));
      }
      await cleanupSession(sessionId);
    }
  }, QR_TIMEOUT_MS);

  // 心跳机制：每 10 秒检查一次会话状态
  const heartbeatInterval = setInterval(() => {
    if (sessionClosed || ws.readyState !== WebSocket.OPEN) {
      clearInterval(heartbeatInterval);
      return;
    }
    console.log(`[${sessionId}] 💓 心跳检查 - 会话活跃`);
  }, 10000);

  // 注册会话
  activeSessions.set(sessionId, { context, timeout: timeoutHandle, heartbeatInterval });
  console.log(`会话已创建: ${sessionId}, 超时时间: ${QR_TIMEOUT_MS}ms (${QR_TIMEOUT_MS / 1000}秒)`);

  // 专门处理 qrcode/check 响应
  page.on('response', async (checkResponse) => {
    if (!checkResponse.url().includes('/sso/v2/qrcode/check') || sessionClosed) return;

    console.log(`[${sessionId}] ✅ 捕获 /sso/v2/qrcode/check 响应`);

    try {
      const status = checkResponse.status();
      console.log(`[${sessionId}] 响应状态码: ${status}`);

      if (status !== 200) {
        console.log(`[${sessionId}] ⚠️  非 200 响应，跳过处理`);
        return;
      }

      const data = await checkResponse.json();
      const currentRetcode = data.retcode;
      const msg = data.msg || '';

      console.log(`[${sessionId}] 📊 retcode=${currentRetcode}, msg="${msg}"`);

      if (currentRetcode !== lastRetcode && ws.readyState === WebSocket.OPEN) {
        if (lastRetcode !== null) {
          console.log(`[${sessionId}] ⚠️  状态变化: retcode ${lastRetcode} -> ${currentRetcode}`);
        }

        console.log(`[${sessionId}] 📤 发送 WebSocket 消息: type=status_update, retcode=${currentRetcode}`);
        ws.send(JSON.stringify({
          type: 'status_update',
          session_id: sessionId,
          retcode: data.retcode,
          msg: msg,
          data: data.data || null,
          timestamp: Date.now()
        }));
        lastRetcode = currentRetcode;

        // 如果是终止状态,清理会话
        if ([50114003, 50114004, 50114005, 50114006, 50114007].includes(currentRetcode)) {
          console.log(`[${sessionId}] 检测到终止状态 (retcode=${currentRetcode}), 即将清理会话`);
          sessionClosed = true;
          await cleanupSession(sessionId);
        }
      }
    } catch (error: any) {
      if (sessionClosed) return;

      // 登录成功时页面会跳转,导致无法获取响应体 (资源已销毁)
      // 这是正常现象,需提取 cookies 并发送 login_confirmed 事件
      const errorMessage = error?.message || '';
      const isLoginSuccess = errorMessage.includes('No resource with given identifier found');

      if (isLoginSuccess) {
        console.log(`[${sessionId}] ✅ 检测到登录成功信号 (资源已销毁错误)`);

        try {
          const cookies = await context.cookies();
          const cookieMap: Record<string, string> = {};
          cookies.forEach(c => { cookieMap[c.name] = c.value; });

          console.log(`[${sessionId}] 提取到 ${cookies.length} 个 cookies`);

          // 调用VIP API获取真实的UID和昵称
          const verification = await verifyCookiesAndExtractUserInfo(context);

          if (!verification.valid) {
            console.error(`[${sessionId}] VIP API验证失败: ${verification.error}`);
            if (ws.readyState === WebSocket.OPEN) {
              ws.send(JSON.stringify({
                type: 'error',
                error_type: 'ValidationFailed',
                message: verification.error || 'Failed to verify cookies',
                timestamp: Date.now()
              }));
            }
            sessionClosed = true;
            await cleanupSession(sessionId);
            return;
          }

          const uid = verification.uid || '';
          const screen_name = verification.screen_name || 'Unknown';

          console.log(`[${sessionId}] 从VIP API提取 UID: ${uid}, 昵称: ${screen_name}`);

          if (ws.readyState === WebSocket.OPEN) {
            console.log(`[${sessionId}] 发送 WebSocket 消息: type=login_confirmed, uid=${uid}`);
            ws.send(JSON.stringify({
              type: 'login_confirmed',
              session_id: sessionId,
              status: 'confirmed',
              cookies: cookieMap,
              uid: uid,
              screen_name: screen_name,
              timestamp: Date.now()
            }));
          }

          // 登录成功,清理会话
          sessionClosed = true;
          await cleanupSession(sessionId);
        } catch (cookieError: any) {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({
              type: 'error',
              error_type: 'CookieExtractionFailed',
              message: cookieError?.message || 'Failed to extract cookies after login',
              timestamp: Date.now()
            }));
          }
          sessionClosed = true;
          await cleanupSession(sessionId);
        }
      } else if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
          type: 'error',
          error_type: 'ResponseParseFailed',
          message: errorMessage || 'Failed to parse qrcode check response',
          timestamp: Date.now()
        }));
      }
    }
  });

  await page.goto('https://passport.weibo.com/sso/signin?entry=miniblog&source=miniblog&disp=popup&url=https%3A%2F%2Fweibo.com%2Fnewlogin%3Ftabtype%3Dweibo%26gid%3D102803%26openLoginLayer%3D0%26url%3Dhttps%253A%252F%252Fweibo.com%252F&from=weibopro', {
    waitUntil: 'domcontentloaded',
    timeout: 30000
  });

  await page.waitForTimeout(3000);

  try {
    await page.click('text=扫描二维码登录', { timeout: 3000 });
    await page.waitForTimeout(2000);
  } catch {}

  const qrSelectors = ['.login-qrcode img', '.qrcode img', 'img[src*="qrcode"]', '[class*="qrcode"] img'];

  let qrImageSrc: string | null = null;
  for (const selector of qrSelectors) {
    try {
      await page.waitForSelector(selector, { timeout: 5000, state: 'visible' });
      qrImageSrc = await page.locator(selector).first().getAttribute('src');
      if (qrImageSrc) break;
    } catch {}
  }

  if (!qrImageSrc) {
    sessionClosed = true;
    await cleanupSession(sessionId);
    throw new Error('QR code not found');
  }

  let qrImageBase64: string;
  if (qrImageSrc.startsWith('data:image')) {
    qrImageBase64 = qrImageSrc.split(',')[1];
  } else {
    const response = await fetch(new URL(qrImageSrc, page.url()).href);
    qrImageBase64 = Buffer.from(await response.arrayBuffer()).toString('base64');
  }

  const now = Date.now();
  const expiresAt = now + QR_TIMEOUT_MS;

  if (ws.readyState === WebSocket.OPEN) {
    console.log(`[${sessionId}] 发送 WebSocket 消息: type=qrcode_generated, expires_in=${Math.floor(QR_TIMEOUT_MS / 1000)}秒`);
    ws.send(JSON.stringify({
      type: 'qrcode_generated',
      session_id: sessionId,
      qr_image: qrImageBase64,
      expires_in: Math.floor((expiresAt - now) / 1000),
      expires_at: expiresAt,
      timestamp: now
    }));
  }

  // ✅ 不再立即关闭 context,保持监听直到登录完成或超时
}


const wss = new WebSocketServer({ port: PORT });

wss.on('connection', (ws) => {
  console.log('🔗 新的WebSocket连接建立');
  let currentSessionId: string | null = null;

  ws.on('message', async (data) => {
    console.log('📨 收到WebSocket消息:', data.toString());
    try {
      const message = JSON.parse(data.toString());
      console.log('📋 解析后的消息类型:', message.type);

      if (message.type === 'generate_qrcode') {
        try {
          await generateQrcode(ws);
        } catch (error: any) {
          ws.readyState === WebSocket.OPEN && ws.send(JSON.stringify({
            type: 'error',
            error_type: 'QrcodeGenerationFailed',
            message: error.message,
            timestamp: Date.now()
          }));
        }
      } else if (message.type === 'ping') {
        ws.send(JSON.stringify({ type: 'pong', timestamp: Date.now() }));
      }
    } catch {
      ws.send(JSON.stringify({
        type: 'error',
        error_type: 'InvalidMessage',
        message: 'Failed to parse message',
        timestamp: Date.now()
      }));
    }
  });

  // WebSocket关闭时清理所有相关会话
  ws.on('close', async () => {
    console.log('❌ WebSocket连接关闭,清理所有会话');
    // 清理所有活跃会话 (通常一个连接只有一个会话,但安全起见清理所有)
    for (const [sessionId] of activeSessions) {
      await cleanupSession(sessionId);
    }
  });
});

process.on('SIGINT', async () => {
  console.log('收到SIGINT信号,清理所有资源');

  // 清理所有会话
  for (const [sessionId] of activeSessions) {
    await cleanupSession(sessionId);
  }

  if (globalBrowser) {
    await globalBrowser.close();
  }
  wss.close();
  process.exit(0);
});

console.log(`微博登录服务已启动 - WebSocket端口: ${PORT}`);
