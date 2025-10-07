import { CommandError, ErrorCode, CrawlErrorCode, LoginErrorCode } from '../types/error';
import { isCommandError, parseCommandError } from './typeGuards';

interface TauriError {
  message?: string;
  payload?: unknown;
}

const ERROR_MESSAGES: Record<ErrorCode, string> = {
  // 爬取任务相关错误
  [CrawlErrorCode.INVALID_KEYWORD]: '关键字不能为空',
  [CrawlErrorCode.INVALID_TIME]: '时间格式错误或时间范围无效',
  [CrawlErrorCode.COOKIES_NOT_FOUND]: '未找到Cookies,请先扫码登录',
  [CrawlErrorCode.COOKIES_EXPIRED]: 'Cookies可能已过期,请重新登录',
  [CrawlErrorCode.TASK_NOT_FOUND]: '任务不存在',
  [CrawlErrorCode.INVALID_STATUS]: '任务状态无法执行此操作',
  [CrawlErrorCode.ALREADY_RUNNING]: '已有任务正在运行,请先暂停或等待完成',
  [CrawlErrorCode.NO_DATA]: '任务尚无数据可导出',
  [CrawlErrorCode.INVALID_FORMAT]: '不支持的导出格式',
  [CrawlErrorCode.STORAGE_ERROR]: '数据存储操作失败',
  [CrawlErrorCode.FILE_SYSTEM_ERROR]: '文件写入失败',

  // 网络相关
  [LoginErrorCode.NetworkFailed]: '网络连接失败,请检查网络设置',
  [LoginErrorCode.ConnectionTimeout]: '连接超时,请稍后重试',
  [LoginErrorCode.RequestFailed]: '请求失败,请稍后重试',

  // Playwright服务器相关
  [LoginErrorCode.PlaywrightServerNotRunning]: 'Playwright服务器未运行\n\n请在终端执行以下命令启动:\n./scripts/start-playwright-server.sh\n\n或者检查9223端口是否被占用',

  // 二维码相关
  [LoginErrorCode.QrCodeExpired]: '二维码已过期,请重新生成',
  [LoginErrorCode.QrCodeGenerationFailed]: '二维码生成失败,请重试',

  // 限流相关
  [LoginErrorCode.RateLimited]: '请求过于频繁,请稍后再试',
  [LoginErrorCode.TooManyRequests]: '请求过多,请稍后再试',

  // Redis相关
  [LoginErrorCode.RedisConnectionFailed]: 'Redis连接失败,请检查服务状态',
  [LoginErrorCode.RedisOperationFailed]: '数据存储失败,请重试',

  // Cookie相关
  [LoginErrorCode.CookieNotFound]: '未找到Cookie数据',
  [LoginErrorCode.CookieExpired]: 'Cookie已过期,请重新登录',
  [LoginErrorCode.CookieValidationFailed]: 'Cookie验证失败',

  // 登录相关
  [LoginErrorCode.LoginFailed]: '登录失败,请重试',
  [LoginErrorCode.LoginCancelled]: '登录已取消',
  [LoginErrorCode.LoginTimeout]: '登录超时,请重试',

  // 验证相关
  [LoginErrorCode.ValidationFailed]: '验证失败,请重新登录',
  [LoginErrorCode.InvalidCredentials]: '凭证无效,请重新登录',
};


/**
 * 检查错误是否为TauriError结构
 */
function isTauriError(error: unknown): error is TauriError {
  return (
    error &&
    typeof error === 'object' &&
    ('message' in error || 'payload' in error)
  );
}

/**
 * 格式化Tauri错误为标准错误消息
 */
export function formatTauriError(error: unknown): string {
  // 1. 尝试解析CommandError（后端标准错误）
  const commandError = parseCommandError(error);
  if (commandError) {
    return JSON.stringify(commandError);
  }

  // 2. 处理传统TauriError结构
  if (isTauriError(error)) {
    const { message, payload } = error as TauriError;

    // 如果payload是CommandError结构，提取其内容
    if (payload && isCommandError(payload)) {
      return JSON.stringify(payload);
    }

    return message || (typeof payload === 'string' ? payload : '未知错误,请稍后重试');
  }

  // 3. 处理字符串错误
  if (typeof error === 'string') {
    return error;
  }

  // 4. 其他未知错误
  return '未知错误,请稍后重试';
}

/**
 * 根据错误代码获取友好错误消息
 */
export function getFriendlyErrorMessage(error: string): string {
  // 1. 尝试解析CommandError JSON
  const commandError = parseCommandError(error);
  if (commandError) {
    return ERROR_MESSAGES[commandError.code as ErrorCode] || commandError.error;
  }

  // 2. 使用传统的错误代码匹配
  for (const [key, friendlyMessage] of Object.entries(ERROR_MESSAGES)) {
    if (error.includes(key)) return friendlyMessage;
  }

  // 3. 返回原始错误消息
  return error;
}

/**
 * 统一的Tauri错误处理入口
 * 将后端错误转换为用户友好的消息
 */
export function handleTauriError(error: unknown): string {
  const formattedError = formatTauriError(error);
  return getFriendlyErrorMessage(formattedError);
}

/**
 * 提取错误代码
 * 用于需要根据错误代码进行特殊处理的场景
 */
export function extractErrorCode(error: unknown): string | null {
  // 1. 尝试解析CommandError
  const commandError = parseCommandError(error);
  if (commandError) {
    return commandError.code;
  }

  // 2. 处理TauriError的payload
  if (isTauriError(error) && error.payload && isCommandError(error.payload)) {
    return error.payload.code;
  }

  // 3. 处理JSON字符串
  if (typeof error === 'string') {
    // 4. 字符串中是否包含错误代码
    for (const code of Object.values(CrawlErrorCode)) {
      if (error.includes(code)) return code;
    }
    for (const code of Object.values(LoginErrorCode)) {
      if (error.includes(code)) return code;
    }
  }

  return null;
}

/**
 * 检查是否为特定错误代码
 */
export function hasErrorCode(error: unknown, targetCode: ErrorCode): boolean {
  const errorCode = extractErrorCode(error);
  return errorCode === targetCode;
}
