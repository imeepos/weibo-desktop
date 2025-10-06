export enum QrCodeStatus {
  Pending = 'pending',
  Scanned = 'scanned',
  Confirmed = 'confirmed',
  Rejected = 'rejected',
  Expired = 'expired',
}

export interface CookiesData {
  uid: string;
  cookies: Record<string, string>;
  fetched_at: string;
  validated_at: string;
  redis_key: string;
  screen_name?: string;
}

export enum LoginEventType {
  QrCodeGenerated = 'qr_code_generated',
  QrCodeScanned = 'qr_code_scanned',
  Confirmed = 'confirmed',
  ValidationSuccess = 'validation_success',
  QrCodeExpired = 'qr_code_expired',
  Error = 'error',
}

export type LoginEventDetails =
  | { auto_refreshed?: boolean }
  | { screen_name?: string; redis_key?: string }
  | { error?: string }
  | Record<string, never>;

export interface LoginEvent {
  event_type: LoginEventType;
  timestamp: string;
  session_id: string;
  uid?: string;
  details: LoginEventDetails;
}

export interface GenerateQrcodeResponse {
  qr_id: string;
  qr_image: string;
  expires_at: string;
  expires_in: number;
}

export interface LoginStatusEvent {
  qr_id: string;
  status: QrCodeStatus;
  cookies?: CookiesData;
  updated_at: string;
  qr_refreshed?: boolean;
  qr_image?: string;
}

export interface LoginErrorEvent {
  qr_id: string;
  error_type: string;
  message: string;
  timestamp: string;
}

export interface SaveCookiesResponse {
  success: boolean;
  redis_key: string;
  validation_duration_ms: number;
  is_overwrite: boolean;
}
