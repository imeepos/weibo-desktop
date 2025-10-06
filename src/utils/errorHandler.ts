interface TauriError {
  message?: string;
  payload?: unknown;
}

const ERROR_MESSAGES: Record<string, string> = {
  // 网络相关
  NetworkFailed: '网络连接失败,请检查网络设置',
  ConnectionTimeout: '连接超时,请稍后重试',
  RequestFailed: '请求失败,请稍后重试',

  // 二维码相关
  QrCodeExpired: '二维码已过期,请重新生成',
  QrCodeGenerationFailed: '二维码生成失败,请重试',

  // 限流相关
  RateLimited: '请求过于频繁,请稍后再试',
  TooManyRequests: '请求过多,请稍后再试',

  // Redis相关
  RedisConnectionFailed: 'Redis连接失败,请检查服务状态',
  RedisOperationFailed: '数据存储失败,请重试',

  // Cookie相关
  CookieNotFound: '未找到Cookie数据',
  CookieExpired: 'Cookie已过期,请重新登录',
  CookieValidationFailed: 'Cookie验证失败',

  // 登录相关
  LoginFailed: '登录失败,请重试',
  LoginCancelled: '登录已取消',
  LoginTimeout: '登录超时,请重试',

  // 验证相关
  ValidationFailed: '验证失败,请重新登录',
  InvalidCredentials: '凭证无效,请重新登录',
};

export function formatTauriError(error: unknown): string {
  if (typeof error === 'string') return error;

  if (error && typeof error === 'object') {
    const { message, payload } = error as TauriError;
    return message || (typeof payload === 'string' ? payload : '未知错误,请稍后重试');
  }

  return '未知错误,请稍后重试';
}

export function getFriendlyErrorMessage(error: string): string {
  for (const [key, friendlyMessage] of Object.entries(ERROR_MESSAGES)) {
    if (error.includes(key)) return friendlyMessage;
  }
  return error;
}

export function handleTauriError(error: unknown): string {
  return getFriendlyErrorMessage(formatTauriError(error));
}
