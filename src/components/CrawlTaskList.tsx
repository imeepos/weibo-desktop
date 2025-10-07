import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Search, Filter, RefreshCw } from 'lucide-react';
import { CrawlTaskSummary, CrawlStatus } from '../types/crawl';
import { handleTauriError } from '../utils/errorHandler';
import ErrorDisplay from './ErrorDisplay';
import { EmptyState } from './EmptyState';

interface CrawlTaskListProps {
  onTaskSelect: (taskId: string) => void;
  refreshTrigger?: number;
}

const statusConfig: Record<CrawlStatus, { label: string; color: string; animated: boolean }> = {
  Created: { label: '已创建', color: 'bg-gray-100 text-gray-700', animated: false },
  HistoryCrawling: { label: '历史回溯中', color: 'bg-blue-100 text-blue-700', animated: true },
  HistoryCompleted: { label: '历史完成', color: 'bg-green-100 text-green-700', animated: false },
  IncrementalCrawling: { label: '增量更新中', color: 'bg-blue-100 text-blue-700', animated: true },
  Paused: { label: '已暂停', color: 'bg-yellow-100 text-yellow-700', animated: false },
  Failed: { label: '失败', color: 'bg-red-100 text-red-700', animated: false },
};

const filterOptions: Array<{ value: CrawlStatus | 'All'; label: string }> = [
  { value: 'All', label: '全部' },
  { value: 'Created', label: '已创建' },
  { value: 'HistoryCrawling', label: '历史回溯中' },
  { value: 'HistoryCompleted', label: '历史完成' },
  { value: 'IncrementalCrawling', label: '增量更新中' },
  { value: 'Paused', label: '已暂停' },
  { value: 'Failed', label: '失败' },
];

