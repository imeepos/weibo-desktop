import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useKeyboardShortcut } from '../hooks/useKeyboardShortcut';
import { invoke } from '@tauri-apps/api/tauri';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { QrcodeDisplay } from '../components/QrcodeDisplay';
import { LoginStatus } from '../components/LoginStatus';
import { handleTauriError } from '../utils/errorHandler';
import { THEME, BUTTON, TIMING } from '../constants/ui';
import {
  GenerateQrcodeResponse,
  LoginStatusEvent,
  LoginErrorEvent,
  LoginEvent,
  LoginEventType,
  QrCodeStatus,
} from '../types/weibo';

const createEventFromStatus = (event: LoginStatusEvent): LoginEvent | null => {
  const baseEvent = {
    session_id: event.qr_id,
    timestamp: event.updated_at,
  };

  const eventMap: Record<QrCodeStatus, () => LoginEvent | null> = {
    [QrCodeStatus.Pending]: () => null,
    [QrCodeStatus.Scanned]: () => ({
      ...baseEvent,
      event_type: LoginEventType.QrCodeScanned,
      details: {},
    }),
    [QrCodeStatus.Confirmed]: () => event.cookies ? {
      ...baseEvent,
      event_type: LoginEventType.ValidationSuccess,
      uid: event.cookies.uid,
      details: {
        screen_name: event.cookies.screen_name,
        redis_key: event.cookies.redis_key,
      },
    } : {
      ...baseEvent,
      event_type: LoginEventType.Confirmed,
      details: {},
    },
    [QrCodeStatus.Expired]: () => ({
      ...baseEvent,
      event_type: LoginEventType.QrCodeExpired,
      details: {},
    }),
    [QrCodeStatus.Rejected]: () => null,
  };

  return eventMap[event.status]();
};

export const LoginPage = () => {
  const navigate = useNavigate();
  const qrDataRef = useRef<GenerateQrcodeResponse | null>(null);
  const [qrData, setQrData] = useState<GenerateQrcodeResponse | null>(null);
  const [currentEvent, setCurrentEvent] = useState<LoginEvent | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    qrDataRef.current = qrData;
  }, [qrData]);

  useEffect(() => {
    generateQrcode();
  }, [generateQrcode]);

  useKeyboardShortcut([
    { key: 'r', ctrl: true, callback: () => generateQrcode() },
    { key: 'c', ctrl: true, callback: () => navigate('/cookies') },
    { key: 'd', ctrl: true, callback: () => navigate('/dependency') },
  ]);

  useEffect(() => {
    let unlistenStatus: UnlistenFn | undefined;
    let unlistenError: UnlistenFn | undefined;
    let isMounted = true;

    const handleStatusUpdate = (event: { payload: LoginStatusEvent }) => {
      if (!isMounted) return;

      const statusEvent = event.payload;

      if (statusEvent.qr_refreshed && statusEvent.qr_image) {
        setQrData(prev => prev ? {
          ...prev,
          qr_image: statusEvent.qr_image,
          expires_at: new Date(Date.now() + TIMING.QR_EXPIRY_MS).toISOString(),
          expires_in: 180,
        } : null);

        setCurrentEvent({
          event_type: LoginEventType.QrCodeGenerated,
          timestamp: new Date().toISOString(),
          session_id: statusEvent.qr_id,
          details: { auto_refreshed: true },
        });
      }

      const loginEvent = createEventFromStatus(statusEvent);
      if (loginEvent) {
        setCurrentEvent(loginEvent);

        if (loginEvent.event_type === LoginEventType.ValidationSuccess) {
          setTimeout(() => {
            if (isMounted) navigate('/cookies');
          }, TIMING.REDIRECT_DELAY_MS);
        }
      }
    };

    const handleError = (event: { payload: LoginErrorEvent }) => {
      if (!isMounted) return;
      setError(event.payload.message);
    };

    const setupListeners = async () => {
      const [statusUnlisten, errorUnlisten] = await Promise.all([
        listen<LoginStatusEvent>('login_status_update', handleStatusUpdate),
        listen<LoginErrorEvent>('login_error', handleError)
      ]);

      if (isMounted) {
        unlistenStatus = statusUnlisten;
        unlistenError = errorUnlisten;
      } else {
        statusUnlisten();
        errorUnlisten();
      }
    };

    setupListeners();

    return () => {
      isMounted = false;
      unlistenStatus?.();
      unlistenError?.();
    };
  }, [navigate]);

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
    } catch (err) {
      setError(handleTauriError(err));
    } finally {
      setIsGenerating(false);
    }
  }, []);

  const handleExpired = useCallback(() => {
    setCurrentEvent(prev =>
      prev?.event_type !== LoginEventType.QrCodeExpired ? {
        event_type: LoginEventType.QrCodeExpired,
        timestamp: new Date().toISOString(),
        session_id: qrDataRef.current?.qr_id || '',
        details: {},
      } : prev
    );
  }, []);

  const handleReset = useCallback(() => {
    setQrData(null);
    setCurrentEvent(null);
    setError(null);
  }, []);

  const currentStatus = useMemo(() => {
    if (!currentEvent) return undefined;
    const typeToStatus: Record<string, QrCodeStatus> = {
      [LoginEventType.QrCodeGenerated]: QrCodeStatus.Pending,
      [LoginEventType.QrCodeScanned]: QrCodeStatus.Scanned,
      [LoginEventType.Confirmed]: QrCodeStatus.Confirmed,
      [LoginEventType.ValidationSuccess]: QrCodeStatus.Confirmed,
      [LoginEventType.QrCodeExpired]: QrCodeStatus.Expired,
    };
    return typeToStatus[currentEvent.event_type];
  }, [currentEvent]);

  return (
    <div className={`${THEME.GRADIENT_BG} flex items-center justify-center p-4 py-12`}>
      <div className="max-w-md w-full space-y-6">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-gray-900">微博扫码登录</h1>
          <p className="mt-2 text-gray-600">使用微博App扫描二维码登录</p>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <span className="text-xl">❌</span>
              <div className="flex-1">
                <p className="font-semibold text-red-900 mb-1">操作失败</p>
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

        {qrData && (
          <QrcodeDisplay
            qrId={qrData.qr_id}
            qrImage={qrData.qr_image}
            expiresAt={qrData.expires_at}
            status={currentStatus}
            onExpired={handleExpired}
          />
        )}

        <LoginStatus event={currentEvent} isLoading={false} />

        <div className="space-y-3">
          {currentEvent?.event_type === LoginEventType.ValidationSuccess && (
            <div className="flex gap-3">
              <button
                onClick={handleReset}
                className={`flex-1 ${BUTTON.SUCCESS}`}
              >
                再次登录
              </button>
              <button
                onClick={() => navigate('/cookies')}
                className={`flex-1 ${BUTTON.PRIMARY}`}
              >
                查看Cookies
              </button>
            </div>
          )}

          {currentEvent?.event_type === LoginEventType.QrCodeExpired && (
            <button
              onClick={generateQrcode}
              className={`w-full ${BUTTON.WARNING}`}
            >
              刷新二维码
            </button>
          )}
        </div>

        <div className="text-center text-sm text-gray-500">
          <p>Cookies将自动保存到Redis (有效期30天)</p>
          <p className="mt-2 text-xs">
            快捷键: Ctrl+R 刷新 | Ctrl+C Cookies | Ctrl+D 依赖检测
          </p>
        </div>
      </div>
    </div>
  );
};
