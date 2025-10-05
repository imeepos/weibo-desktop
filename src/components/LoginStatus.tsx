import { LoginEvent, LoginEventType } from '../types/weibo';

interface LoginStatusProps {
  event: LoginEvent | null;
  isLoading: boolean;
}

/**
 * 登录状态组件
 *
 * 职责: 将系统事件翻译成人类语言
 * 哲学: 每个事件都值得被优雅地呈现
 */
export const LoginStatus = ({ event, isLoading }: LoginStatusProps) => {
  if (isLoading) {
    return (
      <div className="flex items-center gap-3 p-4 bg-blue-50 rounded-lg">
        <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600"></div>
        <span className="text-blue-700">处理中...</span>
      </div>
    );
  }

  if (!event) {
    return null;
  }

  const getEventIcon = (): string => {
    switch (event.event_type) {
      case LoginEventType.QrCodeGenerated:
        return '✓';
      case LoginEventType.QrCodeScanned:
        return '👀';
      case LoginEventType.Confirmed:
        return '✓';
      case LoginEventType.ValidationSuccess:
        return '🎉';
      case LoginEventType.QrCodeExpired:
        return '⏰';
      case LoginEventType.Error:
        return '❌';
      default:
        return 'ℹ️';
    }
  };

  const getEventColor = (): string => {
    switch (event.event_type) {
      case LoginEventType.ValidationSuccess:
        return 'bg-green-50 text-green-700 border-green-200';
      case LoginEventType.QrCodeScanned:
      case LoginEventType.Confirmed:
        return 'bg-yellow-50 text-yellow-700 border-yellow-200';
      case LoginEventType.Error:
      case LoginEventType.QrCodeExpired:
        return 'bg-red-50 text-red-700 border-red-200';
      default:
        return 'bg-blue-50 text-blue-700 border-blue-200';
    }
  };

  const getEventMessage = (): string => {
    switch (event.event_type) {
      case LoginEventType.QrCodeGenerated:
        return '二维码生成成功';
      case LoginEventType.QrCodeScanned:
        return '已扫描,等待确认';
      case LoginEventType.Confirmed:
        return '确认登录成功';
      case LoginEventType.ValidationSuccess:
        return `登录成功! 欢迎 ${event.details?.screen_name || event.uid}`;
      case LoginEventType.QrCodeExpired:
        return '二维码已过期,请重新生成';
      case LoginEventType.Error:
        return `错误: ${event.details?.error || '未知错误'}`;
      default:
        return '未知事件';
    }
  };

  return (
    <div className={`flex items-start gap-3 p-4 rounded-lg border ${getEventColor()}`}>
      <span className="text-2xl">{getEventIcon()}</span>
      <div className="flex-1">
        <p className="font-medium">{getEventMessage()}</p>
        {event.uid && (
          <p className="text-sm mt-1 opacity-75">UID: {event.uid}</p>
        )}
        <p className="text-xs mt-1 opacity-50">
          {new Date(event.timestamp).toLocaleString('zh-CN')}
        </p>
      </div>
    </div>
  );
};
