/**
 * 爬虫处理器
 *
 * 职责:
 * - 处理微博搜索爬取请求
 * - 提取帖子数据
 * - 检测验证码
 */

import { BrowserContext, WebSocket } from '../types';
import { ensureBrowser } from '../browser';

const REQUEST_TIMEOUT_MS = 30000;

/**
 * 爬取请求
 */
interface CrawlRequest {
  keyword: string;
  startTime?: string;    // YYYYMMDDhhmmss
  endTime?: string;      // YYYYMMDDhhmmss
  page: number;
  cookies: Record<string, string>;
}

/**
 * 爬取结果
 */
interface CrawlResult {
  posts: WeiboPost[];
  hasMore: boolean;
  totalResults?: number;
  captchaDetected?: boolean;
}

/**
 * 微博帖子
 */
interface WeiboPost {
  id: string;
  text: string;
  created_at: string;
  author_uid: string;
  author_screen_name: string;
  reposts_count: number;
  comments_count: number;
  attitudes_count: number;
}

/**
 * 微博API响应
 */
interface WeiboApiResponse {
  ok: number;
  data?: {
    cards?: Array<{
      mblog?: {
        id?: string;
        mid?: string;
        text?: string;
        created_at?: string;
        user?: {
          id?: number | string;
          screen_name?: string;
        };
        reposts_count?: number;
        comments_count?: number;
        attitudes_count?: number;
      };
      card_type?: number;
    }>;
    cardlistInfo?: {
      total?: number;
    };
  };
  msg?: string;
}

// 会话管理
const activeSessions = new Map<string, BrowserContext>();

/**
 * 构建搜索URL
 */
function buildSearchUrl(request: CrawlRequest): string {
  const params = new URLSearchParams({
    containerid: '100103type=1',
    q: request.keyword,
    page: request.page.toString(),
  });

  if (request.startTime) {
    params.set('starttime', request.startTime);
  }

  if (request.endTime) {
    params.set('endtime', request.endTime);
  }

  return `https://m.weibo.cn/api/container/getIndex?${params.toString()}`;
}

/**
 * 清理文本内容
 */
function cleanText(text: string): string {
  if (!text) return '';

  // 移除HTML标签
  let cleaned = text.replace(/<[^>]*>/g, '');

  // 移除多余空白
  cleaned = cleaned.replace(/\s+/g, ' ').trim();

  return cleaned;
}

/**
 * 提取帖子ID
 */
function extractPostId(mblog: any): string {
  // 优先使用mid,否则使用id
  return String(mblog.mid || mblog.id || '');
}

/**
 * 提取用户UID
 */
function extractAuthorUid(user: any): string {
  if (!user) return '';
  return String(user.id || user.idstr || '');
}

/**
 * 带时间戳的日志输出
 */
function logWithTimestamp(sessionId: string, message: string, level: 'log' | 'warn' | 'error' = 'log') {
  const timestamp = new Date().toISOString();
  const logMessage = `[${timestamp}][${sessionId}] ${message}`;

  switch (level) {
    case 'warn':
      console.warn(logMessage);
      break;
    case 'error':
      console.error(logMessage);
      break;
    default:
      console.log(logMessage);
  }
}

/**
 * 爬取微博搜索
 */
