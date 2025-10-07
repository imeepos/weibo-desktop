/**
 * 共享类型定义
 */

import { BrowserContext as PlaywrightBrowserContext } from 'playwright';
import { WebSocket as WsWebSocket } from 'ws';

export type BrowserContext = PlaywrightBrowserContext;
export type WebSocket = WsWebSocket;
