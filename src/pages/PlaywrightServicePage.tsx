import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { XCircle, CheckCircle, AlertCircle } from 'lucide-react';
import { handleTauriError } from '../utils/errorHandler';
import { THEME, BUTTON } from '../constants/ui';

interface PlaywrightServiceStatus {
  running: boolean;
  pid?: number;
  port: number;
  healthy: boolean;
}

const STATUS_CHECK_INTERVAL_MS = 5000;

export const PlaywrightServicePage = () => {
  const [status, setStatus] = useState<PlaywrightServiceStatus | null>(null);
  const [logs, setLogs] = useState<string>('');
  const [showLogs, setShowLogs] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isLoadingLogs, setIsLoadingLogs] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const checkStatus = useCallback(async (showLoading = false) => {
    if (showLoading) setIsRefreshing(true);
    setError(null);

    try {
      const result = await invoke<PlaywrightServiceStatus>('check_playwright_server');
      setStatus(result);
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      if (showLoading) setIsRefreshing(false);
    }
  }, []);

  const startService = useCallback(async () => {
    setIsStarting(true);
    setError(null);

    try {
      await invoke('start_playwright_server');
      await checkStatus();
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      setIsStarting(false);
    }
  }, [checkStatus]);

  const stopService = useCallback(async () => {
    setIsStopping(true);
    setError(null);

    try {
      await invoke('stop_playwright_server');
      await checkStatus();
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      setIsStopping(false);
    }
  }, [checkStatus]);

  const loadLogs = useCallback(async () => {
    setIsLoadingLogs(true);
    setError(null);

    try {
      const logContent = await invoke<string>('get_playwright_logs');
      setLogs(logContent);
      setShowLogs(true);
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      setIsLoadingLogs(false);
    }
  }, []);

  useEffect(() => {
    checkStatus();
    const interval = setInterval(() => checkStatus(), STATUS_CHECK_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [checkStatus]);

  useEffect(() => {
    if (showLogs) {
      const logsContainer = document.getElementById('logs-container');
      if (logsContainer) {
        logsContainer.scrollTop = logsContainer.scrollHeight;
      }
    }
  }, [logs, showLogs]);

  const isOperationInProgress = isStarting || isStopping || isRefreshing;

  return (
    <div className={`${THEME.GRADIENT_BG} min-h-screen flex items-center justify-center p-4`}>
      <div className="max-w-2xl w-full space-y-6">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-gray-900">Playwright 服务管理</h1>
          <p className="mt-2 text-gray-600">管理和监控 Playwright 自动化服务</p>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <XCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
              <div className="flex-1">
                <p className="font-semibold text-red-900 mb-1">操作失败</p>
                <p className="text-red-700 text-sm">{error}</p>
              </div>
              <button
                onClick={() => setError(null)}
                className="text-red-600 hover:text-red-800 text-2xl leading-none"
              >
                ×
              </button>
            </div>
          </div>
        )}

        <div className={`${THEME.CARD_BG} p-6 space-y-6`}>
          <div className="space-y-4">
            <div className="flex items-center justify-between pb-4 border-b">
              <h2 className="text-xl font-semibold text-gray-800">服务状态</h2>
              <div className="flex items-center gap-2">
                <span
                  className={`w-3 h-3 rounded-full ${
                    status?.running ? 'bg-green-500 animate-pulse' : 'bg-red-500'
                  }`}
                />
                <span className={`font-semibold ${status?.running ? 'text-green-700' : 'text-red-700'}`}>
                  {status?.running ? '运行中' : '已停止'}
                </span>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="bg-gray-50 rounded-lg p-4">
                <p className="text-sm text-gray-600 mb-1">进程 ID</p>
                <p className="text-lg font-mono font-semibold text-gray-900">
                  {status?.pid ?? '-'}
                </p>
              </div>
              <div className="bg-gray-50 rounded-lg p-4">
                <p className="text-sm text-gray-600 mb-1">端口</p>
                <p className="text-lg font-mono font-semibold text-gray-900">
                  {status?.port ?? '-'}
                </p>
              </div>
              <div className="bg-gray-50 rounded-lg p-4 col-span-2">
                <p className="text-sm text-gray-600 mb-1">健康状态</p>
                <div className="flex items-center gap-2">
                  {status?.healthy ? (
                    <>
                      <CheckCircle className="w-5 h-5 text-green-600" />
                      <span className="text-lg font-semibold text-green-700">健康</span>
                    </>
                  ) : (
                    <>
                      <AlertCircle className="w-5 h-5 text-yellow-600" />
                      <span className="text-lg font-semibold text-yellow-700">未就绪</span>
                    </>
                  )}
                </div>
              </div>
            </div>
          </div>

          <div className="space-y-3 pt-4 border-t">
            <div className="flex gap-3">
              <button
                onClick={startService}
                disabled={status?.running || isOperationInProgress}
                className={`flex-1 ${BUTTON.PRIMARY} ${status?.running || isOperationInProgress ? 'opacity-50 cursor-not-allowed' : ''}`}
              >
                {isStarting ? '启动中...' : '启动服务'}
              </button>
              <button
                onClick={stopService}
                disabled={!status?.running || isOperationInProgress}
                className={`flex-1 ${BUTTON.WARNING} ${!status?.running || isOperationInProgress ? 'opacity-50 cursor-not-allowed' : ''}`}
              >
                {isStopping ? '停止中...' : '停止服务'}
              </button>
            </div>

            <div className="flex gap-3">
              <button
                onClick={() => checkStatus(true)}
                disabled={isOperationInProgress}
                className={`flex-1 ${BUTTON.SECONDARY} ${isOperationInProgress ? 'opacity-50 cursor-not-allowed' : ''}`}
              >
                {isRefreshing ? '刷新中...' : '刷新状态'}
              </button>
              <button
                onClick={loadLogs}
                disabled={isLoadingLogs}
                className={`flex-1 ${BUTTON.NAVIGATION_PRIMARY} justify-center ${isLoadingLogs ? 'opacity-50 cursor-not-allowed' : ''}`}
              >
                {isLoadingLogs ? '加载中...' : '查看日志'}
              </button>
            </div>
          </div>
        </div>

        {showLogs && (
          <div className={`${THEME.CARD_BG} p-6 space-y-4`}>
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-semibold text-gray-800">服务日志</h2>
              <button
                onClick={() => setShowLogs(false)}
                className="text-gray-600 hover:text-gray-800 text-2xl leading-none"
              >
                ×
              </button>
            </div>
            <div
              id="logs-container"
              className="bg-gray-900 text-green-400 font-mono text-xs p-4 rounded-lg overflow-y-auto max-h-96"
            >
              <pre className="whitespace-pre-wrap break-words">
                {logs || '暂无日志'}
              </pre>
            </div>
          </div>
        )}

        <div className="text-center text-sm text-gray-500">
          <p>状态每 5 秒自动刷新</p>
        </div>
      </div>
    </div>
  );
};
