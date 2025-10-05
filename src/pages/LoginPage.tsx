import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { QrcodeDisplay } from '../components/QrcodeDisplay';
import { LoginStatus } from '../components/LoginStatus';
import { handleTauriError } from '../utils/errorHandler';
import {
  GenerateQrcodeResponse,
  PollStatusResponse,
  LoginEvent,
  LoginEventType,
  QrCodeStatus,
} from '../types/weibo';

/**
 * 状态映射: QrCodeStatus -> LoginEvent
 *
 * 将后端的状态枚举转换为前端的事件对象
 * 每个状态转换都是一个有意义的时刻
 */
const mapStatusToEvent = (
  sessionId: string,
  response: PollStatusResponse
): LoginEvent | null => {
  const { status, cookies, updated_at } = response;

  switch (status) {
    case QrCodeStatus.Scanned:
      return {
        event_type: LoginEventType.QrCodeScanned,
        timestamp: updated_at,
        session_id: sessionId,
        details: {},
      };

    case QrCodeStatus.Confirmed:
      if (cookies) {
        return {
          event_type: LoginEventType.ValidationSuccess,
          timestamp: updated_at,
          session_id: sessionId,
          uid: cookies.uid,
          details: {
            screen_name: cookies.screen_name,
            redis_key: cookies.redis_key,
          },
        };
      }
      return {
        event_type: LoginEventType.Confirmed,
        timestamp: updated_at,
        session_id: sessionId,
        details: {},
      };

    case QrCodeStatus.Expired:
      return {
        event_type: LoginEventType.QrCodeExpired,
        timestamp: updated_at,
        session_id: sessionId,
        details: {},
      };

    case QrCodeStatus.Pending:
    default:
      return null;
  }
};

/**
 * 微博扫码登录页面
 *
 * 职责: 协调用户意图与系统状态
 * 哲学: 异步的世界需要优雅的编排
 */
export const LoginPage = () => {
  const [qrData, setQrData] = useState<GenerateQrcodeResponse | null>(null);
  const [currentEvent, setCurrentEvent] = useState<LoginEvent | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [isPolling, setIsPolling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const generateQrcode = useCallback(async () => {
    setIsGenerating(true);
    setError(null);
    setCurrentEvent(null);

    try {
      const response = await invoke<GenerateQrcodeResponse>('generate_qrcode');
      setQrData(response);

      setCurrentEvent({
        event_type: LoginEventType.QrCodeGenerated,
        timestamp: new Date().toISOString(),
        session_id: response.qr_id,
        details: {},
      });

      startPolling(response.qr_id);
    } catch (err) {
      setError(handleTauriError(err));
      console.error('生成二维码失败:', err);
    } finally {
      setIsGenerating(false);
    }
  }, []);

  const startPolling = useCallback(async (qrId: string) => {
    setIsPolling(true);

    try {
      while (true) {
        const response = await invoke<PollStatusResponse>('poll_login_status', {
          qrId,
        });

        // 状态机映射: status -> LoginEvent
        const event = mapStatusToEvent(qrId, response);
        if (event) {
          setCurrentEvent(event);
        }

        // 终止条件: confirmed 且有 cookies,或 expired
        if (
          (response.status === 'confirmed' && response.cookies) ||
          response.status === 'expired'
        ) {
          break;
        }

        await new Promise((resolve) => setTimeout(resolve, 3000));
      }
    } catch (err) {
      setError(handleTauriError(err));
      console.error('轮询失败:', err);
    } finally {
      setIsPolling(false);
    }
  }, []);

  const handleExpired = useCallback(() => {
    if (currentEvent?.event_type !== LoginEventType.QrCodeExpired) {
      setCurrentEvent({
        event_type: LoginEventType.QrCodeExpired,
        timestamp: new Date().toISOString(),
        session_id: qrData?.qr_id || '',
        details: {},
      });
    }
  }, [qrData, currentEvent]);

  const handleReset = useCallback(() => {
    setQrData(null);
    setCurrentEvent(null);
    setError(null);
  }, []);

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 flex items-center justify-center p-4">
      <div className="max-w-md w-full space-y-6">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-gray-900">微博扫码登录</h1>
          <p className="mt-2 text-gray-600">获取微博Cookies</p>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4">
            <p className="text-red-700">{error}</p>
          </div>
        )}

        {qrData && (
          <QrcodeDisplay
            qrId={qrData.qr_id}
            qrImage={qrData.qr_image}
            expiresAt={qrData.expires_at}
            onExpired={handleExpired}
          />
        )}

        <LoginStatus event={currentEvent} isLoading={isPolling} />

        <div className="space-y-3">
          {!qrData && (
            <button
              onClick={generateQrcode}
              disabled={isGenerating}
              className="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white font-semibold py-3 px-4 rounded-lg transition-colors"
            >
              {isGenerating ? '生成中...' : '生成二维码'}
            </button>
          )}

          {qrData && currentEvent?.event_type === LoginEventType.ValidationSuccess && (
            <button
              onClick={handleReset}
              className="w-full bg-green-600 hover:bg-green-700 text-white font-semibold py-3 px-4 rounded-lg transition-colors"
            >
              再次登录
            </button>
          )}

          {qrData && currentEvent?.event_type === LoginEventType.QrCodeExpired && (
            <button
              onClick={generateQrcode}
              className="w-full bg-orange-600 hover:bg-orange-700 text-white font-semibold py-3 px-4 rounded-lg transition-colors"
            >
              刷新二维码
            </button>
          )}
        </div>

        <div className="text-center text-sm text-gray-500">
          <p>使用微博App扫描二维码并确认登录</p>
          <p className="mt-1">Cookies将自动保存到Redis</p>
        </div>
      </div>
    </div>
  );
};
