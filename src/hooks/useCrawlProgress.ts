import { useState, useEffect, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CrawlProgressEvent, CrawlCompletedEvent, CrawlErrorEvent } from '../types/crawl';

interface UseCrawlProgressReturn {
  progress: CrawlProgressEvent | null;
  completed: CrawlCompletedEvent | null;
  error: CrawlErrorEvent | null;
  clearProgress: () => void;
}

/**
 * 监听爬取进度事件
 *
 * @param taskId - 可选的任务ID过滤器，仅监听指定任务的事件
 * @returns 包含进度、完成、错误状态及清除函数
 */
export const useCrawlProgress = (taskId?: string): UseCrawlProgressReturn => {
  const [progress, setProgress] = useState<CrawlProgressEvent | null>(null);
  const [completed, setCompleted] = useState<CrawlCompletedEvent | null>(null);
  const [error, setError] = useState<CrawlErrorEvent | null>(null);

  const clearProgress = useCallback(() => {
    setProgress(null);
    setCompleted(null);
    setError(null);
  }, []);

  useEffect(() => {
    const unlistenFns: UnlistenFn[] = [];

    const setupListeners = async () => {
      const progressUnlisten = await listen<CrawlProgressEvent>('crawl-progress', (event) => {
        if (!taskId || event.payload.taskId === taskId) {
          setProgress(event.payload);
        }
      });

      const completedUnlisten = await listen<CrawlCompletedEvent>('crawl-completed', (event) => {
        if (!taskId || event.payload.taskId === taskId) {
          setCompleted(event.payload);
          setProgress(null);
        }
      });

      const errorUnlisten = await listen<CrawlErrorEvent>('crawl-error', (event) => {
        if (!taskId || event.payload.taskId === taskId) {
          setError(event.payload);
          setProgress(null);
        }
      });

      unlistenFns.push(progressUnlisten, completedUnlisten, errorUnlisten);
    };

    setupListeners();

    return () => {
      unlistenFns.forEach((unlisten) => unlisten());
    };
  }, [taskId]);

  return { progress, completed, error, clearProgress };
};