export const CrawlTaskList = ({ onTaskSelect, refreshTrigger }: CrawlTaskListProps) => {
  const [tasks, setTasks] = useState<CrawlTaskSummary[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterStatus, setFilterStatus] = useState<CrawlStatus | 'All'>('All');
  const [searchKeyword, setSearchKeyword] = useState('');

  const loadTasks = async (abortSignal?: AbortSignal) => {
    // 防止并发请求
    if (isLoading) {
      console.log('[CrawlTaskList] 已有请求在进行中，跳过重复请求');
      return;
    }

    try {
      console.log('[CrawlTaskList] 开始加载任务列表...');
      setIsLoading(true);
      setError(null);

      const status = filterStatus === 'All' ? null : filterStatus;
      console.log('[CrawlTaskList] 发送请求:', { status, sortBy: 'updatedAt', sortOrder: 'desc' });

      // 检查是否已被取消
      if (abortSignal?.aborted) {
        console.log('[CrawlTaskList] 请求已被取消');
        return;
      }

      // 添加超时处理
      const timeoutPromise = new Promise((_, reject) => {
        setTimeout(() => reject(new Error('请求超时，请检查网络连接')), 10000);
      });

      const requestPromise = invoke<{tasks: CrawlTaskSummary[], total: number}>('list_crawl_tasks', {
        request: {
          status,
          sortBy: 'updatedAt',
          sortOrder: 'desc',
        },
      });

      const response = await Promise.race([requestPromise, timeoutPromise]) as {tasks: CrawlTaskSummary[], total: number};

      // 再次检查是否已被取消
      if (abortSignal?.aborted) {
        console.log('[CrawlTaskList] 请求完成但已被取消，忽略结果');
        return;
      }

      console.log('[CrawlTaskList] 收到响应:', response);
      console.log('[CrawlTaskList] 任务数量:', response.tasks?.length || 0);
      console.log('[CrawlTaskList] 总数:', response.total);

      // 严格验证响应数据格式
      if (!response || typeof response !== 'object') {
        throw new Error('服务器响应格式错误：响应不是有效对象');
      }

      if (!response.tasks || !Array.isArray(response.tasks)) {
        throw new Error('服务器响应格式错误：tasks字段不是有效数组');
      }

      if (typeof response.total !== 'number') {
        throw new Error('服务器响应格式错误：total字段不是有效数字');
      }

      // 验证任务数据结构
      const validTasks = response.tasks.filter(task => {
        if (!task || typeof task !== 'object') return false;
        if (!task.taskId || !task.keyword || !task.status) return false;
        if (typeof task.keyword !== 'string') return false;
        return true;
      });

      if (validTasks.length !== response.tasks.length) {
        console.warn('[CrawlTaskList] 部分任务数据格式不正确，已过滤');
      }

      // 最后检查是否已被取消
      if (abortSignal?.aborted) {
        console.log('[CrawlTaskList] 数据验证完成但请求已被取消，忽略结果');
        return;
      }

      setTasks(validTasks);
      console.log('[CrawlTaskList] 任务列表设置成功，任务:', validTasks.map(t => ({ id: t.taskId, keyword: t.keyword, status: t.status })));

    } catch (err) {
      // 如果是取消操作，不显示错误
      if (abortSignal?.aborted) {
        console.log('[CrawlTaskList] 请求因取消而失败，忽略错误');
        return;
      }

      console.error('[CrawlTaskList] 加载任务列表失败:', err);

      let errorMessage: string;
      if (err instanceof Error) {
        if (err.message.includes('请求超时')) {
          errorMessage = '请求超时，请检查网络连接后重试';
        } else if (err.message.includes('服务器响应格式错误')) {
          errorMessage = err.message;
        } else {
          errorMessage = handleTauriError(err);
        }
      } else {
        errorMessage = handleTauriError(err);
      }

      console.error('[CrawlTaskList] 处理后的错误消息:', errorMessage);
      setError(errorMessage);

      // 发生错误时清空任务列表，避免显示过期数据
      setTasks([]);

    } finally {
      // 只有在请求没有被取消的情况下才设置loading为false
      if (!abortSignal?.aborted) {
        console.log('[CrawlTaskList] 加载完成，设置loading为false');
        setIsLoading(false);
      } else {
        console.log('[CrawlTaskList] 请求被取消，保持loading状态');
      }
    }
  };

  useEffect(() => {
    const abortController = new AbortController();

    const loadTasksSafe = async () => {
      try {
        await loadTasks(abortController.signal);
      } catch (err) {
        if (!abortController.signal.aborted) {
          console.error('[CrawlTaskList] useEffect中发生未处理的错误:', err);
        }
      }
    };

    loadTasksSafe();

    return () => {
      abortController.abort();
      console.log('[CrawlTaskList] 组件卸载或依赖变化，取消进行中的请求');
      // 确保loading状态被重置
      setIsLoading(false);
    };
  }, [filterStatus, refreshTrigger]);

  const filteredTasks = searchKeyword
    ? tasks.filter(task => task.keyword.toLowerCase().includes(searchKeyword.toLowerCase()))
    : tasks;

  const formatDateTime = (isoString: string): string => {
    return new Date(isoString).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const StatusBadge = ({ status }: { status: CrawlStatus }) => {
    const config = statusConfig[status];
    return (
      <span className={`px-3 py-1 rounded-full text-sm font-medium ${config.color} ${config.animated ? 'animate-pulse' : ''}`}>
        {config.label}
      </span>
    );
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <RefreshCw className="w-8 h-8 text-blue-600 animate-spin" />
        <span className="ml-3 text-gray-600">加载任务列表...</span>
      </div>
    );
  }

  if (error) {
    return (
      <ErrorDisplay
        error={error}
        onRetry={loadTasks}
        showRetryButton={true}
        className="m-4"
      />
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-col sm:flex-row gap-4">
        <div className="flex-1 relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
          <input
            type="text"
            placeholder="搜索关键字..."
            value={searchKeyword}
            onChange={(e) => setSearchKeyword(e.target.value)}
            className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
        </div>

        <div className="flex items-center gap-2">
          <Filter className="w-5 h-5 text-gray-400" />
          <select
            value={filterStatus}
            onChange={(e) => setFilterStatus(e.target.value as CrawlStatus | 'All')}
            className="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          >
            {filterOptions.map(option => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        <button
          onClick={() => {
            // 取消当前请求并开始新的请求
            const abortController = new AbortController();
            loadTasks(abortController.signal);
          }}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors flex items-center gap-2"
        >
          <RefreshCw className="w-4 h-4" />
          刷新
        </button>
      </div>

      {filteredTasks.length === 0 ? (
        <EmptyState
          icon={Search}
          title={searchKeyword ? '未找到匹配的任务' : '暂无爬取任务'}
          description={searchKeyword ? '请尝试其他关键字' : '创建第一个爬取任务开始使用'}
        />
      ) : (
        <div className="grid gap-4">
          {filteredTasks.map(task => (
            <div
              key={task.taskId}
              onClick={() => onTaskSelect(task.taskId)}
              className="bg-white border border-gray-200 rounded-lg p-5 hover:border-blue-400 hover:shadow-md transition-all cursor-pointer"
            >
              <div className="flex items-start justify-between mb-3">
                <h3 className="text-lg font-semibold text-gray-900">{task.keyword}</h3>
                <StatusBadge status={task.status} />
              </div>

              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-gray-500">已爬取:</span>
                  <span className="ml-2 font-medium text-gray-900">{task.crawledCount.toLocaleString()} 条</span>
                </div>
                <div>
                  <span className="text-gray-500">创建时间:</span>
                  <span className="ml-2 text-gray-700">{formatDateTime(task.createdAt)}</span>
                </div>
              </div>

              <div className="mt-2 text-sm text-gray-500">
                最后更新: {formatDateTime(task.updatedAt)}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
