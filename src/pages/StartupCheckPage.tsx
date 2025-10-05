import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { handleTauriError } from '../utils/errorHandler';
import { DependencyCheckResult, InstallationProgressEvent } from '../types/dependency';

/**
 * 启动时依赖检测页面
 *
 * 职责:
 * - 应用启动时自动检测依赖状态
 * - 根据检测结果决定后续流程
 * - 显示检测进度和安装指引
 *
 * 哲学: 应用的启动过程需要优雅且透明
 */
export const StartupCheckPage = () => {
  // 检测状态
  const [isChecking, setIsChecking] = useState(false);
  const [checkResults, setCheckResults] = useState<DependencyCheckResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  // 进度信息
  const [currentProgress, setCurrentProgress] = useState({
    currentIndex: 0,
    totalCount: 0,
    currentDep: '',
    status: ''
  });

  // 安装状态
  const [installing, setInstalling] = useState(false);
  const [installationProgress, setInstallationProgress] = useState<InstallationProgressEvent>({
    task_id: '',
    dependency_id: '',
    status: '',
    progress_percent: 0
  });

  useEffect(() => {
    // 启动依赖检测
    performDependencyCheck();

    // 监听依赖检测进度事件
    const unlistenProgress = listen('dependency-check-progress', (event) => {
      const progress = event.payload as any;
      setCurrentProgress({
        currentIndex: progress.current_index || 0,
        totalCount: progress.total_count || 0,
        currentDep: progress.current_dependency || '',
        status: progress.status || ''
      });
    });

    // 监听安装进度事件
    const unlistenInstallation = listen('installation-progress', (event) => {
      const progress = event.payload as InstallationProgressEvent;
      setInstallationProgress(progress);
    });

    return () => {
      unlistenProgress.then(f => f?.());
      unlistenInstallation.then(f => f?.());
    };
  }, []);

  const performDependencyCheck = async () => {
    setIsChecking(true);
    setError(null);

    try {
      const results = await invoke<DependencyCheckResult[]>('check_dependencies');
      setCheckResults(results);
      handleCheckResults(results);
    } catch (err) {
      const errorMsg = handleTauriError(err);
      setError(errorMsg);
      console.error('依赖检测失败:', err);
    } finally {
      setIsChecking(false);
    }
  };

  const handleCheckResults = (results: DependencyCheckResult[]) => {
    // 分离必需和可选依赖
    const missingRequired = results.filter(result =>
      result.status === 'missing' || result.status === 'version_mismatch'
    );
    const missingOptional = results.filter(result =>
      (result.status === 'missing' || result.status === 'version_mismatch') &&
      result.dependency_id.startsWith('optional-')
    );

    if (missingRequired.length === 0) {
      // 所有必需依赖都满足，尝试自动安装可选依赖
      if (missingOptional.length > 0) {
        autoInstallOptionalDependencies(missingOptional);
      } else {
        // 所有依赖都满足，可以显示成功状态或调用回调函数
        console.log('所有依赖都满足，应用可以正常启动');
      }
    }
    // 必需依赖缺失时，显示安装指引（在渲染逻辑中处理）
  };

  const autoInstallOptionalDependencies = async (dependencies: DependencyCheckResult[]) => {
    setInstalling(true);

    try {
      // 并行安装所有可选依赖
      const installPromises = dependencies.map(dep =>
        invoke('install_dependency', {
          dependencyId: dep.dependency_id,
          force: false
        })
      );

      await Promise.allSettled(installPromises);

      // 安装完成后重新检测
      await performDependencyCheck();
    } catch (err) {
      const errorMsg = handleTauriError(err);
      setError(errorMsg);
      console.error('自动安装依赖失败:', err);
    } finally {
      setInstalling(false);
    }
  };

  const handleManualInstall = async (dependencyId: string) => {
    try {
      const results = await invoke<DependencyCheckResult[]>('trigger_manual_check', { dependencyId });
      setCheckResults(results);
      handleCheckResults(results);
    } catch (err) {
      const errorMsg = handleTauriError(err);
      setError(errorMsg);
      console.error('手动检测失败:', err);
    }
  };

  const getMissingRequiredDeps = () => {
    return checkResults.filter(result =>
      (result.status === 'missing' || result.status === 'version_mismatch') &&
      !result.dependency_id.startsWith('optional-')
    );
  };

  const getMissingOptionalDeps = () => {
    return checkResults.filter(result =>
      (result.status === 'missing' || result.status === 'version_mismatch') &&
      result.dependency_id.startsWith('optional-')
    );
  };

  const renderProgress = () => {
    if (!isChecking && !installing) return null;

    const progressPercent = installing
      ? installationProgress.progress_percent
      : currentProgress.totalCount > 0
        ? Math.round((currentProgress.currentIndex / currentProgress.totalCount) * 100)
        : 0;

    return (
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
        <div className="bg-white rounded-lg shadow-xl p-8 max-w-md w-full mx-4">
          <div className="text-center">
            <div className="mb-4">
              <div className="inline-flex items-center justify-center w-16 h-16 bg-blue-100 rounded-full">
                {installing ? (
                  <div className="w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
                ) : (
                  <div className="w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
                )}
              </div>
            </div>

            <h2 className="text-xl font-semibold text-gray-900 mb-2">
              {installing ? '正在安装依赖' : '正在检测依赖'}
            </h2>

            <p className="text-gray-600 mb-4">
              {installing
                ? `${installationProgress.status} (${progressPercent}%)`
                : currentProgress.currentDep
                  ? `正在检测: ${currentProgress.currentDep}`
                  : '准备检测依赖环境...'
              }
            </p>

            <div className="w-full bg-gray-200 rounded-full h-2">
              <div
                className="bg-blue-600 h-2 rounded-full transition-all duration-300 ease-out"
                style={{ width: `${progressPercent}%` }}
              ></div>
            </div>

            {!installing && currentProgress.totalCount > 0 && (
              <p className="text-sm text-gray-500 mt-2">
                {currentProgress.currentIndex} / {currentProgress.totalCount}
              </p>
            )}
          </div>
        </div>
      </div>
    );
  };

  const renderInstallationGuide = (dependency: DependencyCheckResult) => {
    const installGuides: Record<string, string> = {
      'redis': `## 安装 Redis Server

Redis 是内存数据库，用于存储用户会话和缓存数据。

### 方式1: Docker (推荐)
\`\`\`bash
docker run -d -p 6379:6379 redis:7-alpine
\`\`\`

### 方式2: 手动安装
1. 访问 [Redis官网](https://redis.io/download)
2. 下载适合您操作系统的版本
3. 按照官方文档完成安装
4. 启动Redis服务: \`redis-server\`

### 验证安装
\`\`\`bash
redis-cli ping
# 应该返回: PONG
\`\`\``,

      'playwright': `## 安装 Playwright

Playwright 用于浏览器自动化测试。

### 安装命令
\`\`\`bash
pnpm install playwright
npx playwright install
\`\`\`

### 验证安装
\`\`\`bash
npx playwright --version
\`\`\``,

      'node': `## 安装 Node.js

Node.js 是 JavaScript 运行时环境。

### 方式1: 官网下载
1. 访问 [Node.js官网](https://nodejs.org/)
2. 下载 LTS 版本 (推荐 20.x 或更高)
3. 运行安装程序并按提示完成安装

### 方式2: 包管理器
**macOS (使用 Homebrew):**
\`\`\`bash
brew install node
\`\`\`

**Linux (使用 apt):**
\`\`\`bash
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt-get install -y nodejs
\`\`\`

### 验证安装
\`\`\`bash
node --version
npm --version
\`\`\``
    };

    const guide = installGuides[dependency.dependency_id] || `## 安装 ${dependency.dependency_id}

请访问相关官网获取安装指引。`;

    return (
      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">
          ⚠️ 缺少必需依赖: {dependency.dependency_id}
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

          <button
            onClick={() => window.open('https://github.com/your-repo/issues', '_blank')}
            className="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 transition-colors"
          >
            获取帮助
          </button>
        </div>
      </div>
    );
  };

  const renderContent = () => {
    if (error) {
      return (
        <div className="min-h-screen bg-gray-50 flex items-center justify-center p-6">
          <div className="bg-white rounded-lg shadow p-8 max-w-md w-full">
            <div className="text-center">
              <div className="w-16 h-16 bg-red-100 rounded-full flex items-center justify-center mx-auto mb-4">
                <span className="text-2xl">❌</span>
              </div>

              <h2 className="text-xl font-semibold text-gray-900 mb-2">检测失败</h2>

              <p className="text-gray-600 mb-6">{error}</p>

              <button
                onClick={performDependencyCheck}
                className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
              >
                重新检测
              </button>
            </div>
          </div>
        </div>
      );
    }

    const missingRequired = getMissingRequiredDeps();
    const missingOptional = getMissingOptionalDeps();

    if (missingRequired.length > 0) {
      return (
        <div className="min-h-screen bg-gray-50 p-6">
          <div className="max-w-4xl mx-auto">
            <div className="text-center mb-8">
              <h1 className="text-3xl font-bold text-gray-900 mb-4">依赖检测</h1>
              <p className="text-gray-600">
                检测到缺少必需的依赖项，请按照指引完成安装后继续
              </p>
            </div>

            {missingRequired.map((dep) => (
              <div key={dep.dependency_id}>
                {renderInstallationGuide(dep)}
              </div>
            ))}

            {missingOptional.length > 0 && (
              <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
                <p className="text-blue-700">
                  ℹ️ 检测到 {missingOptional.length} 个可选依赖缺失，将在必需依赖安装完成后自动安装
                </p>
              </div>
            )}
          </div>
        </div>
      );
    }

    // 所有依赖满足，显示加载状态
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-2xl">✅</span>
          </div>

          <h2 className="text-xl font-semibold text-gray-900 mb-2">环境准备就绪</h2>
          <p className="text-gray-600">正在进入应用...</p>
        </div>
      </div>
    );
  };

  return (
    <>
      {renderContent()}
      {renderProgress()}
    </>
  );
};