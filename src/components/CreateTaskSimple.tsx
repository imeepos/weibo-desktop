import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Plus, Calendar } from 'lucide-react';
import { handleTauriError } from '../utils/errorHandler';
import { CreateTaskRequestSimple } from '../types/crawl_simple';

interface CreateTaskSimpleProps {
  onTaskCreated?: (taskId: string) => void;
}

export const CreateTaskSimple = ({ onTaskCreated }: CreateTaskSimpleProps) => {
  const [isCreating, setIsCreating] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [keyword, setKeyword] = useState('');
  const [eventStartTime, setEventStartTime] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!keyword.trim()) {
      setError('请输入搜索关键字');
      return;
    }

    if (!eventStartTime) {
      setError('请选择事件开始时间');
      return;
    }

    setIsCreating(true);
    setError(null);

    try {
      const request: CreateTaskRequestSimple = {
        keyword: keyword.trim(),
        eventStartTime: new Date(eventStartTime).toISOString(),
      };

      const taskId = await invoke<string>('create_simple_crawl_task', {
        keyword: request.keyword,
        eventStartTime: eventStartTime,
      });

      console.log('[CreateTaskSimple] 任务创建成功:', taskId);

      // 重置表单
      setKeyword('');
      setEventStartTime('');
      setShowForm(false);

      // 通知父组件
      onTaskCreated?.(taskId);

    } catch (err) {
      console.error('[CreateTaskSimple] 创建任务失败:', err);
      setError(handleTauriError(err));
    } finally {
      setIsCreating(false);
    }
  };

  const getDefaultEventTime = (): string => {
    const date = new Date();
    date.setHours(date.getHours() - 24); // 默认24小时前
    return date.toISOString().slice(0, 16);
  };

  if (!showForm) {
    return (
      <button
        onClick={() => setShowForm(true)}
        className="w-full p-4 border-2 border-dashed border-gray-300 rounded-lg hover:border-blue-400 hover:bg-blue-50 transition-colors flex items-center justify-center gap-2 text-gray-600 hover:text-blue-600"
      >
        <Plus className="w-5 h-5" />
        <span>创建新的爬取任务</span>
      </button>
    );
  }

  return (
    <div className="bg-white border border-gray-200 rounded-lg p-6">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-gray-900">创建爬取任务</h3>
        <button
          onClick={() => {
            setShowForm(false);
            setError(null);
          }}
          className="text-gray-400 hover:text-gray-600"
        >
          ✕
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded text-sm text-red-700">
          {error}
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label htmlFor="keyword" className="block text-sm font-medium text-gray-700 mb-1">
            搜索关键字
          </label>
          <input
            id="keyword"
            type="text"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            placeholder="请输入要爬取的关键字"
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            disabled={isCreating}
          />
        </div>

        <div>
          <label htmlFor="eventStartTime" className="block text-sm font-medium text-gray-700 mb-1">
            事件开始时间
          </label>
          <div className="relative">
            <Calendar className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
            <input
              id="eventStartTime"
              type="datetime-local"
              value={eventStartTime || getDefaultEventTime()}
              onChange={(e) => setEventStartTime(e.target.value)}
              className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              disabled={isCreating}
            />
          </div>
          <p className="mt-1 text-sm text-gray-500">
            爬取将从此时间点开始，向现在进行增量更新
          </p>
        </div>

        <div className="flex gap-3 pt-2">
          <button
            type="submit"
            disabled={isCreating || !keyword.trim() || !eventStartTime}
            className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors flex items-center justify-center gap-2"
          >
            {isCreating ? (
              <>
                <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                创建中...
              </>
            ) : (
              <>
                <Plus className="w-4 h-4" />
                创建任务
              </>
            )}
          </button>

          <button
            type="button"
            onClick={() => {
              setShowForm(false);
              setError(null);
            }}
            disabled={isCreating}
            className="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 disabled:bg-gray-100 disabled:text-gray-400 transition-colors"
          >
            取消
          </button>
        </div>
      </form>
    </div>
  );
};