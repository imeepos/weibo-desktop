import { invoke } from '@tauri-apps/api/tauri';

export enum LogLevel {
  Error = 'error',
  Warn = 'warn',
  Info = 'info',
  Debug = 'debug',
}

interface FrontendLog {
  level: LogLevel;
  message: string;
  context: Record<string, any>;
  timestamp: string;
  user_agent: string | null;
  url: string | null;
}

class Logger {
  private logQueue: FrontendLog[] = [];
  private flushInterval = 5000;
  private maxQueueSize = 50;

  constructor() {
    this.startFlushTimer();

    window.addEventListener('beforeunload', () => this.flush());
  }

  private startFlushTimer() {
    window.setInterval(() => this.flush(), this.flushInterval);
  }

  private createLog(level: LogLevel, message: string, context: Record<string, any> = {}): FrontendLog {
    return {
      level,
      message,
      context,
      timestamp: new Date().toISOString(),
      user_agent: navigator.userAgent,
      url: window.location.href,
    };
  }

  private enqueue(log: FrontendLog) {
    this.logQueue.push(log);

    if (this.logQueue.length >= this.maxQueueSize) {
      this.flush();
    }
  }

  private async flush() {
    if (this.logQueue.length === 0) return;

    const logs = [...this.logQueue];
    this.logQueue = [];

    try {
      await invoke('log_frontend_batch', { logs });
    } catch (err) {
      console.error('[Logger] 批量发送日志失败:', err);
    }
  }

  error(message: string, context?: Record<string, any>) {
    const log = this.createLog(LogLevel.Error, message, context);
    this.enqueue(log);
    console.error(message, context);
  }

  warn(message: string, context?: Record<string, any>) {
    const log = this.createLog(LogLevel.Warn, message, context);
    this.enqueue(log);
    console.warn(message, context);
  }

  info(message: string, context?: Record<string, any>) {
    const log = this.createLog(LogLevel.Info, message, context);
    this.enqueue(log);
    console.info(message, context);
  }

  debug(message: string, context?: Record<string, any>) {
    const log = this.createLog(LogLevel.Debug, message, context);
    this.enqueue(log);
    console.debug(message, context);
  }
}

export const logger = new Logger();

window.addEventListener('error', (event) => {
  logger.error('全局错误', {
    message: event.message,
    filename: event.filename,
    lineno: event.lineno,
    colno: event.colno,
    error: event.error?.stack,
  });
});

window.addEventListener('unhandledrejection', (event) => {
  logger.error('未处理的Promise拒绝', {
    reason: event.reason,
    promise: String(event.promise),
  });
});
