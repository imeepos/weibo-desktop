import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { XCircle } from 'lucide-react';
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

interface PlaywrightStatus {
  running: boolean;
  pid?: number;
  port: number;
  healthy: boolean;
}

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
  const isInitialMount = useRef(true);
  const isGeneratingRef = useRef(false);
  const [qrData, setQrData] = useState<GenerateQrcodeResponse | null>(null);
  const [currentEvent, setCurrentEvent] = useState<LoginEvent | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [playwrightStatus, setPlaywrightStatus] = useState<PlaywrightStatus | null>(null);
  const [isStartingServer, setIsStartingServer] = useState(false);

  useEffect(() => {
    qrDataRef.current = qrData;
  }, [qrData]);

  const checkPlaywrightServer = useCallback(async () => {
    try {
      const status = await invoke<PlaywrightStatus>('check_playwright_server');
      setPlaywrightStatus(status);
      return status;
    } catch (err) {
      console.error('检查Playwright服务器状态失败:', err);
      return null;
    }
  }, []);

  const generateQrcode = useCallback(async () => {
    if (isGeneratingRef.current) return; // 防止重复调用

    isGeneratingRef.current = true;
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
      const errorMsg = handleTauriError(err);
      setError(errorMsg);

      if (errorMsg.includes('Playwright服务器未运行')) {
        await checkPlaywrightServer();
      }
    } finally {
      setIsGenerating(false);
      isGeneratingRef.current = false;
    }
  }, [checkPlaywrightServer]);

  const startPlaywrightServer = useCallback(async () => {
    setIsStartingServer(true);
    try {
      await invoke('start_playwright_server');
      await new Promise(resolve => setTimeout(resolve, 2000));
      const status = await checkPlaywrightServer();
      if (status?.healthy) {
        setError(null);
        await generateQrcode();
        return true;
      }
      return false;
    } catch (err) {
      setError(handleTauriError(err));
      return false;
    } finally {
      setIsStartingServer(false);
    }
  }, [checkPlaywrightServer, generateQrcode]);

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

  useEffect(() => {
    if (isInitialMount.current) {
      isInitialMount.current = false;
      void generateQrcode();
    }
  }, []); // 空依赖数组,仅在挂载时执行一次

  useKeyboardShortcut([
    { key: 'r', ctrl: true, callback: () => void generateQrcode() },
    { key: 'c', ctrl: true, callback: () => navigate('/cookies') },
    { key: 'd', ctrl: true, callback: () => navigate('/dependency') },
  ]);

  useEffect(() => {
    let unlistenStatus: UnlistenFn | undefined;
    let unlistenError: UnlistenFn | undefined;
    let unlistenConnectionLost: UnlistenFn | undefined;
    let unlistenConnectionRestored: UnlistenFn | undefined;
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

    const handleConnectionLost = (event: { payload: { qr_id: string; reason: string; timestamp: string } }) => {
      if (!isMounted) return;
      console.warn('WebSocket连接断开:', event.payload);

      if (event.payload.reason === 'reconnecting') {
        setError('连接断开，正在自动重连...');
      } else if (event.payload.reason === 'max_retries_exceeded') {
        setError('连接断开，重连失败。请刷新二维码重试。');
      }
    };

    const handleConnectionRestored = (event: { payload: { qr_id: string; timestamp: string } }) => {
      if (!isMounted) return;
      console.log('WebSocket连接已恢复:', event.payload);
      setError(null);
    };

    const setupListeners = async () => {
      const [statusUnlisten, errorUnlisten, connLostUnlisten, connRestoredUnlisten] = await Promise.all([
        listen<LoginStatusEvent>('login_status_update', handleStatusUpdate),
        listen<LoginErrorEvent>('login_error', handleError),
        listen('websocket_connection_lost', handleConnectionLost),
        listen('websocket_connection_restored', handleConnectionRestored),
      ]);

      if (isMounted) {
        unlistenStatus = statusUnlisten;
        unlistenError = errorUnlisten;
        unlistenConnectionLost = connLostUnlisten;
        unlistenConnectionRestored = connRestoredUnlisten;
      } else {
        statusUnlisten();
        errorUnlisten();
        connLostUnlisten();
        connRestoredUnlisten();
      }
    };

    setupListeners();

    return () => {
      isMounted = false;
      unlistenStatus?.();
      unlistenError?.();
      unlistenConnectionLost?.();
      unlistenConnectionRestored?.();
    };
  }, [navigate]);

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
              <XCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
              <div className="flex-1">
                <p className="font-semibold text-red-900 mb-1">操作失败</p>
                <p className="text-red-700 text-sm whitespace-pre-line">{error}</p>

                {error.includes('Playwright服务器未运行') && playwrightStatus && !playwrightStatus.healthy && (
                  <div className="mt-3 space-y-2">
                    <div className="text-xs text-gray-600">
                      服务状态: {playwrightStatus.running ? '进程运行中' : '未运行'} |
                      健康检查: {playwrightStatus.healthy ? '通过' : '失败'}
                      {playwrightStatus.pid && ` (PID: ${playwrightStatus.pid})`}
                    </div>
                    <button
                      onClick={startPlaywrightServer}
                      disabled={isStartingServer}
                      className={`w-full ${BUTTON.PRIMARY} ${isStartingServer ? 'opacity-50 cursor-not-allowed' : ''}`}
                    >
                      {isStartingServer ? '正在启动服务器...' : '自动启动 Playwright 服务器'}
                    </button>
                  </div>
                )}
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
              disabled={isGenerating}
              className={`w-full ${BUTTON.WARNING} ${isGenerating ? 'opacity-50 cursor-not-allowed' : ''}`}
            >
              {isGenerating ? '正在生成...' : '刷新二维码'}
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
