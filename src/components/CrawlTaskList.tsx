import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Search, Filter, RefreshCw } from 'lucide-react';
import { CrawlTaskSummary, CrawlStatus } from '../types/crawl';
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

  const loadTasks = async () => {
    try {
      setIsLoading(true);
      setError(null);

      const status = filterStatus === 'All' ? null : filterStatus;
      const result = await invoke<CrawlTaskSummary[]>('list_crawl_tasks', {
        status,
        sortBy: 'updated_at',
        sortOrder: 'desc',
      });

      setTasks(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载任务列表失败');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadTasks();
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
      <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">
        <p className="font-medium">加载失败</p>
        <p className="text-sm mt-1">{error}</p>
        <button
          onClick={loadTasks}
          className="mt-3 px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
        >
          重试
        </button>
      </div>
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
          onClick={loadTasks}
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
              key={task.id}
              onClick={() => onTaskSelect(task.id)}
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
