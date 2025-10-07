import { useState, useCallback } from 'react';
import { Download } from 'lucide-react';
import { CrawlTaskList } from '../components/CrawlTaskList';
import { CrawlTaskForm } from '../components/CrawlTaskForm';
import { CrawlProgress } from '../components/CrawlProgress';
import { ExportDialog } from '../components/ExportDialog';
import { EmptyState } from '../components/EmptyState';
import { useCrawlTask } from '../hooks/useCrawlTask';
import { useCrawlProgress } from '../hooks/useCrawlProgress';
import { useCrawlExport } from '../hooks/useCrawlExport';
import { THEME, BUTTON } from '../constants/ui';
import { invoke } from '@tauri-apps/api/core';
import { handleTauriError } from '../utils/errorHandler';
import type { CrawlTask } from '../types/crawl';

export const CrawlPage = () => {
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [selectedTask, setSelectedTask] = useState<CrawlTask | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [isLoadingTask, setIsLoadingTask] = useState(false);

  const { startTask } = useCrawlTask();
  const { clearProgress } = useCrawlProgress(selectedTaskId || undefined);
  const { clearResult } = useCrawlExport();

  const handleTaskSelect = useCallback(async (taskId: string) => {
    setSelectedTaskId(taskId);
    setIsLoadingTask(true);

    try {
      const task = await invoke<CrawlTask>('get_crawl_task', { taskId });
      setSelectedTask(task);
    } catch (err) {
      console.error('Failed to load task:', handleTauriError(err));
      setSelectedTask(null);
    } finally {
      setIsLoadingTask(false);
    }
  }, []);

  const handleTaskCreated = useCallback(async (taskId: string) => {
    setRefreshKey(prev => prev + 1);
    await handleTaskSelect(taskId);

    try {
      await startTask(taskId);
    } catch (err) {
      console.error('Failed to start task:', err);
    }
  }, [handleTaskSelect, startTask]);

  const handleProgressComplete = useCallback(() => {
    setRefreshKey(prev => prev + 1);
  }, []);

  const handleProgressError = useCallback((error: string) => {
    console.error('Crawl error:', error);
    setRefreshKey(prev => prev + 1);
  }, []);

  const handleExportClick = useCallback(() => {
    if (!selectedTask) return;
    setShowExportDialog(true);
  }, [selectedTask]);

  const handleExportClose = useCallback(() => {
    setShowExportDialog(false);
    clearResult();
  }, [clearResult]);

  const handleClearSelection = useCallback(() => {
    setSelectedTaskId(null);
    setSelectedTask(null);
    clearProgress();
  }, [clearProgress]);

  return (
    <div className={`${THEME.GRADIENT_BG} min-h-screen p-6`}>
      <div className="max-w-7xl mx-auto">
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900">微博关键字爬取</h1>
          <p className="mt-2 text-gray-600">创建和管理微博关键字搜索爬取任务</p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="space-y-6">
            <div className={THEME.CARD_BG}>
              <div className="p-6 border-b">
                <h2 className="text-xl font-semibold text-gray-800">任务列表</h2>
              </div>
              <div className="p-6">
                <CrawlTaskList
                  onTaskSelect={handleTaskSelect}
                  refreshTrigger={refreshKey}
                />
              </div>
            </div>

            <CrawlTaskForm onTaskCreated={handleTaskCreated} />
          </div>

          <div className="space-y-6">
            {selectedTaskId && selectedTask ? (
              <>
                <div className="flex items-center justify-between mb-4">
                  <button
                    onClick={handleClearSelection}
                    className={BUTTON.SECONDARY}
                  >
                    ← 返回列表
                  </button>
                  <button
                    onClick={handleExportClick}
                    className={`${BUTTON.NAVIGATION_PRIMARY}`}
                    disabled={selectedTask.crawledCount === 0}
                  >
                    <Download className="w-4 h-4" />
                    导出数据
                  </button>
                </div>

                {isLoadingTask ? (
                  <div className={`${THEME.CARD_BG} p-6`}>
                    <div className="animate-pulse space-y-4">
                      <div className="h-4 bg-gray-200 rounded w-3/4"></div>
                      <div className="h-4 bg-gray-200 rounded w-1/2"></div>
                      <div className="h-4 bg-gray-200 rounded w-2/3"></div>
                    </div>
                  </div>
                ) : (
                  <CrawlProgress
                    taskId={selectedTaskId}
                    onComplete={handleProgressComplete}
                    onError={handleProgressError}
                  />
                )}
              </>
            ) : (
              <div className={`${THEME.CARD_BG} p-12`}>
                <EmptyState
                  title="未选择任务"
                  description="从左侧任务列表中选择一个任务查看详情"
                />
              </div>
            )}
          </div>
        </div>
      </div>

      {showExportDialog && selectedTask && (
        <ExportDialog
          taskId={selectedTask.id}
          keyword={selectedTask.keyword}
          onClose={handleExportClose}
        />
      )}
    </div>
  );
};
