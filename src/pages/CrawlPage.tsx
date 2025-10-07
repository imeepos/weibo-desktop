import { useState, useCallback } from 'react';
import { CrawlTaskListSimple } from '../components/CrawlTaskListSimple';
import { CreateTaskSimple } from '../components/CreateTaskSimple';
import { EmptyState } from '../components/EmptyState';
import { THEME } from '../constants/ui';

/**
 * 微博关键字爬取页面 - PostgreSQL简化版本
 *
 * 架构变更说明:
 * - 移除复杂的Redis时间分片架构
 * - 使用PostgreSQL简化版本
 * - 5种任务状态（Created, Crawling, Completed, Paused, Failed）
 * - 直接增量爬取逻辑
 */
export const CrawlPage = () => {
  const [refreshKey, setRefreshKey] = useState(0);

  const handleTaskCreated = useCallback((taskId: string) => {
    console.log('[CrawlPage] 任务创建成功:', taskId);
    // 刷新任务列表
    setRefreshKey(prev => prev + 1);
  }, []);

  return (
    <div className={`${THEME.GRADIENT_BG} min-h-screen p-6`}>
      <div className="max-w-7xl mx-auto">
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900">微博关键字爬取</h1>
          <p className="mt-2 text-gray-600">创建和管理微博关键字搜索爬取任务（PostgreSQL简化版本）</p>
        </div>

        <div className="space-y-6">
          {/* 创建任务表单 */}
          <CreateTaskSimple onTaskCreated={handleTaskCreated} />

          {/* 任务列表 */}
          <div className={THEME.CARD_BG}>
            <div className="p-6 border-b">
              <h2 className="text-xl font-semibold text-gray-800">任务列表</h2>
            </div>
            <div className="p-6">
              <CrawlTaskListSimple refreshTrigger={refreshKey} />
            </div>
          </div>

          {/* 说明区域 */}
          <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
            <h3 className="text-sm font-semibold text-blue-900 mb-2">简化架构说明</h3>
            <ul className="text-sm text-blue-800 space-y-1">
              <li>✅ 使用PostgreSQL存储，移除复杂的Redis时间分片</li>
              <li>✅ 5种简化状态：已创建、爬取中、已完成、已暂停、失败</li>
              <li>✅ 直接增量爬取，自动去重</li>
              <li>✅ 任务列表支持筛选、搜索、操作（启动/暂停/删除）</li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
};
