import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CrawlTaskSummary } from '../types/crawl';
import { handleTauriError } from '../utils/errorHandler';

interface CreateTaskResponse {
  taskId: string;
  createdAt: string;
}

interface LoadingStates {
  create: boolean;
  start: boolean;
  pause: boolean;
  list: boolean;
}

interface ListTasksParams {
  status?: string;
  sortBy?: 'CreatedAt' | 'UpdatedAt';
  sortOrder?: 'Asc' | 'Desc';
}

export const useCrawlTask = () => {
  const [tasks, setTasks] = useState<CrawlTaskSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState<LoadingStates>({
    create: false,
    start: false,
    pause: false,
    list: false,
  });

  const isLoading = Object.values(loading).some(state => state);

  const updateLoading = (operation: keyof LoadingStates, state: boolean) => {
    setLoading(prev => ({ ...prev, [operation]: state }));
  };

  const createTask = useCallback(async (
    keyword: string,
    eventStartTime: string,
    uid: string
  ): Promise<string> => {
    updateLoading('create', true);
    setError(null);

    try {
      const response = await invoke<CreateTaskResponse>('create_crawl_task', {
        keyword,
        eventStartTime,
        uid,
      });
      return response.taskId;
    } catch (err) {
      const errorMessage = handleTauriError(err);
      setError(errorMessage);
      throw new Error(errorMessage);
    } finally {
      updateLoading('create', false);
    }
  }, []);

  const startTask = useCallback(async (taskId: string): Promise<void> => {
    updateLoading('start', true);
    setError(null);

    try {
      await invoke<void>('start_crawl', { taskId });
    } catch (err) {
      const errorMessage = handleTauriError(err);
      setError(errorMessage);
      throw new Error(errorMessage);
    } finally {
      updateLoading('start', false);
    }
  }, []);

  const pauseTask = useCallback(async (taskId: string): Promise<void> => {
    updateLoading('pause', true);
    setError(null);

    try {
      await invoke<void>('pause_crawl', { taskId });
    } catch (err) {
      const errorMessage = handleTauriError(err);
      setError(errorMessage);
      throw new Error(errorMessage);
    } finally {
      updateLoading('pause', false);
    }
  }, []);

  const listTasks = useCallback(async (
    params: ListTasksParams = {}
  ): Promise<void> => {
    updateLoading('list', true);
    setError(null);

    try {
      const response = await invoke<{tasks: CrawlTaskSummary[], total: number}>('list_crawl_tasks', {
        request: {
          status: params.status,
          sort_by: params.sortBy,
          sort_order: params.sortOrder,
        },
      });
      setTasks(response.tasks);
    } catch (err) {
      const errorMessage = handleTauriError(err);
      setError(errorMessage);
      throw new Error(errorMessage);
    } finally {
      updateLoading('list', false);
    }
  }, []);

  const refreshTasks = useCallback(async (): Promise<void> => {
    await listTasks();
  }, [listTasks]);

  return {
    tasks,
    isLoading,
    error,
    createTask,
    startTask,
    pauseTask,
    listTasks,
    refreshTasks,
  };
};
