import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/tauri';
import { handleTauriError } from '../utils/errorHandler';
import { CookiesData } from '../types/weibo';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { Toast } from '../components/Toast';
import { useToast } from '../hooks/useToast';
import { ListSkeleton } from '../components/LoadingSkeleton';
import { EmptyState } from '../components/EmptyState';
import { BUTTON } from '../constants/ui';

export const CookiesListPage = () => {
  const navigate = useNavigate();
  const { toast, showToast, hideToast } = useToast();
  const [uids, setUids] = useState<string[]>([]);
  const [selectedCookies, setSelectedCookies] = useState<CookiesData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{ uid: string } | null>(null);

  useEffect(() => {
    loadUids();
  }, []);

  const loadUids = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const uidList = await invoke<string[]>('list_all_uids');
      setUids(uidList);
    } catch (err) {
      const errorMsg = handleTauriError(err);
      setError(errorMsg);
    } finally {
      setIsLoading(false);
    }
  };

  const viewCookies = async (uid: string) => {
    setError(null);
    try {
      const cookies = await invoke<CookiesData>('query_cookies', { uid });
      setSelectedCookies(cookies);
    } catch (err) {
      const errorMsg = handleTauriError(err);
      setError(errorMsg);
    }
  };

  const performDelete = async (uid: string) => {
    setError(null);
    setDeleteConfirm(null);

    try {
      await invoke('delete_cookies', { uid });
      await loadUids();
      if (selectedCookies?.uid === uid) {
        setSelectedCookies(null);
      }
    } catch (err) {
      const errorMsg = handleTauriError(err);
      setError(errorMsg);
    }
  };

  const copyCookies = (text: string) => {
    navigator.clipboard.writeText(text).then(() => {
      showToast('已复制到剪贴板');
    }).catch(() => {
      showToast('复制失败', 'error');
    });
  };

  const exportCookies = (cookies: CookiesData) => {
    try {
      const data = JSON.stringify(cookies, null, 2);
      const blob = new Blob([data], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `cookies_${cookies.uid}_${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
      showToast('导出成功');
    } catch (err) {
      showToast('导出失败', 'error');
    }
  };

  return (
    <div className="py-8 px-4">
      <div className="max-w-6xl mx-auto">
        <div className="mb-6 flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900">Cookies 管理</h1>
            <p className="text-gray-600 mt-2">查看和管理已保存的微博账户 Cookies</p>
          </div>
          <button
            onClick={loadUids}
            disabled={isLoading}
            className="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors disabled:opacity-50 flex items-center gap-2"
          >
            <span className={isLoading ? 'animate-spin' : ''}>🔄</span>
            <span className="hidden sm:inline">刷新</span>
          </button>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
            <div className="flex items-start gap-3">
              <span className="text-xl">❌</span>
              <div className="flex-1">
                <p className="font-semibold text-red-900 mb-1">操作失败</p>
                <p className="text-red-700 text-sm">{error}</p>
              </div>
              <button
                onClick={() => setError(null)}
                className="text-red-600 hover:text-red-800"
              >
                ×
              </button>
            </div>
          </div>
        )}

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 sm:gap-6">
          <div className="bg-white rounded-lg shadow p-4 sm:p-6">
            <h2 className="text-lg sm:text-xl font-semibold mb-4">已保存的账户</h2>

            {isLoading ? (
              <ListSkeleton count={3} />
            ) : uids.length === 0 ? (
              <EmptyState
                icon="🍪"
                title="暂无Cookies"
                description="扫码登录后，Cookies将自动保存到这里"
                action={{
                  label: '立即登录',
                  onClick: () => navigate('/login')
                }}
              />
            ) : (
              <ul className="space-y-2">
                {uids.map((uid) => (
                  <li key={uid} className="flex items-center justify-between p-3 bg-gray-50 rounded hover:bg-gray-100 transition-colors">
                    <span className="font-mono text-sm">{uid}</span>
                    <div className="space-x-2">
                      <button
                        onClick={() => viewCookies(uid)}
                        className="text-blue-600 hover:text-blue-800 text-sm font-medium"
                      >
                        查看
                      </button>
                      <button
                        onClick={() => setDeleteConfirm({ uid })}
                        className="text-red-600 hover:text-red-800 text-sm font-medium"
                      >
                        删除
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div className="bg-white rounded-lg shadow p-4 sm:p-6">
            <h2 className="text-lg sm:text-xl font-semibold mb-4">Cookies 详情</h2>

            {selectedCookies ? (
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <h3 className="text-lg font-semibold text-gray-900">账户信息</h3>
                  <div className="flex gap-2">
                    <button
                      onClick={() => copyCookies(JSON.stringify(selectedCookies.cookies))}
                      className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
                    >
                      📋 复制
                    </button>
                    <button
                      onClick={() => exportCookies(selectedCookies)}
                      className="px-3 py-1 text-sm bg-green-600 text-white rounded hover:bg-green-700"
                    >
                      💾 导出
                    </button>
                  </div>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700">UID</label>
                  <div className="mt-1 flex items-center gap-2">
                    <p className="font-mono text-sm">{selectedCookies.uid}</p>
                    <button
                      onClick={() => copyCookies(selectedCookies.uid)}
                      className="text-blue-600 hover:text-blue-800 text-xs"
                    >
                      复制
                    </button>
                  </div>
                </div>

                {selectedCookies.screen_name && (
                  <div>
                    <label className="block text-sm font-medium text-gray-700">昵称</label>
                    <p className="mt-1">{selectedCookies.screen_name}</p>
                  </div>
                )}

                <div>
                  <label className="block text-sm font-medium text-gray-700">获取时间</label>
                  <p className="mt-1 text-sm">{new Date(selectedCookies.fetched_at).toLocaleString('zh-CN')}</p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">Cookies</label>
                  <div className="space-y-1 max-h-64 overflow-y-auto">
                    {Object.entries(selectedCookies.cookies).map(([key, value]) => (
                      <div key={key} className="bg-gray-50 p-2 rounded font-mono text-xs flex items-start justify-between gap-2">
                        <div className="flex-1 break-all">
                          <span className="font-semibold text-blue-700">{key}:</span>{' '}
                          <span className="text-gray-700">{value}</span>
                        </div>
                        <button
                          onClick={() => copyCookies(value)}
                          className="text-blue-600 hover:text-blue-800 flex-shrink-0 text-xs"
                        >
                          复制
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ) : (
              <div className="text-center py-12">
                <span className="text-4xl mb-4 block">📋</span>
                <p className="text-gray-500">选择一个账户查看详情</p>
              </div>
            )}
          </div>
        </div>
      </div>

      {deleteConfirm && (
        <ConfirmDialog
          title="确认删除"
          message={`确定删除UID ${deleteConfirm.uid} 的Cookies吗?`}
          onConfirm={() => performDelete(deleteConfirm.uid)}
          onCancel={() => setDeleteConfirm(null)}
        />
      )}

      {toast && (
        <Toast
          message={toast.message}
          type={toast.type}
          onClose={hideToast}
        />
      )}
    </div>
  );
};
