import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { CheckCircle, XCircle, AlertTriangle, Settings } from 'lucide-react';
import { handleTauriError } from '../utils/errorHandler';
import {
  DependencyCheckResult,
  InstallationProgressEvent,
  DependencyCheckProgressEvent
} from '../types/dependency';
import { INSTALL_GUIDES } from '../constants/installGuides';

export const DependencyCheckPage = () => {

  const [isChecking, setIsChecking] = useState(false);
  const [checkResults, setCheckResults] = useState<DependencyCheckResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [checkProgress, setCheckProgress] = useState<DependencyCheckProgressEvent>({
    current_index: 0,
    total_count: 0,
    current_dependency: '',
    status: ''
  });

  const [installing] = useState(false);
  const [installationProgress, setInstallationProgress] = useState<InstallationProgressEvent>({
    task_id: '',
    dependency_id: '',
    status: '',
    progress_percent: 0
  });

  useEffect(() => {
    let unlistenProgress: UnlistenFn | undefined;
    let unlistenInstallation: UnlistenFn | undefined;
    let isMounted = true;

    const setupListeners = async () => {
      const [progressUnlisten, installUnlisten] = await Promise.all([
        listen<DependencyCheckProgressEvent>('dependency-check-progress', (event) => {
          if (isMounted) setCheckProgress(event.payload);
        }),
        listen<InstallationProgressEvent>('installation-progress', (event) => {
          if (isMounted) setInstallationProgress(event.payload);
        })
      ]);

      if (isMounted) {
        unlistenProgress = progressUnlisten;
        unlistenInstallation = installUnlisten;
      } else {
        progressUnlisten();
        installUnlisten();
      }
    };

    setupListeners();

    return () => {
      isMounted = false;
      unlistenProgress?.();
      unlistenInstallation?.();
    };
  }, []);

  const performDependencyCheck = async () => {
    setIsChecking(true);
    setError(null);

    try {
      const results = await invoke<DependencyCheckResult[]>('check_dependencies');
      setCheckResults(results);
    } catch (err) {
      const errorMsg = handleTauriError(err);
      setError(errorMsg);
    } finally {
      setIsChecking(false);
    }
  };

  const handleManualInstall = async (dependencyId: string) => {
    try {
      const results = await invoke<DependencyCheckResult[]>('trigger_manual_check', { dependencyId });
      setCheckResults(results);
    } catch (err) {
      const errorMsg = handleTauriError(err);
      setError(errorMsg);
    }
  };

  const getMissingDeps = () => {
    return checkResults.filter(result =>
      result.status === 'missing' || result.status === 'version_mismatch'
    );
  };

  const getSatisfiedDeps = () => {
    return checkResults.filter(result => result.status === 'satisfied');
  };

  const renderProgress = () => {
    if (!isChecking && !installing) return null;

    const progressPercent = installing
      ? installationProgress.progress_percent
      : checkProgress.total_count > 0
        ? Math.round((checkProgress.current_index / checkProgress.total_count) * 100)
        : 0;

    return (
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
        <div className="bg-white rounded-lg shadow-xl p-8 max-w-md w-full mx-4">
          <div className="text-center">
            <div className="mb-4">
              <div className="inline-flex items-center justify-center w-16 h-16 bg-blue-100 rounded-full">
                <div className="w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
              </div>
            </div>

            <h2 className="text-xl font-semibold text-gray-900 mb-2">
              {installing ? '正在安装依赖' : '正在检测依赖'}
            </h2>

            <p className="text-gray-600 mb-4">
              {installing
                ? `${installationProgress.status} (${progressPercent}%)`
                : checkProgress.current_dependency
                  ? `正在检测: ${checkProgress.current_dependency}`
                  : '准备检测依赖环境...'
              }
            </p>

            <div className="w-full bg-gray-200 rounded-full h-2">
              <div
                className="bg-blue-600 h-2 rounded-full transition-all duration-300 ease-out"
                style={{ width: `${progressPercent}%` }}
              ></div>
            </div>

            {!installing && checkProgress.total_count > 0 && (
              <p className="text-sm text-gray-500 mt-2">
                {checkProgress.current_index} / {checkProgress.total_count}
              </p>
            )}
          </div>
        </div>
      </div>
    );
  };

  const renderInstallationGuide = (dependency: DependencyCheckResult) => {
    const guide = INSTALL_GUIDES[dependency.dependency_id] || `## 安装 ${dependency.dependency_id}

请访问相关官网获取安装指引。`;

    return (
      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4 flex items-center gap-2">
          <AlertTriangle className="w-5 h-5 text-yellow-600" />
          缺少依赖: {dependency.dependency_id}
        </h3>

        <div className="prose prose-sm max-w-none">
          <div className="whitespace-pre-wrap text-sm text-gray-700 bg-gray-50 p-4 rounded-lg">
            {guide}
          </div>
        </div>

        <div className="mt-4 flex space-x-3">
          <button
            onClick={() => handleManualInstall(dependency.dependency_id)}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          >
            重新检测
          </button>
        </div>
      </div>
    );
  };

  return (
    <>
      <div className="py-8 px-4">
        <div className="max-w-6xl mx-auto">
          <div className="mb-6">
            <h1 className="text-3xl font-bold text-gray-900">运行环境检测</h1>
            <p className="text-gray-600 mt-2">检测应用运行所需的依赖项</p>
          </div>

          {error && (
            <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
              <div className="flex items-start gap-3">
                <XCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
                <div className="flex-1">
                  <p className="font-semibold text-red-900 mb-1">检测失败</p>
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

          {checkResults.length === 0 && !isChecking && (
            <div className="bg-white rounded-lg shadow p-8 mb-6 text-center">
              <div className="w-16 h-16 bg-blue-100 rounded-full flex items-center justify-center mx-auto mb-4">
                <Settings className="w-8 h-8 text-blue-600" />
              </div>
              <h2 className="text-xl font-semibold text-gray-900 mb-2">准备检测依赖环境</h2>
              <p className="text-gray-600 mb-6">
                系统将检测 Node.js、pnpm、Redis、Playwright 等依赖项
              </p>
              <button
                onClick={performDependencyCheck}
                className="px-8 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium"
              >
                开始检测
              </button>
            </div>
          )}

          {checkResults.length > 0 && (
            <div className="bg-white rounded-lg shadow p-4 mb-6 flex items-center justify-between">
              <div className="flex items-center gap-3">
                {getMissingDeps().length === 0 ? (
                  <CheckCircle className="w-8 h-8 text-green-600" />
                ) : (
                  <AlertTriangle className="w-8 h-8 text-yellow-600" />
                )}
                <div>
                  <p className="font-semibold text-gray-900">
                    {getMissingDeps().length === 0 ? '所有依赖已满足' : `${getMissingDeps().length} 个依赖缺失`}
                  </p>
                  <p className="text-sm text-gray-600">
                    {getSatisfiedDeps().length} / {checkResults.length} 项已满足
                  </p>
                </div>
              </div>
              <button
                onClick={performDependencyCheck}
                disabled={isChecking}
                className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:bg-gray-400"
              >
                {isChecking ? '检测中...' : '重新检测'}
              </button>
            </div>
          )}

          {checkResults.length > 0 && (
            <>
              {getSatisfiedDeps().length > 0 && (
                <div className="bg-white rounded-lg shadow p-6 mb-6">
                  <h2 className="text-xl font-semibold text-green-700 mb-4 flex items-center gap-2">
                    <CheckCircle className="w-6 h-6" />
                    已满足的依赖 ({getSatisfiedDeps().length})
                  </h2>
                  <ul className="space-y-2">
                    {getSatisfiedDeps().map((dep) => (
                      <li key={dep.dependency_id} className="flex items-center justify-between p-3 bg-green-50 rounded">
                        <div>
                          <span className="font-semibold">{dep.dependency_id}</span>
                          {dep.detected_version && (
                            <span className="text-sm text-gray-600 ml-2">v{dep.detected_version}</span>
                          )}
                        </div>
                        <span className="text-green-600">✓</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              {getMissingDeps().length > 0 && (
                <div>
                  <h2 className="text-xl font-semibold text-red-700 mb-4 flex items-center gap-2">
                    <XCircle className="w-6 h-6" />
                    缺失的依赖 ({getMissingDeps().length})
                  </h2>
                  {getMissingDeps().map((dep) => (
                    <div key={dep.dependency_id}>
                      {renderInstallationGuide(dep)}
                    </div>
                  ))}
                </div>
              )}
            </>
          )}
        </div>
      </div>
      {renderProgress()}
    </>
  );
};
