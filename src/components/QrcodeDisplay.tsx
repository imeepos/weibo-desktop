import { useEffect, useState } from 'react';
import { QrCodeStatus } from '../types/weibo';

interface QrcodeDisplayProps {
  qrId: string;
  qrImage: string;
  expiresAt: string;
  onExpired: () => void;
}

export const QrcodeDisplay = ({
  qrId,
  qrImage,
  expiresAt,
  onExpired,
}: QrcodeDisplayProps) => {
  const [remainingSeconds, setRemainingSeconds] = useState(0);
  const [status, setStatus] = useState<QrCodeStatus>(QrCodeStatus.Pending);

  useEffect(() => {
    const updateRemaining = () => {
      const now = new Date().getTime();
      const expiresAtTime = new Date(expiresAt).getTime();
      const remaining = Math.max(0, Math.floor((expiresAtTime - now) / 1000));
      setRemainingSeconds(remaining);

      if (remaining === 0) {
        setStatus(QrCodeStatus.Expired);
        onExpired();
      }
    };

    updateRemaining();
    const interval = setInterval(updateRemaining, 1000);

    return () => clearInterval(interval);
  }, [expiresAt, onExpired]);

  const getStatusText = (): string => {
    switch (status) {
      case QrCodeStatus.Pending:
        return '请使用微博App扫描二维码';
      case QrCodeStatus.Scanned:
        return '已扫描,请在手机上确认登录';
      case QrCodeStatus.Confirmed:
        return '登录成功!';
      case QrCodeStatus.Expired:
        return '二维码已过期';
      default:
        return '';
    }
  };

  const getStatusColor = (): string => {
    switch (status) {
      case QrCodeStatus.Pending:
        return 'text-blue-600';
      case QrCodeStatus.Scanned:
        return 'text-yellow-600';
      case QrCodeStatus.Confirmed:
        return 'text-green-600';
      case QrCodeStatus.Expired:
        return 'text-red-600';
      default:
        return 'text-gray-600';
    }
  };

  const isExpired = status === QrCodeStatus.Expired;

  return (
    <div className="flex flex-col items-center gap-4 p-6 bg-white rounded-lg shadow-lg">
      <div className="relative">
        <img
          src={`data:image/png;base64,${qrImage}`}
          alt="微博登录二维码"
          className={`w-64 h-64 ${isExpired ? 'opacity-50 grayscale' : ''}`}
        />
        {isExpired && (
          <div className="absolute inset-0 flex items-center justify-center bg-black bg-opacity-50 text-white text-xl font-bold rounded">
            已过期
          </div>
        )}
      </div>

      <p className={`text-lg font-semibold ${getStatusColor()}`}>
        {getStatusText()}
      </p>

      {status === QrCodeStatus.Pending && (
        <div className="flex items-center gap-2 text-gray-600">
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span>剩余 {remainingSeconds} 秒</span>
        </div>
      )}

      <p className="text-xs text-gray-400 font-mono">
        会话ID: {qrId.substring(0, 12)}...
      </p>
    </div>
  );
};
