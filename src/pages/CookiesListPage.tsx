import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { CookiesData } from '../types/weibo';

/**
 * Cookies管理页面
 *
 * 职责:
 * - 展示已保存的账户
 * - 查看Cookies详情
 * - 删除过期凭证
 *
 * 哲学: 数据的生命周期需要被尊重
 */
export const CookiesListPage = () => {
  const [uids, setUids] = useState<string[]>([]);
  const [selectedCookies, setSelectedCookies] = useState<CookiesData | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    loadUids();
  }, []);

  const loadUids = async () => {
    setIsLoading(true);
    try {
      const uidList = await invoke<string[]>('list_all_uids');
      setUids(uidList);
    } catch (err) {
      console.error('加载UID列表失败:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const viewCookies = async (uid: string) => {
    try {
      const cookies = await invoke<CookiesData>('query_cookies', { uid });
      setSelectedCookies(cookies);
    } catch (err) {
      console.error('查询Cookies失败:', err);
    }
  };

  const deleteCookies = async (uid: string) => {
    if (!confirm(`确定删除UID ${uid} 的Cookies吗?`)) {
      return;
    }

    try {
      await invoke('delete_cookies', { uid });
      await loadUids();
      if (selectedCookies?.uid === uid) {
        setSelectedCookies(null);
      }
    } catch (err) {
      console.error('删除Cookies失败:', err);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50 p-6">
      <div className="max-w-6xl mx-auto">
        <h1 className="text-3xl font-bold text-gray-900 mb-6">Cookies 管理</h1>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-xl font-semibold mb-4">已保存的账户</h2>

            {isLoading ? (
              <p className="text-gray-500">加载中...</p>
            ) : uids.length === 0 ? (
              <p className="text-gray-500">暂无已保存的Cookies</p>
            ) : (
              <ul className="space-y-2">
                {uids.map((uid) => (
                  <li key={uid} className="flex items-center justify-between p-3 bg-gray-50 rounded hover:bg-gray-100">
                    <span className="font-mono text-sm">{uid}</span>
                    <div className="space-x-2">
                      <button
                        onClick={() => viewCookies(uid)}
                        className="text-blue-600 hover:text-blue-800 text-sm"
                      >
                        查看
                      </button>
                      <button
                        onClick={() => deleteCookies(uid)}
                        className="text-red-600 hover:text-red-800 text-sm"
                      >
                        删除
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-xl font-semibold mb-4">Cookies 详情</h2>

            {selectedCookies ? (
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700">UID</label>
                  <p className="mt-1 font-mono text-sm">{selectedCookies.uid}</p>
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
                  <label className="block text-sm font-medium text-gray-700">Cookies</label>
                  <div className="mt-2 space-y-1">
                    {Object.entries(selectedCookies.cookies).map(([key, value]) => (
                      <div key={key} className="bg-gray-50 p-2 rounded font-mono text-xs">
                        <span className="font-semibold">{key}:</span> {value.substring(0, 20)}...
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ) : (
              <p className="text-gray-500">选择一个账户查看详情</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
