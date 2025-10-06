import { Component, ReactNode } from 'react';
import { AlertTriangle } from 'lucide-react';
import { logger } from '../utils/logger';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: any) {
    logger.error('React ErrorBoundary捕获错误', {
      error: error.message,
      stack: error.stack,
      componentStack: errorInfo.componentStack,
    });
    console.error('ErrorBoundary caught:', error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen flex items-center justify-center bg-gray-50 p-4">
          <div className="bg-white rounded-lg shadow-lg p-8 max-w-md w-full text-center">
            <AlertTriangle className="w-16 h-16 text-yellow-600 mx-auto mb-4" />
            <h1 className="text-2xl font-bold text-gray-900 mb-2">出错了</h1>
            <p className="text-gray-600 mb-4">
              应用遇到了意外错误
            </p>
            <div className="bg-red-50 border border-red-200 rounded p-3 mb-6 text-left">
              <p className="text-sm text-red-800 font-mono break-all">
                {this.state.error?.message || '未知错误'}
              </p>
            </div>
            <button
              onClick={() => window.location.href = '/'}
              className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              返回首页
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
