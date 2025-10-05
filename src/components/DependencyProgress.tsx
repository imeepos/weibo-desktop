import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

// 事件接口定义
export interface DependencyCheckProgressEvent {
  dependency_id: string;
  dependency_name: string;
  current_index: number;
  total_count: number;
  status: 'checking' | 'satisfied' | 'missing' | 'version_mismatch' | 'corrupted';
  detected_version?: string;
  error_details?: string;
}

export interface InstallationProgressEvent {
  task_id: string;
  dependency_id: string;
  dependency_name: string;
  status: 'pending' | 'downloading' | 'installing' | 'success' | 'failed';
  progress_percent: number;
  log_entry?: string;
}

interface DependencyProgressProps {
  currentIndex?: number;
  totalCount?: number;
  currentDep?: string;
  status?: string;
}

export const DependencyProgress = ({
  currentIndex: propCurrentIndex,
  totalCount: propTotalCount,
  currentDep: propCurrentDep,
  status: propStatus,
}: DependencyProgressProps) => {
  // 内部状态，用于事件驱动的更新
  const [currentIndex, setCurrentIndex] = useState(propCurrentIndex || 0);
  const [totalCount, setTotalCount] = useState(propTotalCount || 0);
  const [currentDep, setCurrentDep] = useState(propCurrentDep || '');
  const [status, setStatus] = useState(propStatus || 'checking');
  const [detectedVersion, setDetectedVersion] = useState<string>('');
  const [errorDetails, setErrorDetails] = useState<string>('');
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    // 如果通过Props传入，则使用Props值计算进度
    if (propTotalCount && propCurrentIndex !== undefined) {
      const calculatedProgress = Math.round((propCurrentIndex / propTotalCount) * 100);
      setProgress(calculatedProgress);
      setCurrentDep(propCurrentDep || '');
      setStatus(propStatus || 'checking');
    }
  }, [propCurrentIndex, propTotalCount, propCurrentDep, propStatus]);

  useEffect(() => {
    let unlistenCheck: (() => void) | null = null;
    let unlistenInstall: (() => void) | null = null;

    const setupEventListeners = async () => {
      try {
        // 监听依赖检测进度事件
        unlistenCheck = await listen<DependencyCheckProgressEvent>(
          'dependency-check-progress',
          (event) => {
            const payload = event.payload;
            setCurrentIndex(payload.current_index);
            setTotalCount(payload.total_count);
            setCurrentDep(payload.dependency_name);
            setStatus(payload.status);
            setDetectedVersion(payload.detected_version || '');
            setErrorDetails(payload.error_details || '');

            // 计算进度百分比
            const calculatedProgress = Math.round((payload.current_index / payload.total_count) * 100);
            setProgress(calculatedProgress);
          }
        );

        // 监听安装进度事件
        unlistenInstall = await listen<InstallationProgressEvent>(
          'installation-progress',
          (event) => {
            const payload = event.payload;
            setCurrentDep(payload.dependency_name);
            setStatus(payload.status);
            setProgress(payload.progress_percent);

            if (payload.log_entry) {
              console.log(`[${payload.dependency_name}] ${payload.log_entry}`);
            }
          }
        );
      } catch (error) {
        console.error('设置事件监听器失败:', error);
      }
    };

    setupEventListeners();

    return () => {
      unlistenCheck?.();
      unlistenInstall?.();
    };
  }, []);

  const getStatusText = (): string => {
    switch (status) {
      case 'checking':
        return '正在检测';
      case 'installing':
      case 'downloading':
        return '正在安装';
      case 'satisfied':
        return '检测完成';
      case 'missing':
        return '依赖缺失';
      case 'version_mismatch':
        return '版本不匹配';
      case 'corrupted':
        return '依赖损坏';
      case 'success':
        return '安装成功';
      case 'failed':
        return '安装失败';
      case 'pending':
        return '等待安装';
      default:
        return '准备中';
    }
  };

  const getStatusColor = (): string => {
    switch (status) {
      case 'checking':
        return 'bg-blue-500';
      case 'installing':
      case 'downloading':
      case 'pending':
        return 'bg-orange-500';
      case 'satisfied':
      case 'success':
        return 'bg-green-500';
      case 'missing':
      case 'version_mismatch':
      case 'corrupted':
      case 'failed':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getTextColor = (): string => {
    switch (status) {
      case 'checking':
        return 'text-blue-600';
      case 'installing':
      case 'downloading':
      case 'pending':
        return 'text-orange-600';
      case 'satisfied':
      case 'success':
        return 'text-green-600';
      case 'missing':
      case 'version_mismatch':
      case 'corrupted':
      case 'failed':
        return 'text-red-600';
      default:
        return 'text-gray-600';
    }
  };

  const getProgressBarAnimation = (): string => {
    if (status === 'checking' || status === 'installing' || status === 'downloading') {
      return 'transition-all duration-300 ease-out';
    }
    return 'transition-all duration-500 ease-in-out';
  };

  return (
    <div className="w-full max-w-2xl mx-auto p-6 bg-white rounded-lg shadow-lg">
      {/* 进度条容器 */}
      <div className="mb-4">
        <div className="flex justify-between items-center mb-2">
          <h3 className={`text-lg font-semibold ${getTextColor()}`}>
            {getStatusText()}
          </h3>
          <span className="text-sm text-gray-600">
            {currentIndex} / {totalCount}
          </span>
        </div>

        {/* 进度条 */}
        <div className="relative w-full h-3 bg-gray-200 rounded-full overflow-hidden">
          <div
            className={`absolute top-0 left-0 h-full ${getStatusColor()} ${getProgressBarAnimation()}`}
            style={{ width: `${progress}%` }}
          >
            {/* 进度条动画效果 */}
            {(status === 'checking' || status === 'installing' || status === 'downloading') && (
              <div className="absolute inset-0 bg-white opacity-20 animate-pulse"></div>
            )}
          </div>
        </div>

        {/* 百分比显示 */}
        <div className="text-right mt-1">
          <span className="text-xs text-gray-500">{progress}%</span>
        </div>
      </div>

      {/* 当前项目信息 */}
      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${getStatusColor()}`}></div>
          <span className="text-sm font-medium text-gray-700">
            {currentDep || '准备中...'}
          </span>
        </div>

        {/* 检测到的版本信息 */}
        {detectedVersion && (
          <div className="text-xs text-gray-500 ml-4">
            检测到版本: {detectedVersion}
          </div>
        )}

        {/* 错误详情 */}
        {errorDetails && (
          <div className="text-xs text-red-600 ml-4 bg-red-50 p-2 rounded border border-red-200">
            错误: {errorDetails}
          </div>
        )}
      </div>

      {/* 状态指示器 */}
      <div className="mt-4 flex justify-center">
        <div className="flex items-center gap-2 text-xs text-gray-500">
          {(status === 'checking' || status === 'installing' || status === 'downloading') && (
            <>
              <div className="w-1 h-1 bg-gray-400 rounded-full animate-bounce"></div>
              <div className="w-1 h-1 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '0.1s' }}></div>
              <div className="w-1 h-1 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '0.2s' }}></div>
              <span className="ml-2">处理中...</span>
            </>
          )}
        </div>
      </div>

      {/* 快速状态说明 */}
      <div className="mt-4 pt-4 border-t border-gray-200">
        <div className="grid grid-cols-2 gap-2 text-xs text-gray-500">
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 bg-blue-500 rounded-full"></div>
            <span>检测中</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 bg-orange-500 rounded-full"></div>
            <span>安装中</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 bg-green-500 rounded-full"></div>
            <span>已完成</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 bg-red-500 rounded-full"></div>
            <span>需要处理</span>
          </div>
        </div>
      </div>
    </div>
  );
};