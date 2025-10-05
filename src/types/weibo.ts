/**
 * 微博扫码登录类型定义
 *
 * 与Rust后端完全对应的类型系统
 * 每个类型都承载着明确的语义
 */

/// 二维码状态枚举
export enum QrCodeStatus {
  Pending = 'pending',
  Scanned = 'scanned',
  ConfirmedSuccess = 'confirmed_success',
  Expired = 'expired',
  Rejected = 'rejected',
}

/// 登录会话 - 时间的容器
export interface LoginSession {
  qr_id: string;
  status: QrCodeStatus;
  created_at: string;
  scanned_at: string | null;
  confirmed_at: string | null;
  expires_at: string;
}

/// Cookies数据 - 身份的凭证
export interface CookiesData {
  uid: string;
  cookies: Record<string, string>;
  fetched_at: string;
  validated_at: string;
  redis_key: string;
  screen_name?: string;
}

/// 登录事件类型 - 状态机的边
export enum LoginEventType {
  QrCodeGenerated = 'qr_code_generated',
  QrCodeScanned = 'qr_code_scanned',
  ConfirmedSuccess = 'confirmed_success',
  ValidationSuccess = 'validation_success',
  QrCodeExpired = 'qr_code_expired',
  Error = 'error',
}

/// 登录事件 - 时刻的快照
export interface LoginEvent {
  event_type: LoginEventType;
  timestamp: string;
  session_id: string;
  uid?: string;
  details: any;
}

/// 生成二维码响应
export interface GenerateQrcodeResponse {
  session: LoginSession;
  qr_image: string; // base64编码的PNG图像
}

/// 轮询状态响应
export interface PollStatusResponse {
  event: LoginEvent;
  is_final: boolean;
}

/// 保存Cookies响应
export interface SaveCookiesResponse {
  success: boolean;
  redis_key: string;
  validation_duration_ms: number;
  is_overwrite: boolean;
}