async function crawlWeiboSearch(request: CrawlRequest): Promise<CrawlResult> {
  const sessionId = `crawl_${Date.now()}`;
  const startTime = Date.now();

  logWithTimestamp(sessionId, '---------- 创建浏览器上下文 ----------');

  const browser = await ensureBrowser();
  const userAgent = 'Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 Weibo (iPhone13,2__weibo__12.9.0__iphone__os16.0)';
  logWithTimestamp(sessionId, `UserAgent: ${userAgent}`);

  const context = await browser.newContext({ userAgent });
  activeSessions.set(sessionId, context);

  try {
    logWithTimestamp(sessionId, `关键字: ${request.keyword}`);
    logWithTimestamp(sessionId, `页码: ${request.page}`);
    if (request.startTime || request.endTime) {
      logWithTimestamp(sessionId, `时间范围: ${request.startTime || '无'} - ${request.endTime || '无'}`);
    }

    // 设置cookies
    const cookieArray = Object.entries(request.cookies).map(([name, value]) => ({
      name,
      value,
      domain: '.weibo.cn',
      path: '/',
    }));

    await context.addCookies(cookieArray);
    logWithTimestamp(sessionId, `Cookies数量: ${cookieArray.length}`);

    // 构建URL
    const url = buildSearchUrl(request);
    logWithTimestamp(sessionId, `请求URL: ${url}`);

    const page = await context.newPage();

    // 导航到搜索页面
    logWithTimestamp(sessionId, '正在加载页面...');
    const loadStartTime = Date.now();
    const response = await page.goto(url, {
      waitUntil: 'networkidle',
      timeout: REQUEST_TIMEOUT_MS,
    });

    if (!response) {
      throw new Error('页面加载失败: 无响应');
    }

    const loadDuration = Date.now() - loadStartTime;
    logWithTimestamp(sessionId, `页面加载完成, 状态码: ${response.status()}, 耗时: ${loadDuration}ms`);

    // 检测验证码
    logWithTimestamp(sessionId, '检测验证码...');
    const captchaVisible = await page.locator('text=验证码').isVisible().catch(() => false);
    if (captchaVisible) {
      const screenshotPath = `captcha_${Date.now()}.png`;
      await page.screenshot({ path: screenshotPath, fullPage: true });
      logWithTimestamp(sessionId, `检测到验证码, 截图已保存: ${screenshotPath}`, 'warn');

      return {
        posts: [],
        hasMore: false,
        captchaDetected: true,
      };
    }
    logWithTimestamp(sessionId, '未检测到验证码');

    // 获取页面内容
    const content = await page.content();
    logWithTimestamp(sessionId, `页面内容长度: ${content.length} 字符`);

    // 提取JSON数据
    logWithTimestamp(sessionId, '正在解析JSON数据...');
    let apiData: WeiboApiResponse;

    // 尝试从页面中提取JSON
    const preContent = await page.locator('pre').textContent().catch(() => null);
    if (preContent) {
      try {
        apiData = JSON.parse(preContent);
        logWithTimestamp(sessionId, '从<pre>标签解析JSON成功');
      } catch (e) {
        logWithTimestamp(sessionId, `解析<pre>内容失败: ${e instanceof Error ? e.message : String(e)}`, 'error');
        throw new Error('JSON解析失败');
      }
    } else {
      // 降级处理: 直接获取响应body
      try {
        const bodyContent = await response.text();
        apiData = JSON.parse(bodyContent);
        logWithTimestamp(sessionId, '从响应body解析JSON成功');
      } catch (e) {
        logWithTimestamp(sessionId, `解析响应body失败: ${e instanceof Error ? e.message : String(e)}`, 'error');
        throw new Error('无法获取API数据');
      }
    }

    // 验证响应
    if (apiData.ok !== 1) {
      throw new Error(`API返回错误: ${apiData.msg || '未知错误'}`);
    }

    if (!apiData.data) {
      logWithTimestamp(sessionId, 'API响应中无data字段', 'warn');
      return {
        posts: [],
        hasMore: false,
        totalResults: 0,
      };
    }

    // 提取帖子
    logWithTimestamp(sessionId, '开始提取帖子数据...');
    const posts: WeiboPost[] = [];
    const cards = apiData.data.cards || [];

    logWithTimestamp(sessionId, `JSON解析成功, cards数量: ${cards.length}`);

    let skippedCount = 0;
    for (const card of cards) {
      // 跳过非帖子类型的card
      if (!card.mblog) {
        skippedCount++;
        continue;
      }

      const mblog = card.mblog;
      const id = extractPostId(mblog);

      if (!id) {
        logWithTimestamp(sessionId, '跳过无效帖子(缺少ID)', 'warn');
        skippedCount++;
        continue;
      }

      const post: WeiboPost = {
        id,
        text: cleanText(mblog.text || ''),
        created_at: mblog.created_at || '',
        author_uid: extractAuthorUid(mblog.user),
        author_screen_name: mblog.user?.screen_name || '',
        reposts_count: mblog.reposts_count || 0,
        comments_count: mblog.comments_count || 0,
        attitudes_count: mblog.attitudes_count || 0,
      };

      posts.push(post);
    }

    logWithTimestamp(sessionId, `帖子提取完成, 有效帖子: ${posts.length}/${cards.length} (跳过: ${skippedCount})`);

    // 判断是否还有更多结果
    // 如果当前页返回的帖子数少于20,说明没有更多了
    const hasMore = posts.length >= 20;

    // 尝试获取总结果数
    const totalResults = apiData.data.cardlistInfo?.total;
    if (totalResults) {
      logWithTimestamp(sessionId, `总结果数: ${totalResults}`);
    }

    const totalDuration = Date.now() - startTime;
    logWithTimestamp(sessionId, `爬取完成, 总耗时: ${totalDuration}ms`);

    return {
      posts,
      hasMore,
      totalResults,
      captchaDetected: false,
    };

  } catch (error: any) {
    const totalDuration = Date.now() - startTime;
    logWithTimestamp(sessionId, '========== 爬取失败 ==========', 'error');
    logWithTimestamp(sessionId, `错误类型: ${error.constructor?.name || 'Unknown'}`, 'error');
    logWithTimestamp(sessionId, `错误消息: ${error.message || String(error)}`, 'error');
    logWithTimestamp(sessionId, `失败耗时: ${totalDuration}ms`, 'error');
    if (error.stack) {
      logWithTimestamp(sessionId, `错误堆栈: ${error.stack}`, 'error');
    }
    throw error;
  } finally {
    // 清理会话
    await context.close().catch(e => logWithTimestamp(sessionId, `关闭context失败: ${e instanceof Error ? e.message : String(e)}`, 'error'));
    activeSessions.delete(sessionId);
    logWithTimestamp(sessionId, '会话已清理');
  }
}

