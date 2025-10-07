/**
 * 微博统一服务 - WebSocket架构
 *
 * 职责:
 * - 统一WebSocket服务器入口(端口9223)
 * - 根据message.action路由到对应处理器
 * - 统一管理浏览器生命周期
 * - 统一响应格式
 *
 * 消息协议:
 * Client -> Server: { action: 'generate_qrcode' | 'crawl_weibo_search' | 'ping', payload?: any }
 * Server -> Client: 统一响应格式 | 登录事件流
 */

import { WebSocketServer, WebSocket } from 'ws';
import { handleGenerateQrcode, cleanupAllSessions as cleanupLoginSessions } from './handlers/login-handler';
import { handleCrawlWeiboSearch, cleanupAllSessions as cleanupCrawlerSessions } from './handlers/crawler-handler';
import { closeBrowser } from './browser';

const PORT = 9223;

/**
 * 统一消息格式
 */
interface WebSocketMessage {
  action: 'generate_qrcode' | 'crawl_weibo_search' | 'ping';
  payload?: any;
}

/**
 * 路由消息到对应处理器
 */
async function routeMessage(ws: WebSocket, message: WebSocketMessage): Promise<void> {
  const { action, payload } = message;

  console.log(`📋 路由消息: action=${action}`);

  switch (action) {
    case 'generate_qrcode':
      await handleGenerateQrcode(ws);
      break;

    case 'crawl_weibo_search':
      await handleCrawlWeiboSearch(ws, payload);
      break;

    case 'ping':
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
          success: true,
          data: { type: 'pong', timestamp: Date.now() }
        }));
      }
      break;

    default:
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
          success: false,
          error: `未知action: ${action}`,
          timestamp: Date.now()
        }));
      }
  }
}

// 启动统一WebSocket服务器
const wss = new WebSocketServer({ port: PORT });

wss.on('connection', (ws) => {
  console.log('🔗 新的WebSocket连接建立');

  ws.on('message', async (data) => {
    console.log('📨 收到WebSocket消息:', data.toString());

    try {
      const message = JSON.parse(data.toString()) as WebSocketMessage;

      // 向后兼容: 支持旧的 type 字段
      if (!message.action && (message as any).type) {
        message.action = (message as any).type;
      }

      if (!message.action) {
        throw new Error('缺少action字段');
      }

      await routeMessage(ws, message);

    } catch (error: any) {
      console.error('处理消息失败:', error);

      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
          success: false,
          error: error.message || '消息处理失败',
          timestamp: Date.now()
        }));
      }
    }
  });

  ws.on('close', () => {
    console.log('❌ WebSocket连接关闭');
  });

  ws.on('error', (error) => {
    console.error('WebSocket错误:', error);
  });
});

// 优雅关闭
process.on('SIGINT', async () => {
  console.log('收到SIGINT信号,清理资源...');

  // 清理所有活跃会话
  await cleanupLoginSessions();
  await cleanupCrawlerSessions();

  // 关闭浏览器
  await closeBrowser();

  // 关闭WebSocket服务器
  wss.close();
  console.log('WebSocket服务器已关闭');

  process.exit(0);
});

console.log(`微博统一服务已启动 - WebSocket端口: ${PORT}`);
console.log(`支持的actions: generate_qrcode, crawl_weibo_search, ping`);
