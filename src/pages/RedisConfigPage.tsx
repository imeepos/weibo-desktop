import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { CheckCircle, XCircle, Eye, EyeOff, Loader2 } from 'lucide-react';
import { handleTauriError } from '../utils/errorHandler';
import { THEME, BUTTON } from '../constants/ui';

interface RedisConfig {
  host: string;
  port: number;
  password?: string;
  database?: number;
}

interface RedisConnectionTestResult {
  success: boolean;
  latency_ms?: number;
  message: string;
  error?: string;
}

interface ValidationErrors {
  host?: string;
  port?: string;
  database?: string;
}

const DEFAULT_CONFIG: RedisConfig = {
  host: 'localhost',
  port: 6379,
  password: '',
  database: 0,
};

export const RedisConfigPage = () => {
  const [config, setConfig] = useState<RedisConfig>(DEFAULT_CONFIG);
  const [showPassword, setShowPassword] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [testResult, setTestResult] = useState<RedisConnectionTestResult | null>(null);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [validationErrors, setValidationErrors] = useState<ValidationErrors>({});

  const loadConfig = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const loadedConfig = await invoke<RedisConfig>('load_redis_config');
      setConfig({
        ...loadedConfig,
        password: loadedConfig.password || '',
        database: loadedConfig.database ?? 0,
      });
    } catch (err) {
      const errorMsg = handleTauriError(err);
      if (!errorMsg.includes('未找到') && !errorMsg.includes('not found')) {
        setError(errorMsg);
      }
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const validateConfig = (): boolean => {
    const errors: ValidationErrors = {};

    if (!config.host.trim()) {
      errors.host = '主机地址不能为空';
    }

    if (config.port < 1 || config.port > 65535) {
      errors.port = '端口必须在 1-65535 之间';
    }

    if (config.database !== undefined && (config.database < 0 || config.database > 15)) {
      errors.database = '数据库索引必须在 0-15 之间';
    }

    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleTestConnection = async () => {
    if (!validateConfig()) return;

    setIsTesting(true);
    setError(null);
    setTestResult(null);
    setSaveSuccess(false);

    try {
      const preparedConfig = {
        ...config,
        password: config.password?.trim() || undefined,
        database: config.database ?? 0,
      };

      const result = await invoke<RedisConnectionTestResult>(
        'test_redis_connection',
        { config: preparedConfig }
      );
      setTestResult(result);
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      setIsTesting(false);
    }
  };

  const handleSaveConfig = async () => {
    if (!validateConfig()) return;

    setIsSaving(true);
    setError(null);
    setSaveSuccess(false);
    setTestResult(null);

    try {
      const preparedConfig = {
        ...config,
        password: config.password?.trim() || undefined,
        database: config.database ?? 0,
      };

      await invoke('save_redis_config', { config: preparedConfig });
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 3000);
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      setIsSaving(false);
    }
  };

  const updateConfig = (field: keyof RedisConfig, value: string | number) => {
    setConfig((prev) => ({ ...prev, [field]: value }));
    setValidationErrors((prev) => ({ ...prev, [field]: undefined }));
    setTestResult(null);
    setSaveSuccess(false);
  };

  if (isLoading) {
    return (
      <div className={`${THEME.GRADIENT_BG} min-h-screen flex items-center justify-center`}>
        <div className="flex items-center gap-3 text-gray-700">
          <Loader2 className="w-6 h-6 animate-spin" />
          <span className="text-lg">加载配置中...</span>
        </div>
      </div>
    );
  }

  return (
    <div className={`${THEME.GRADIENT_BG} min-h-screen flex items-center justify-center p-4`}>
      <div className="max-w-2xl w-full space-y-6">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-gray-900">Redis 配置管理</h1>
          <p className="mt-2 text-gray-600">配置和测试 Redis 数据库连接</p>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <XCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
              <div className="flex-1">
                <p className="font-semibold text-red-900 mb-1">操作失败</p>
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

        {saveSuccess && (
          <div className="bg-green-50 border border-green-200 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
              <div className="flex-1">
                <p className="font-semibold text-green-900 mb-1">保存成功</p>
                <p className="text-green-700 text-sm">Redis 配置已更新</p>
              </div>
              <button
                onClick={() => setSaveSuccess(false)}
                className="text-green-600 hover:text-green-800 text-2xl leading-none"
              >
                ×
              </button>
            </div>
          </div>
        )}

        {testResult && (
          <div
            className={`${
              testResult.success ? 'bg-green-50 border-green-200' : 'bg-yellow-50 border-yellow-200'
            } border rounded-lg p-4`}
          >
            <div className="flex items-start gap-3">
              {testResult.success ? (
                <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
              ) : (
                <XCircle className="w-5 h-5 text-yellow-600 flex-shrink-0 mt-0.5" />
              )}
              <div className="flex-1">
                <p
                  className={`font-semibold mb-1 ${
                    testResult.success ? 'text-green-900' : 'text-yellow-900'
                  }`}
                >
                  {testResult.success ? '连接测试成功' : '连接测试失败'}
                </p>
                <p
                  className={`text-sm ${testResult.success ? 'text-green-700' : 'text-yellow-700'}`}
                >
                  {testResult.message}
                </p>
                {testResult.error && (
                  <p className="text-sm text-yellow-700 mt-2 font-mono bg-yellow-100 p-2 rounded">
                    {testResult.error}
                  </p>
                )}
                {testResult.latency_ms !== undefined && (
                  <p className="text-sm text-green-700 mt-2">
                    延迟: <span className="font-mono font-semibold">{testResult.latency_ms}ms</span>
                  </p>
                )}
              </div>
              <button
                onClick={() => setTestResult(null)}
                className={`${
                  testResult.success ? 'text-green-600 hover:text-green-800' : 'text-yellow-600 hover:text-yellow-800'
                } text-2xl leading-none`}
              >
                ×
              </button>
            </div>
          </div>
        )}

        <div className={`${THEME.CARD_BG} p-6 space-y-6`}>
          <div>
            <h2 className="text-xl font-semibold text-gray-800 pb-4 border-b">连接配置</h2>
          </div>

          <div className="space-y-4">
            <div>
              <label htmlFor="host" className="block text-sm font-medium text-gray-700 mb-2">
                主机地址 <span className="text-red-500">*</span>
              </label>
              <input
                id="host"
                type="text"
                value={config.host}
                onChange={(e) => updateConfig('host', e.target.value)}
                placeholder="localhost 或 127.0.0.1"
                className={`w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent ${
                  validationErrors.host ? 'border-red-500' : 'border-gray-300'
                }`}
              />
              {validationErrors.host && (
                <p className="mt-1 text-sm text-red-600">{validationErrors.host}</p>
              )}
            </div>

            <div>
              <label htmlFor="port" className="block text-sm font-medium text-gray-700 mb-2">
                端口 <span className="text-red-500">*</span>
              </label>
              <input
                id="port"
                type="number"
                min="1"
                max="65535"
                value={config.port}
                onChange={(e) => updateConfig('port', parseInt(e.target.value) || 6379)}
                className={`w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent ${
                  validationErrors.port ? 'border-red-500' : 'border-gray-300'
                }`}
              />
              {validationErrors.port && (
                <p className="mt-1 text-sm text-red-600">{validationErrors.port}</p>
              )}
              <p className="mt-1 text-sm text-gray-500">默认: 6379</p>
            </div>

            <div>
              <label htmlFor="password" className="block text-sm font-medium text-gray-700 mb-2">
                密码 (可选)
              </label>
              <div className="relative">
                <input
                  id="password"
                  type={showPassword ? 'text' : 'password'}
                  value={config.password || ''}
                  onChange={(e) => updateConfig('password', e.target.value)}
                  placeholder="如果 Redis 设置了密码"
                  className="w-full px-4 py-2 pr-12 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-700"
                >
                  {showPassword ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
                </button>
              </div>
            </div>

            <div>
              <label htmlFor="database" className="block text-sm font-medium text-gray-700 mb-2">
                数据库索引 (可选)
              </label>
              <select
                id="database"
                value={config.database ?? 0}
                onChange={(e) => updateConfig('database', parseInt(e.target.value))}
                className={`w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent ${
                  validationErrors.database ? 'border-red-500' : 'border-gray-300'
                }`}
              >
                {Array.from({ length: 16 }, (_, i) => (
                  <option key={i} value={i}>
                    数据库 {i}
                  </option>
                ))}
              </select>
              {validationErrors.database && (
                <p className="mt-1 text-sm text-red-600">{validationErrors.database}</p>
              )}
              <p className="mt-1 text-sm text-gray-500">Redis 支持 0-15 共 16 个数据库</p>
            </div>
          </div>

          <div className="space-y-3 pt-4 border-t">
            <div className="flex gap-3">
              <button
                onClick={handleTestConnection}
                disabled={isTesting || isSaving}
                className={`flex-1 ${BUTTON.SECONDARY} justify-center ${
                  isTesting || isSaving ? 'opacity-50 cursor-not-allowed' : ''
                }`}
              >
                {isTesting ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin mr-2" />
                    测试中...
                  </>
                ) : (
                  '测试连接'
                )}
              </button>
              <button
                onClick={handleSaveConfig}
                disabled={isSaving || isTesting}
                className={`flex-1 ${BUTTON.PRIMARY} justify-center ${
                  isSaving || isTesting ? 'opacity-50 cursor-not-allowed' : ''
                }`}
              >
                {isSaving ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin mr-2" />
                    保存中...
                  </>
                ) : (
                  '保存配置'
                )}
              </button>
            </div>
          </div>
        </div>

        <div className="text-center text-sm text-gray-500">
          <p>配置将保存到本地,用于后续的 Redis 连接</p>
        </div>
      </div>
    </div>
  );
};