/**
 * 处理爬取请求
 */
export async function handleCrawlWeiboSearch(ws: WebSocket, payload: any): Promise<void> {
  const requestId = `req_${Date.now()}`;
  const requestStartTime = Date.now();

  try {
    if (!payload) {
      throw new Error('缺少payload');
    }

    const request: CrawlRequest = payload;

    // 验证必需字段
    if (!request.keyword || typeof request.keyword !== 'string') {
      throw new Error('无效的keyword');
    }

    if (!request.page || typeof request.page !== 'number') {
      throw new Error('无效的page');
    }

    if (!request.cookies || typeof request.cookies !== 'object') {
      throw new Error('无效的cookies');
    }

    logWithTimestamp(requestId, '========== 新的爬取请求 ==========');
    logWithTimestamp(requestId, `关键字: ${request.keyword}`);
    logWithTimestamp(requestId, `页码: ${request.page}`);
    if (request.startTime || request.endTime) {
      logWithTimestamp(requestId, `时间范围: ${request.startTime || '无'} - ${request.endTime || '无'}`);
    }
    logWithTimestamp(requestId, `Cookies数量: ${Object.keys(request.cookies).length}`);

    // 执行爬取
    const result = await crawlWeiboSearch(request);
    const requestDuration = Date.now() - requestStartTime;

    logWithTimestamp(requestId, '---------- 爬取完成 ----------');
    logWithTimestamp(requestId, `耗时: ${requestDuration}ms`);
    logWithTimestamp(requestId, `提取帖子数: ${result.posts.length}`);
    logWithTimestamp(requestId, `是否有更多: ${result.hasMore}`);
    logWithTimestamp(requestId, `验证码检测: ${result.captchaDetected || false}`);
    if (result.totalResults) {
      logWithTimestamp(requestId, `总结果数: ${result.totalResults}`);
    }

    // 返回统一成功响应
    if (ws.readyState === 1 /* WebSocket.OPEN */) {
      ws.send(JSON.stringify({
        request_id: requestId,
        success: true,
        data: result,
        timestamp: Date.now(),
      }));
      logWithTimestamp(requestId, '响应已发送到客户端');
    }

  } catch (error: any) {
    const requestDuration = Date.now() - requestStartTime;
    logWithTimestamp(requestId, '========== 请求处理失败 ==========', 'error');
    logWithTimestamp(requestId, `错误类型: ${error.constructor?.name || 'Unknown'}`, 'error');
    logWithTimestamp(requestId, `错误消息: ${error.message || String(error)}`, 'error');
    logWithTimestamp(requestId, `失败耗时: ${requestDuration}ms`, 'error');

    // 返回统一错误响应
    if (ws.readyState === 1 /* WebSocket.OPEN */) {
      ws.send(JSON.stringify({
        request_id: requestId,
        success: false,
        error: error.message || String(error),
        timestamp: Date.now(),
      }));
      logWithTimestamp(requestId, '错误响应已发送到客户端', 'error');
    }
  }
}

/**
 * 清理所有活跃会话
 */
export async function cleanupAllSessions(): Promise<void> {
  for (const [sessionId, context] of activeSessions) {
    try {
      await context.close();
      console.log(`会话已清理: ${sessionId}`);
    } catch (error) {
      console.error(`清理会话失败 ${sessionId}:`, error);
    }
  }
  activeSessions.clear();
}
