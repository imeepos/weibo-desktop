import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { XCircle, CheckCircle, Loader2, Download, FolderOpen } from 'lucide-react';
import { handleTauriError } from '../utils/errorHandler';
import { BUTTON } from '../constants/ui';
import type { ExportDataResponse } from '../types/crawl';

interface ExportDialogProps {
  taskId: string;
  keyword: string;
  onClose: () => void;
}

type ExportFormat = 'json' | 'csv';

export const ExportDialog = ({ taskId, keyword, onClose }: ExportDialogProps) => {
  const [format, setFormat] = useState<ExportFormat>('json');
  const [startTime, setStartTime] = useState<string>('');
  const [endTime, setEndTime] = useState<string>('');
  const [isExporting, setIsExporting] = useState(false);
  const [exportResult, setExportResult] = useState<ExportDataResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleExport = async () => {
    setIsExporting(true);
    setError(null);
    setExportResult(null);

    try {
      const timeRange =
        startTime && endTime
          ? {
              start: new Date(startTime).toISOString(),
              end: new Date(endTime).toISOString(),
            }
          : undefined;

      const result = await invoke<ExportDataResponse>('export_crawl_data', {
        taskId,
        format,
        timeRange,
      });

      setExportResult(result);
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      setIsExporting(false);
    }
  };

  const handleOpenFolder = async () => {
    if (!exportResult) return;

    try {
      await invoke('open_file_location', { path: exportResult.filePath });
    } catch (err) {
      setError(handleTauriError(err));
    }
  };

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  const isValidTimeRange = !startTime || !endTime || new Date(startTime) <= new Date(endTime);

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center p-4 z-50"
      onClick={handleBackdropClick}
    >
      <div className="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <div className="sticky top-0 bg-white border-b px-6 py-4 flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold text-gray-900">导出爬取数据</h2>
            <p className="text-sm text-gray-600 mt-1">关键字: {keyword}</p>
          </div>
          <button
            onClick={onClose}
            className="text-gray-500 hover:text-gray-700 text-2xl leading-none"
            disabled={isExporting}
          >
            ×
          </button>
        </div>

        <div className="p-6 space-y-6">
          {error && (
            <div className="bg-red-50 border border-red-200 rounded-lg p-4">
              <div className="flex items-start gap-3">
                <XCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
                <div className="flex-1">
                  <p className="font-semibold text-red-900 mb-1">导出失败</p>
                  <p className="text-red-700 text-sm whitespace-pre-line">{error}</p>
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

          {exportResult && (
            <div className="bg-green-50 border border-green-200 rounded-lg p-4">
              <div className="flex items-start gap-3">
                <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
                <div className="flex-1">
                  <p className="font-semibold text-green-900 mb-1">导出成功</p>
                  <p className="text-green-700 text-sm mb-2">
                    已导出 {exportResult.exportedCount} 条数据
                  </p>
                  <div className="bg-green-100 rounded p-2 mb-3">
                    <p className="text-xs text-green-800 font-mono break-all">
                      {exportResult.filePath}
                    </p>
                  </div>
                  <button
                    onClick={handleOpenFolder}
                    className="flex items-center gap-2 text-green-700 hover:text-green-900 font-medium text-sm"
                  >
                    <FolderOpen className="w-4 h-4" />
                    打开文件所在目录
                  </button>
                </div>
              </div>
            </div>
          )}

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              导出格式 <span className="text-red-500">*</span>
            </label>
            <div className="space-y-2">
              <label className="flex items-center gap-3 p-3 border rounded-lg cursor-pointer hover:bg-gray-50 transition-colors">
                <input
                  type="radio"
                  name="format"
                  value="json"
                  checked={format === 'json'}
                  onChange={(e) => setFormat(e.target.value as ExportFormat)}
                  className="w-4 h-4 text-blue-600"
                  disabled={isExporting}
                />
                <div className="flex-1">
                  <p className="font-medium text-gray-900">JSON</p>
                  <p className="text-sm text-gray-600">结构化数据,易于程序处理</p>
                </div>
              </label>
              <label className="flex items-center gap-3 p-3 border rounded-lg cursor-pointer hover:bg-gray-50 transition-colors">
                <input
                  type="radio"
                  name="format"
                  value="csv"
                  checked={format === 'csv'}
                  onChange={(e) => setFormat(e.target.value as ExportFormat)}
                  className="w-4 h-4 text-blue-600"
                  disabled={isExporting}
                />
                <div className="flex-1">
                  <p className="font-medium text-gray-900">CSV</p>
                  <p className="text-sm text-gray-600">表格数据,可用Excel打开</p>
                </div>
              </label>
            </div>
          </div>

          <div className="border-t pt-6">
            <label className="block text-sm font-medium text-gray-700 mb-3">
              时间范围过滤 (可选)
            </label>
            <div className="space-y-4">
              <div>
                <label htmlFor="startTime" className="block text-sm text-gray-600 mb-2">
                  开始时间
                </label>
                <input
                  id="startTime"
                  type="datetime-local"
                  value={startTime}
                  onChange={(e) => setStartTime(e.target.value)}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  disabled={isExporting}
                />
              </div>
              <div>
                <label htmlFor="endTime" className="block text-sm text-gray-600 mb-2">
                  结束时间
                </label>
                <input
                  id="endTime"
                  type="datetime-local"
                  value={endTime}
                  onChange={(e) => setEndTime(e.target.value)}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  disabled={isExporting}
                />
              </div>
              {startTime && endTime && !isValidTimeRange && (
                <p className="text-sm text-red-600">结束时间必须晚于开始时间</p>
              )}
              {(!startTime || !endTime) && (startTime || endTime) && (
                <p className="text-sm text-yellow-600">请同时设置开始时间和结束时间,或留空导出全部数据</p>
              )}
            </div>
          </div>
        </div>

        <div className="sticky bottom-0 bg-gray-50 border-t px-6 py-4 flex gap-3">
          <button
            onClick={onClose}
            disabled={isExporting}
            className={`flex-1 ${BUTTON.SECONDARY} justify-center ${
              isExporting ? 'opacity-50 cursor-not-allowed' : ''
            }`}
          >
            {exportResult ? '关闭' : '取消'}
          </button>
          <button
            onClick={handleExport}
            disabled={isExporting || !isValidTimeRange}
            className={`flex-1 ${BUTTON.PRIMARY} justify-center ${
              isExporting || !isValidTimeRange ? 'opacity-50 cursor-not-allowed' : ''
            }`}
          >
            {isExporting ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin mr-2" />
                导出中...
              </>
            ) : (
              <>
                <Download className="w-4 h-4 mr-2" />
                导出数据
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
