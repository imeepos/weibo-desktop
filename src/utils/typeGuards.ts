/**
 * 运行时类型验证工具
 *
 * 提供类型守卫函数，确保运行时数据的类型安全
 */

import type { CommandError, ErrorCode } from '../types/error';
import { CrawlErrorCode, LoginErrorCode } from '../types/error';
import type { CrawlProgressEvent, CrawlCompletedEvent, CrawlErrorEvent } from '../types/crawl';

/**
 * 检查对象是否为有效的CommandError
 */
export function isCommandError(obj: unknown): obj is CommandError {
  return (
    obj != null &&
    typeof obj === 'object' &&
    'code' in obj &&
    'error' in obj &&
    typeof (obj as CommandError).code === 'string' &&
    typeof (obj as CommandError).error === 'string'
  );
}

/**
 * 检查字符串是否为有效的错误代码
 */
export function isValidErrorCode(code: string): code is ErrorCode {
  // 检查爬取错误代码
  if (Object.values(CrawlErrorCode).includes(code as CrawlErrorCode)) {
    return true;
  }

  // 检查登录错误代码
  if (Object.values(LoginErrorCode).includes(code as LoginErrorCode)) {
    return true;
  }

  return false;
}

/**
 * 检查对象是否为有效的CrawlProgressEvent
 */
export function isCrawlProgressEvent(obj: unknown): obj is CrawlProgressEvent {
  return (
    obj != null &&
    typeof obj === 'object' &&
    'taskId' in obj &&
    'status' in obj &&
    'currentTimeRange' in obj &&
    'currentPage' in obj &&
    'crawledCount' in obj &&
    'timestamp' in obj &&
    typeof (obj as CrawlProgressEvent).taskId === 'string' &&
    typeof (obj as CrawlProgressEvent).status === 'string' &&
    typeof (obj as CrawlProgressEvent).currentPage === 'number' &&
    typeof (obj as CrawlProgressEvent).crawledCount === 'number' &&
    typeof (obj as CrawlProgressEvent).timestamp === 'string' &&
    isTimeRange((obj as CrawlProgressEvent).currentTimeRange)
  );
}

/**
 * 检查对象是否为有效的CrawlCompletedEvent
 */
export function isCrawlCompletedEvent(obj: unknown): obj is CrawlCompletedEvent {
  return (
    obj != null &&
    typeof obj === 'object' &&
    'taskId' in obj &&
    'finalStatus' in obj &&
    'totalCrawled' in obj &&
    'duration' in obj &&
    'timestamp' in obj &&
    typeof (obj as CrawlCompletedEvent).taskId === 'string' &&
    typeof (obj as CrawlCompletedEvent).finalStatus === 'string' &&
    typeof (obj as CrawlCompletedEvent).totalCrawled === 'number' &&
    typeof (obj as CrawlCompletedEvent).duration === 'number' &&
    typeof (obj as CrawlCompletedEvent).timestamp === 'string'
  );
}

/**
 * 检查对象是否为有效的CrawlErrorEvent
 */
export function isCrawlErrorEvent(obj: unknown): obj is CrawlErrorEvent {
  return (
    obj != null &&
    typeof obj === 'object' &&
    'taskId' in obj &&
    'error' in obj &&
    'errorCode' in obj &&
    'timestamp' in obj &&
    typeof (obj as CrawlErrorEvent).taskId === 'string' &&
    typeof (obj as CrawlErrorEvent).error === 'string' &&
    typeof (obj as CrawlErrorEvent).errorCode === 'string' &&
    typeof (obj as CrawlErrorEvent).timestamp === 'string' &&
    ['CAPTCHA_DETECTED', 'NETWORK_ERROR', 'STORAGE_ERROR'].includes((obj as CrawlErrorEvent).errorCode)
  );
}

/**
 * 检查对象是否为有效的时间范围
 */
export function isTimeRange(obj: unknown): obj is { start: string; end: string } {
  return (
    obj != null &&
    typeof obj === 'object' &&
    'start' in obj &&
    'end' in obj &&
    typeof (obj as { start: unknown; end: unknown }).start === 'string' &&
    typeof (obj as { start: unknown; end: unknown }).end === 'string'
  );
}

/**
 * 检查字符串是否为有效的ISO 8601时间戳
 */
export function isValidISOTimestamp(timestamp: string): boolean {
  const date = new Date(timestamp);
  return !isNaN(date.getTime()) && timestamp === date.toISOString();
}

/**
 * 检查对象是否为有效的任务状态
 */
export function isValidCrawlStatus(status: string): status is 'Created' | 'HistoryCrawling' | 'HistoryCompleted' | 'IncrementalCrawling' | 'Paused' | 'Failed' {
  return ['Created', 'HistoryCrawling', 'HistoryCompleted', 'IncrementalCrawling', 'Paused', 'Failed'].includes(status);
}

/**
 * 运行时验证并解析CommandError
 */
export function parseCommandError(data: unknown): CommandError | null {
  if (typeof data === 'string') {
    try {
      const parsed = JSON.parse(data);
      if (isCommandError(parsed)) {
        return parsed;
      }
    } catch {
      // 不是有效的JSON，继续下一步
    }
  }

  if (isCommandError(data)) {
    return data;
  }

  return null;
}

/**
 * 运行时验证并解析事件数据
 */
export function parseCrawlEventData(data: unknown): {
  type: 'progress' | 'completed' | 'error' | 'unknown';
  event: CrawlProgressEvent | CrawlCompletedEvent | CrawlErrorEvent | null;
} {
  if (isCrawlProgressEvent(data)) {
    return { type: 'progress', event: data };
  }

  if (isCrawlCompletedEvent(data)) {
    return { type: 'completed', event: data };
  }

  if (isCrawlErrorEvent(data)) {
    return { type: 'error', event: data };
  }

  return { type: 'unknown', event: null };
}