/**
 * 错误处理相关类型定义
 *
 * 与后端CommandError结构体完全对应
 */

/**
 * Tauri命令执行错误
 * 与后端src/commands/crawl_commands.rs中的CommandError结构体对应
 */
export interface CommandError {
  /** 错误消息 */
  error: string;
  /** 错误代码 */
  code: string;
}

/**
 * 爬取任务相关错误代码枚举
 */
export enum CrawlErrorCode {
  /** 关键字无效 */
  INVALID_KEYWORD = 'INVALID_KEYWORD',
  /** 时间格式或范围无效 */
  INVALID_TIME = 'INVALID_TIME',
  /** 未找到Cookies */
  COOKIES_NOT_FOUND = 'COOKIES_NOT_FOUND',
  /** Cookies已过期 */
  COOKIES_EXPIRED = 'COOKIES_EXPIRED',
  /** 任务不存在 */
  TASK_NOT_FOUND = 'TASK_NOT_FOUND',
  /** 任务状态无效 */
  INVALID_STATUS = 'INVALID_STATUS',
  /** 已有任务在运行 */
  ALREADY_RUNNING = 'ALREADY_RUNNING',
  /** 无数据可导出 */
  NO_DATA = 'NO_DATA',
  /** 导出格式无效 */
  INVALID_FORMAT = 'INVALID_FORMAT',
  /** 存储操作错误 */
  STORAGE_ERROR = 'STORAGE_ERROR',
  /** 文件系统操作错误 */
  FILE_SYSTEM_ERROR = 'FILE_SYSTEM_ERROR',
}

/**
 * 登录相关错误代码枚举
 */
export enum LoginErrorCode {
  /** 网络连接失败 */
  NetworkFailed = 'NetworkFailed',
  /** 连接超时 */
  ConnectionTimeout = 'ConnectionTimeout',
  /** 请求失败 */
  RequestFailed = 'RequestFailed',
  /** Playwright服务器未运行 */
  PlaywrightServerNotRunning = 'PlaywrightServerNotRunning',
  /** 二维码已过期 */
  QrCodeExpired = 'QrCodeExpired',
  /** 二维码生成失败 */
  QrCodeGenerationFailed = 'QrCodeGenerationFailed',
  /** 请求过于频繁 */
  RateLimited = 'RateLimited',
  /** 请求过多 */
  TooManyRequests = 'TooManyRequests',
  /** Redis连接失败 */
  RedisConnectionFailed = 'RedisConnectionFailed',
  /** Redis操作失败 */
  RedisOperationFailed = 'RedisOperationFailed',
  /** Cookie未找到 */
  CookieNotFound = 'CookieNotFound',
  /** Cookie已过期 */
  CookieExpired = 'CookieExpired',
  /** Cookie验证失败 */
  CookieValidationFailed = 'CookieValidationFailed',
  /** 登录失败 */
  LoginFailed = 'LoginFailed',
  /** 登录取消 */
  LoginCancelled = 'LoginCancelled',
  /** 登录超时 */
  LoginTimeout = 'LoginTimeout',
  /** 验证失败 */
  ValidationFailed = 'ValidationFailed',
  /** 凭证无效 */
  InvalidCredentials = 'InvalidCredentials',
}

/**
 * 所有错误代码的联合类型
 */
export type ErrorCode = CrawlErrorCode | LoginErrorCode;

/**
 * 标准化的API错误响应
 */
export interface ApiErrorResponse {
  error: CommandError;
}

/**
 * 错误消息映射类型
 */
export type ErrorMessageMap = Record<ErrorCode, string>;