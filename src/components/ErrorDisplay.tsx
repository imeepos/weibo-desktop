import { AlertTriangle, RefreshCw, Info, XCircle } from 'lucide-react';
import { extractErrorCode } from '../utils/errorHandler';
import { CrawlErrorCode } from '../types/error';

interface ErrorDisplayProps {
  error: string | null;
  onRetry?: () => void;
  onDismiss?: () => void;
  className?: string;
  showRetryButton?: boolean;
  showDismissButton?: boolean;
}

/**
 * 统一的错误提示组件
 * 提供用户友好的错误信息展示和操作按钮
 */
export const ErrorDisplay = ({
  error,
  onRetry,
  onDismiss,
  className = '',
  showRetryButton = false,
  showDismissButton = false,
}: ErrorDisplayProps) => {
  if (!error) return null;

  const errorCode = extractErrorCode(error);
  const isRetryableError = errorCode && [
    CrawlErrorCode.STORAGE_ERROR,
    CrawlErrorCode.FILE_SYSTEM_ERROR,
  ].includes(errorCode as CrawlErrorCode);

  const getErrorIcon = () => {
    if (errorCode === CrawlErrorCode.INVALID_TIME) {
      return <Info className="w-5 h-5 text-blue-500" />;
    }
    return <AlertTriangle className="w-5 h-5 text-red-500" />;
  };

  const getErrorSeverity = () => {
    if (errorCode === CrawlErrorCode.INVALID_TIME || errorCode === CrawlErrorCode.INVALID_FORMAT) {
      return 'warning';
    }
    if (errorCode === CrawlErrorCode.COOKIES_EXPIRED || errorCode === CrawlErrorCode.COOKIES_NOT_FOUND) {
      return 'info';
    }
    return 'error';
  };

  const severity = getErrorSeverity();
  const baseClasses = 'rounded-lg p-4 mb-4 flex items-start gap-3 transition-all duration-200';
  const severityClasses = {
    error: 'bg-red-50 border border-red-200 text-red-800',
    warning: 'bg-yellow-50 border border-yellow-200 text-yellow-800',
    info: 'bg-blue-50 border border-blue-200 text-blue-800',
  };

  return (
    <div className={`${baseClasses} ${severityClasses[severity]} ${className}`}>
      {getErrorIcon()}

      <div className="flex-1 min-w-0">
        <div className="font-medium text-sm mb-1">
          {getErrorMessageTitle(errorCode)}
        </div>
        <div className="text-sm whitespace-pre-line">
          {error}
        </div>

        {/* 添加错误提示 */}
        {errorCode && getErrorHint(errorCode) && (
          <div className="mt-2 text-xs opacity-75">
            💡 {getErrorHint(errorCode)}
          </div>
        )}
      </div>

      <div className="flex gap-2 ml-2">
        {showRetryButton && onRetry && isRetryableError && (
          <button
            onClick={onRetry}
            className="inline-flex items-center gap-1 px-3 py-1 text-xs font-medium rounded-md bg-white bg-opacity-60 hover:bg-opacity-100 transition-colors"
          >
            <RefreshCw className="w-3 h-3" />
            重试
          </button>
        )}

        {showDismissButton && onDismiss && (
          <button
            onClick={onDismiss}
            className="inline-flex items-center p-1 rounded-md hover:bg-white hover:bg-opacity-20 transition-colors"
          >
            <XCircle className="w-4 h-4" />
          </button>
        )}
      </div>
    </div>
  );
};

/**
 * 根据错误代码获取错误标题
 */
function getErrorMessageTitle(errorCode: string | null): string {
  if (!errorCode) return '发生错误';

  const titles: Record<string, string> = {
    [CrawlErrorCode.INVALID_KEYWORD]: '输入验证错误',
    [CrawlErrorCode.INVALID_TIME]: '时间设置错误',
    [CrawlErrorCode.COOKIES_NOT_FOUND]: '需要登录',
    [CrawlErrorCode.COOKIES_EXPIRED]: '登录已过期',
    [CrawlErrorCode.TASK_NOT_FOUND]: '任务不存在',
    [CrawlErrorCode.INVALID_STATUS]: '操作不可用',
    [CrawlErrorCode.ALREADY_RUNNING]: '任务冲突',
    [CrawlErrorCode.NO_DATA]: '暂无数据',
    [CrawlErrorCode.INVALID_FORMAT]: '格式错误',
    [CrawlErrorCode.STORAGE_ERROR]: '数据存储错误',
    [CrawlErrorCode.FILE_SYSTEM_ERROR]: '文件操作错误',
  };

  return titles[errorCode] || '系统错误';
}

/**
 * 根据错误代码获取用户提示
 */
function getErrorHint(errorCode: string): string | null {
  const hints: Record<string, string> = {
    [CrawlErrorCode.COOKIES_NOT_FOUND]: '请先在登录页面扫码登录微博账号',
    [CrawlErrorCode.COOKIES_EXPIRED]: '请重新登录微博账号以获取最新的Cookies',
    [CrawlErrorCode.INVALID_TIME]: '请检查开始时间格式，确保使用有效的日期时间',
    [CrawlErrorCode.NO_DATA]: '请等待爬取任务开始或检查搜索关键字是否正确',
    [CrawlErrorCode.ALREADY_RUNNING]: '请先暂停当前正在运行的任务，或等待其完成',
    [CrawlErrorCode.FILE_SYSTEM_ERROR]: '请检查磁盘空间和文件写入权限',
  };

  return hints[errorCode] || null;
}

export default ErrorDisplay;