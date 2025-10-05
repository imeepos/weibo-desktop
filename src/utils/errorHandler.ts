/**
 * 错误处理工具
 *
 * 哲学: 错误不是终点,而是优雅转化的契机
 * 每个错误都应该被理解、被尊重、被转化为有意义的反馈
 */

/**
 * Tauri InvokeError 结构
 */
interface TauriError {
  message?: string;
  payload?: unknown;
}

/**
 * 错误类型映射表
 * 将技术错误转化为用户可理解的语言
 */
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

/**
 * 格式化 Tauri 错误
 *
 * 处理策略:
 * 1. 字符串错误直接返回
 * 2. 对象错误提取 message 或 payload
 * 3. 未知错误返回通用提示
 */
export function formatTauriError(error: unknown): string {
  // 字符串错误: 直接返回
  if (typeof error === 'string') {
    return error;
  }

  // 对象错误: 提取有意义的信息
  if (error && typeof error === 'object') {
    const err = error as TauriError;

    if (err.message) {
      return err.message;
    }

    if (err.payload && typeof err.payload === 'string') {
      return err.payload;
    }

    // 尝试序列化对象获取更多信息
    try {
      return JSON.stringify(error);
    } catch {
      // 序列化失败,返回通用提示
    }
  }

  return '未知错误,请稍后重试';
}

/**
 * 获取用户友好的错误消息
 *
 * 职责: 将技术术语转化为用户可理解的语言
 *
 * 策略:
 * 1. 检查是否包含已知错误关键词
 * 2. 返回映射的友好消息
 * 3. 否则返回原始错误信息
 */
export function getFriendlyErrorMessage(error: string): string {
  // 遍历错误映射表,查找匹配的关键词
  for (const [key, friendlyMessage] of Object.entries(ERROR_MESSAGES)) {
    if (error.includes(key)) {
      return friendlyMessage;
    }
  }

  // 未找到映射,返回原始错误
  return error;
}

/**
 * 处理 Tauri 错误的完整流程
 *
 * 这是最终暴露给组件的统一接口
 *
 * 处理流程:
 * 1. 格式化错误对象为字符串
 * 2. 转化为用户友好的消息
 * 3. 返回最终的错误提示
 */
export function handleTauriError(error: unknown): string {
  const formatted = formatTauriError(error);
  return getFriendlyErrorMessage(formatted);
}
