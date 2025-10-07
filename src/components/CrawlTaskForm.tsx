import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertCircle, Loader2 } from 'lucide-react';
import { handleTauriError } from '../utils/errorHandler';
import { BUTTON, THEME } from '../constants/ui';

interface CrawlTaskFormData {
  keyword: string;
  eventStartTime: string;
  uid: string;
}

interface CreateCrawlTaskResponse {
  taskId: string;
  createdAt: string;
}

interface CrawlTaskFormProps {
  onTaskCreated?: (taskId: string) => void;
}

const MAX_EVENT_START_TIME = new Date().toISOString().slice(0, 16);

export const CrawlTaskForm = ({ onTaskCreated }: CrawlTaskFormProps) => {
  const [formData, setFormData] = useState<CrawlTaskFormData>({
    keyword: '',
    eventStartTime: '',
    uid: '',
  });

  const [availableUids, setAvailableUids] = useState<string[]>([]);
  const [isLoadingUids, setIsLoadingUids] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [validationErrors, setValidationErrors] = useState<Partial<Record<keyof CrawlTaskFormData, string>>>({});

  const loadAvailableUids = useCallback(async () => {
    setIsLoadingUids(true);
    setError(null);

    try {
      const uids = await invoke<string[]>('query_cookies');
      setAvailableUids(uids);

      if (uids.length === 0) {
        setError('暂无可用账号，请先完成微博扫码登录');
      }
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      setIsLoadingUids(false);
    }
  }, []);

  useEffect(() => {
    loadAvailableUids();
  }, [loadAvailableUids]);

  const validateForm = (): boolean => {
    const errors: Partial<Record<keyof CrawlTaskFormData, string>> = {};

    const trimmedKeyword = formData.keyword.trim();
    if (!trimmedKeyword) {
      errors.keyword = '关键字不能为空';
    }

    if (!formData.eventStartTime) {
      errors.eventStartTime = '请选择事件开始时间';
    } else {
      const selectedTime = new Date(formData.eventStartTime);
      const now = new Date();
      if (selectedTime > now) {
        errors.eventStartTime = '事件开始时间不能是未来时间';
      }
    }

    if (!formData.uid) {
      errors.uid = '请选择微博账号';
    }

    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!validateForm()) {
      return;
    }

    setIsSubmitting(true);

    try {
      const response = await invoke<CreateCrawlTaskResponse>('create_crawl_task', {
        keyword: formData.keyword.trim(),
        eventStartTime: new Date(formData.eventStartTime).toISOString(),
        uid: formData.uid,
      });

      setFormData({
        keyword: '',
        eventStartTime: '',
        uid: '',
      });
      setValidationErrors({});

      if (onTaskCreated) {
        onTaskCreated(response.taskId);
      }
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleFieldChange = (field: keyof CrawlTaskFormData, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    setValidationErrors(prev => ({ ...prev, [field]: undefined }));
  };

  return (
    <div className={`${THEME.CARD_BG} p-6`}>
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-900">创建爬取任务</h2>
        <p className="mt-1 text-sm text-gray-600">配置微博关键字搜索任务</p>
      </div>

      {error && (
        <div className="mb-6 bg-red-50 border border-red-200 rounded-lg p-4">
          <div className="flex items-start gap-3">
            <AlertCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <p className="font-semibold text-red-900 mb-1">创建失败</p>
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

      <form onSubmit={handleSubmit} className="space-y-6">
        <div>
          <label htmlFor="keyword" className="block text-sm font-semibold text-gray-700 mb-2">
            搜索关键字 <span className="text-red-500">*</span>
          </label>
          <input
            id="keyword"
            type="text"
            value={formData.keyword}
            onChange={(e) => handleFieldChange('keyword', e.target.value)}
            disabled={isSubmitting}
            placeholder="请输入搜索关键字，例如：国庆"
            className={`w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 transition-colors ${
              validationErrors.keyword
                ? 'border-red-500 focus:ring-red-500'
                : 'border-gray-300 focus:ring-blue-500'
            } ${isSubmitting ? 'bg-gray-100 cursor-not-allowed' : 'bg-white'}`}
          />
          {validationErrors.keyword && (
            <p className="mt-1 text-sm text-red-600">{validationErrors.keyword}</p>
          )}
        </div>

        <div>
          <label htmlFor="eventStartTime" className="block text-sm font-semibold text-gray-700 mb-2">
            事件开始时间 <span className="text-red-500">*</span>
          </label>
          <input
            id="eventStartTime"
            type="datetime-local"
            value={formData.eventStartTime}
            onChange={(e) => handleFieldChange('eventStartTime', e.target.value)}
            disabled={isSubmitting}
            max={MAX_EVENT_START_TIME}
            className={`w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 transition-colors ${
              validationErrors.eventStartTime
                ? 'border-red-500 focus:ring-red-500'
                : 'border-gray-300 focus:ring-blue-500'
            } ${isSubmitting ? 'bg-gray-100 cursor-not-allowed' : 'bg-white'}`}
          />
          {validationErrors.eventStartTime && (
            <p className="mt-1 text-sm text-red-600">{validationErrors.eventStartTime}</p>
          )}
          <p className="mt-1 text-xs text-gray-500">选择事件开始的时间点，系统将从此时间开始向后回溯爬取</p>
        </div>

        <div>
          <label htmlFor="uid" className="block text-sm font-semibold text-gray-700 mb-2">
            微博账号 <span className="text-red-500">*</span>
          </label>
          {isLoadingUids ? (
            <div className="w-full px-4 py-2 border border-gray-300 rounded-lg bg-gray-50 flex items-center gap-2">
              <Loader2 className="w-4 h-4 animate-spin text-gray-600" />
              <span className="text-sm text-gray-600">加载可用账号...</span>
            </div>
          ) : availableUids.length === 0 ? (
            <div className="w-full px-4 py-2 border border-yellow-300 rounded-lg bg-yellow-50">
              <p className="text-sm text-yellow-800">暂无可用账号，请先完成微博扫码登录</p>
            </div>
          ) : (
            <select
              id="uid"
              value={formData.uid}
              onChange={(e) => handleFieldChange('uid', e.target.value)}
              disabled={isSubmitting}
              className={`w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 transition-colors ${
                validationErrors.uid
                  ? 'border-red-500 focus:ring-red-500'
                  : 'border-gray-300 focus:ring-blue-500'
              } ${isSubmitting ? 'bg-gray-100 cursor-not-allowed' : 'bg-white'}`}
            >
              <option value="">请选择账号</option>
              {availableUids.map((uid) => (
                <option key={uid} value={uid}>
                  {uid}
                </option>
              ))}
            </select>
          )}
          {validationErrors.uid && (
            <p className="mt-1 text-sm text-red-600">{validationErrors.uid}</p>
          )}
        </div>

        <div className="pt-4 border-t">
          <button
            type="submit"
            disabled={isSubmitting || isLoadingUids || availableUids.length === 0}
            className={`w-full ${BUTTON.PRIMARY} ${
              isSubmitting || isLoadingUids || availableUids.length === 0
                ? 'opacity-50 cursor-not-allowed'
                : ''
            } flex items-center justify-center gap-2`}
          >
            {isSubmitting && <Loader2 className="w-5 h-5 animate-spin" />}
            {isSubmitting ? '创建中...' : '创建任务'}
          </button>
        </div>
      </form>
    </div>
  );
};
