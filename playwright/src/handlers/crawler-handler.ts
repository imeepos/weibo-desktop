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
 * 爬取微博搜索
 */
async function crawlWeiboSearch(request: CrawlRequest): Promise<CrawlResult> {
  const browser = await ensureBrowser();
  const context = await browser.newContext({
    userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 Weibo (iPhone13,2__weibo__12.9.0__iphone__os16.0)',
  });

  const sessionId = `crawl_${Date.now()}`;
  activeSessions.set(sessionId, context);

  try {
    console.log(`[${sessionId}] 开始爬取: keyword="${request.keyword}", page=${request.page}`);

    // 设置cookies
    const cookieArray = Object.entries(request.cookies).map(([name, value]) => ({
      name,
      value,
      domain: '.weibo.cn',
      path: '/',
    }));

    await context.addCookies(cookieArray);
    console.log(`[${sessionId}] 已设置 ${cookieArray.length} 个cookies`);

    // 构建URL
    const url = buildSearchUrl(request);
    console.log(`[${sessionId}] 请求URL: ${url}`);

    const page = await context.newPage();

    // 导航到搜索页面
    const response = await page.goto(url, {
      waitUntil: 'networkidle',
      timeout: REQUEST_TIMEOUT_MS,
    });

    if (!response) {
      throw new Error('页面加载失败: 无响应');
    }

    console.log(`[${sessionId}] 响应状态: ${response.status()}`);

    // 检测验证码
    const captchaVisible = await page.locator('text=验证码').isVisible().catch(() => false);
    if (captchaVisible) {
      const screenshotPath = `captcha_${Date.now()}.png`;
      await page.screenshot({ path: screenshotPath, fullPage: true });
      console.warn(`[${sessionId}] 检测到验证码,截图已保存: ${screenshotPath}`);

      return {
        posts: [],
        hasMore: false,
        captchaDetected: true,
      };
    }

    // 获取页面内容
    const content = await page.content();
    console.log(`[${sessionId}] 页面内容长度: ${content.length} 字符`);

    // 提取JSON数据
    let apiData: WeiboApiResponse;

    // 尝试从页面中提取JSON
    const preContent = await page.locator('pre').textContent().catch(() => null);
    if (preContent) {
      try {
        apiData = JSON.parse(preContent);
        console.log(`[${sessionId}] 从<pre>标签解析JSON成功`);
      } catch (e) {
        console.error(`[${sessionId}] 解析<pre>内容失败:`, e);
        throw new Error('JSON解析失败');
      }
    } else {
      // 降级处理: 直接获取响应body
      try {
        const bodyContent = await response.text();
        apiData = JSON.parse(bodyContent);
        console.log(`[${sessionId}] 从响应body解析JSON成功`);
      } catch (e) {
        console.error(`[${sessionId}] 解析响应body失败:`, e);
        throw new Error('无法获取API数据');
      }
    }

    // 验证响应
    if (apiData.ok !== 1) {
      throw new Error(`API返回错误: ${apiData.msg || '未知错误'}`);
    }

    if (!apiData.data) {
      console.warn(`[${sessionId}] API响应中无data字段`);
      return {
        posts: [],
        hasMore: false,
        totalResults: 0,
      };
    }

    // 提取帖子
    const posts: WeiboPost[] = [];
    const cards = apiData.data.cards || [];

    console.log(`[${sessionId}] 获取到 ${cards.length} 个cards`);

    for (const card of cards) {
      // 跳过非帖子类型的card
      if (!card.mblog) {
        continue;
      }

      const mblog = card.mblog;
      const id = extractPostId(mblog);

      if (!id) {
        console.warn(`[${sessionId}] 跳过无效帖子(缺少ID)`);
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

    console.log(`[${sessionId}] 提取到 ${posts.length} 条有效帖子`);

    // 判断是否还有更多结果
    // 如果当前页返回的帖子数少于20,说明没有更多了
    const hasMore = posts.length >= 20;

    // 尝试获取总结果数
    const totalResults = apiData.data.cardlistInfo?.total;

    return {
      posts,
      hasMore,
      totalResults,
      captchaDetected: false,
    };

  } catch (error: any) {
    console.error(`[${sessionId}] 爬取失败:`, error);
    throw error;
  } finally {
    // 清理会话
    await context.close().catch(e => console.error(`[${sessionId}] 关闭context失败:`, e));
    activeSessions.delete(sessionId);
    console.log(`[${sessionId}] 会话已清理`);
  }
}

/**
 * 处理爬取请求
 */
export async function handleCrawlWeiboSearch(ws: WebSocket, payload: any): Promise<void> {
  const requestId = `req_${Date.now()}`;

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

    console.log(`[${requestId}] 收到爬取请求: keyword="${request.keyword}", page=${request.page}`);

    // 执行爬取
    const result = await crawlWeiboSearch(request);

    // 返回统一成功响应
    if (ws.readyState === 1 /* WebSocket.OPEN */) {
      ws.send(JSON.stringify({
        request_id: requestId,
        success: true,
        data: result,
        timestamp: Date.now(),
      }));
      console.log(`[${requestId}] 响应已发送: ${result.posts.length}条帖子, hasMore=${result.hasMore}`);
    }

  } catch (error: any) {
    console.error(`[${requestId}] 处理消息失败:`, error);

    // 返回统一错误响应
    if (ws.readyState === 1 /* WebSocket.OPEN */) {
      ws.send(JSON.stringify({
        request_id: requestId,
        success: false,
        error: error.message || String(error),
        timestamp: Date.now(),
      }));
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
