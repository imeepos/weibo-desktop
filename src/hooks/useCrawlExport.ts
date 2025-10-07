import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { handleTauriError } from '../utils/errorHandler';
import type { ExportDataResponse } from '../types/crawl';

interface TimeRange {
  start: string;
  end: string;
}

export const useCrawlExport = () => {
  const [isExporting, setIsExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [exportResult, setExportResult] = useState<ExportDataResponse | null>(null);

  const exportData = useCallback(
    async (
      taskId: string,
      format: 'json' | 'csv',
      timeRange?: TimeRange
    ): Promise<ExportDataResponse> => {
      setIsExporting(true);
      setError(null);
      setExportResult(null);

      try {
        const result = await invoke<ExportDataResponse>('export_crawl_data', {
          taskId,
          format,
          timeRange,
        });

        setExportResult(result);
        return result;
      } catch (err) {
        const errorMessage = handleTauriError(err);
        setError(errorMessage);
        throw new Error(errorMessage);
      } finally {
        setIsExporting(false);
      }
    },
    []
  );

  const clearResult = useCallback(() => {
    setExportResult(null);
    setError(null);
  }, []);

  return {
    isExporting,
    error,
    exportResult,
    exportData,
    clearResult,
  };
};
