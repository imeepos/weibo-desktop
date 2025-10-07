import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Pause, Play, XCircle, Clock, FileText, TrendingUp, ChevronRight, AlertCircle } from 'lucide-react';
import {
  CrawlProgressEvent,
  CrawlCompletedEvent,
  CrawlErrorEvent,
  CrawlTask,
  CrawlCheckpoint
} from '../types/crawl';
import { handleTauriError } from '../utils/errorHandler';
import { THEME, BUTTON } from '../constants/ui';

interface CrawlProgressProps {
  taskId: string;
  onComplete?: () => void;
  onError?: (error: string) => void;
}

interface ProgressState {
  status: 'Preparing' | 'HistoryCrawling' | 'IncrementalCrawling' | 'Paused' | 'Completed' | 'Error';
  preparingMessage?: string;
  currentTimeRange: {
    start: string;
    end: string;
  } | null;
  currentPage: number;
  crawledCount: number;
  progressPercentage: number;
  errorMessage: string | null;
}

export const CrawlProgress = ({ taskId, onComplete, onError }: CrawlProgressProps) => {
  const [task, setTask] = useState<CrawlTask | null>(null);
  const [checkpoint, setCheckpoint] = useState<CrawlCheckpoint | null>(null);
  const [progress, setProgress] = useState<ProgressState>({
    status: 'Preparing',
    preparingMessage: '正在初始化任务...',
    currentTimeRange: null,
    currentPage: 0,
    crawledCount: 0,
    progressPercentage: 0,
    errorMessage: null,
  });
  const [isPausing, setIsPausing] = useState(false);
  const [isResuming, setIsResuming] = useState(false);
  const [isCancelling, setIsCancelling] = useState(false);
  const [lastUpdateTime, setLastUpdateTime] = useState<number>(Date.now());
  const [showTimeoutWarning, setShowTimeoutWarning] = useState(false);

  useEffect(() => {
    const fetchInitialData = async () => {
      try {
        const taskData = await invoke<CrawlTask>('get_crawl_task', { request: { taskId } });
        setTask(taskData);

        try {
          const checkpointData = await invoke<CrawlCheckpoint>('get_crawl_checkpoint', { request: { taskId } });
          setCheckpoint(checkpointData);
        } catch {
          // 检查点可能不存在
        }
      } catch (err) {
        const error = handleTauriError(err);
        setProgress(prev => ({ ...prev, status: 'Error', errorMessage: error }));
        onError?.(error);
      }
    };

    fetchInitialData();
  }, [taskId, onError]);

  useEffect(() => {
    const timer = setInterval(() => {
      if (progress.status === 'HistoryCrawling' || progress.status === 'IncrementalCrawling') {
        const timeSinceLastUpdate = Date.now() - lastUpdateTime;
        if (timeSinceLastUpdate > 30000) {
          setShowTimeoutWarning(true);
        }
      }
    }, 5000);

    return () => clearInterval(timer);
  }, [progress.status, lastUpdateTime]);

  useEffect(() => {
    const unlistenProgress = listen<CrawlProgressEvent>('crawl-progress', (event) => {
      if (event.payload.taskId !== taskId) return;

      setLastUpdateTime(Date.now());
      setShowTimeoutWarning(false);

      setProgress(prev => ({
        ...prev,
        status: event.payload.status,
        currentTimeRange: event.payload.currentTimeRange,
        currentPage: event.payload.currentPage,
        crawledCount: event.payload.crawledCount,
        progressPercentage: calculateProgressPercentage(event.payload),
      }));
    });

    const unlistenCompleted = listen<CrawlCompletedEvent>('crawl-completed', (event) => {
      if (event.payload.taskId !== taskId) return;

      setProgress(prev => ({
        ...prev,
        status: 'Completed',
        crawledCount: event.payload.totalCrawled,
        progressPercentage: 100,
      }));

      onComplete?.();
    });

    const unlistenError = listen<CrawlErrorEvent>('crawl-error', (event) => {
      if (event.payload.taskId !== taskId) return;

      const errorMessage = `${event.payload.error} (${event.payload.errorCode})`;
      setProgress(prev => ({
        ...prev,
        status: 'Error',
        errorMessage,
      }));

      onError?.(errorMessage);
    });

    return () => {
      unlistenProgress.then(f => f());
      unlistenCompleted.then(f => f());
      unlistenError.then(f => f());
    };
  }, [taskId, onComplete, onError]);

  const calculateProgressPercentage = (event: CrawlProgressEvent): number => {
    if (!task) return 0;

    const eventStart = new Date(task.eventStartTime).getTime();
    const now = Date.now();
    const currentEnd = new Date(event.currentTimeRange.end).getTime();

    const totalDuration = now - eventStart;
    const progressedDuration = currentEnd - eventStart;

    return Math.min(Math.max((progressedDuration / totalDuration) * 100, 0), 100);
  };

  const handlePause = async () => {
    setIsPausing(true);
    try {
      await invoke('pause_crawl', { request: { taskId } });
      setProgress(prev => ({ ...prev, status: 'Paused' }));
    } catch (err) {
      const error = handleTauriError(err);
      setProgress(prev => ({ ...prev, status: 'Error', errorMessage: error }));
      onError?.(error);
    } finally {
      setIsPausing(false);
    }
  };

  const handleResume = async () => {
    setIsResuming(true);
    try {
      await invoke('start_crawl', { request: { taskId } });
      setProgress(prev => ({ ...prev, status: 'HistoryCrawling' }));
    } catch (err) {
      const error = handleTauriError(err);
      setProgress(prev => ({ ...prev, status: 'Error', errorMessage: error }));
      onError?.(error);
    } finally {
      setIsResuming(false);
    }
  };

  const handleCancel = async () => {
    setIsCancelling(true);
    try {
      await invoke('cancel_crawl', { request: { taskId } });
      setProgress(prev => ({ ...prev, status: 'Paused' }));
    } catch (err) {
      const error = handleTauriError(err);
      setProgress(prev => ({ ...prev, status: 'Error', errorMessage: error }));
      onError?.(error);
    } finally {
      setIsCancelling(false);
    }
  };

  const formatDateTime = (isoString: string): string => {
    return new Date(isoString).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  const getStatusColor = (): string => {
    switch (progress.status) {
      case 'Preparing':
        return 'text-purple-700 bg-purple-50';
      case 'HistoryCrawling':
      case 'IncrementalCrawling':
        return 'text-blue-700 bg-blue-50';
      case 'Paused':
        return 'text-yellow-700 bg-yellow-50';
      case 'Completed':
        return 'text-green-700 bg-green-50';
      case 'Error':
        return 'text-red-700 bg-red-50';
      default:
        return 'text-gray-700 bg-gray-50';
    }
  };

  const getStatusText = (): string => {
    switch (progress.status) {
      case 'Preparing':
        return '准备中';
      case 'HistoryCrawling':
        return '历史回溯中';
      case 'IncrementalCrawling':
        return '增量更新中';
      case 'Paused':
        return '已暂停';
      case 'Completed':
        return '已完成';
      case 'Error':
        return '爬取失败';
      default:
        return '未知状态';
    }
  };

  if (!task) {
    return (
      <div className={`${THEME.CARD_BG} p-6`}>
        <div className="animate-pulse space-y-4">
          <div className="h-4 bg-gray-200 rounded w-3/4"></div>
          <div className="h-4 bg-gray-200 rounded w-1/2"></div>
        </div>
      </div>
    );
  }

  return (
    <div className={`${THEME.CARD_BG} p-6 space-y-6`}>
      <div className="flex items-center justify-between pb-4 border-b">
        <div>
          <h2 className="text-xl font-semibold text-gray-800">爬取进度</h2>
          <p className="text-sm text-gray-600 mt-1">关键字: {task.keyword}</p>
        </div>
        <div className={`px-4 py-2 rounded-lg font-semibold ${getStatusColor()}`}>
          {getStatusText()}
        </div>
      </div>

      {progress.status === 'Preparing' && progress.preparingMessage && (
        <div className="bg-purple-50 border border-purple-200 rounded-lg p-4">
          <div className="flex items-center gap-3">
            <Clock className="w-5 h-5 text-purple-600 animate-spin" />
            <div>
              <p className="font-semibold text-purple-900">任务准备中</p>
              <p className="text-purple-700 text-sm mt-1">{progress.preparingMessage}</p>
            </div>
          </div>
        </div>
      )}

      {showTimeoutWarning && (
        <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4">
          <div className="flex items-start gap-3">
            <AlertCircle className="w-5 h-5 text-yellow-600 flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-yellow-900">进度更新较慢</p>
              <p className="text-yellow-700 text-sm">
                爬取任务仍在运行，但30秒内未收到更新。这可能是因为：
              </p>
              <ul className="list-disc ml-5 mt-1 text-yellow-700 text-sm">
                <li>时间分片计算耗时较长</li>
                <li>网络请求响应较慢</li>
                <li>当前页面帖子较少</li>
              </ul>
            </div>
          </div>
        </div>
      )}

      {progress.errorMessage && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <div className="flex items-start gap-3">
            <XCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <p className="font-semibold text-red-900 mb-1">爬取错误</p>
              <p className="text-red-700 text-sm">{progress.errorMessage}</p>
            </div>
          </div>
        </div>
      )}

      <div className="space-y-4">
        {(progress.status === 'Preparing' || (progress.status === 'HistoryCrawling' && progress.currentPage === 0)) && (
          <div className="flex items-center gap-2 text-purple-700 mb-4">
            <div className="flex gap-1">
              <div className="w-2 h-2 bg-purple-600 rounded-full animate-bounce" style={{ animationDelay: '0ms' }}></div>
              <div className="w-2 h-2 bg-purple-600 rounded-full animate-bounce" style={{ animationDelay: '150ms' }}></div>
              <div className="w-2 h-2 bg-purple-600 rounded-full animate-bounce" style={{ animationDelay: '300ms' }}></div>
            </div>
            <span className="text-sm">正在准备爬取任务...</span>
          </div>
        )}

        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-700">进度</span>
            <span className="text-sm font-semibold text-blue-700">
              {progress.progressPercentage.toFixed(1)}%
            </span>
          </div>
          <div className="w-full h-3 bg-gray-200 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-blue-500 to-blue-600 transition-all duration-500 ease-out"
              style={{ width: `${progress.progressPercentage}%` }}
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="bg-gray-50 rounded-lg p-4">
            <div className="flex items-center gap-2 mb-2">
              <FileText className="w-4 h-4 text-gray-600" />
              <p className="text-sm text-gray-600">当前页码</p>
            </div>
            <p className="text-2xl font-semibold text-gray-900">
              {progress.currentPage === 0 ? '—' : progress.currentPage}
            </p>
          </div>

          <div className="bg-gray-50 rounded-lg p-4">
            <div className="flex items-center gap-2 mb-2">
              <TrendingUp className="w-4 h-4 text-gray-600" />
              <p className="text-sm text-gray-600">已爬取数量</p>
            </div>
            <p className="text-2xl font-semibold text-gray-900">
              {progress.crawledCount === 0 && progress.status === 'HistoryCrawling'
                ? '准备中...'
                : progress.crawledCount.toLocaleString()}
            </p>
          </div>
        </div>

        {progress.currentTimeRange && (
          <div className="bg-blue-50 rounded-lg p-4">
            <div className="flex items-center gap-2 mb-3">
              <Clock className="w-4 h-4 text-blue-600" />
              <p className="text-sm font-medium text-blue-900">当前时间范围</p>
            </div>
            <div className="flex items-center gap-2 text-sm">
              <span className="font-mono text-blue-900">
                {formatDateTime(progress.currentTimeRange.start)}
              </span>
              <ChevronRight className="w-4 h-4 text-blue-600" />
              <span className="font-mono text-blue-900">
                {formatDateTime(progress.currentTimeRange.end)}
              </span>
            </div>
          </div>
        )}

        {checkpoint && (
          <div className="bg-purple-50 rounded-lg p-4 space-y-2">
            <p className="text-sm font-medium text-purple-900">检查点信息</p>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div>
                <span className="text-purple-700">方向: </span>
                <span className="font-semibold text-purple-900">
                  {checkpoint.direction === 'Backward' ? '向后回溯' : '向前更新'}
                </span>
              </div>
              <div>
                <span className="text-purple-700">已完成分片: </span>
                <span className="font-semibold text-purple-900">
                  {checkpoint.completedShards.length}
                </span>
              </div>
            </div>
            <div className="text-xs text-purple-700">
              保存时间: {formatDateTime(checkpoint.savedAt)}
            </div>
          </div>
        )}
      </div>

      <div className="flex gap-3 pt-4 border-t">
        {progress.status === 'Paused' ? (
          <button
            onClick={handleResume}
            disabled={isResuming}
            className={`flex-1 ${BUTTON.PRIMARY} flex items-center justify-center gap-2 ${
              isResuming ? 'opacity-50 cursor-not-allowed' : ''
            }`}
          >
            <Play className="w-4 h-4" />
            {isResuming ? '恢复中...' : '恢复爬取'}
          </button>
        ) : (
          <button
            onClick={handlePause}
            disabled={
              isPausing ||
              !(progress.status === 'HistoryCrawling' || progress.status === 'IncrementalCrawling')
            }
            className={`flex-1 ${BUTTON.WARNING} flex items-center justify-center gap-2 ${
              isPausing || progress.status === 'Completed' || progress.status === 'Error'
                ? 'opacity-50 cursor-not-allowed'
                : ''
            }`}
          >
            <Pause className="w-4 h-4" />
            {isPausing ? '暂停中...' : '暂停爬取'}
          </button>
        )}

        <button
          onClick={handleCancel}
          disabled={
            isCancelling ||
            progress.status === 'Completed' ||
            progress.status === 'Error'
          }
          className={`flex-1 bg-red-600 hover:bg-red-700 text-white font-semibold py-3 px-4 rounded-lg transition-colors flex items-center justify-center gap-2 ${
            isCancelling || progress.status === 'Completed' || progress.status === 'Error'
              ? 'opacity-50 cursor-not-allowed'
              : ''
          }`}
        >
          <XCircle className="w-4 h-4" />
          {isCancelling ? '取消中...' : '取消任务'}
        </button>
      </div>
    </div>
  );
};
