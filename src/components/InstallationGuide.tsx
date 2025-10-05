import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

/**
 * 安装链接接口
 */
export interface InstallationLink {
  /** 链接文本 */
  text: string;
  /** 链接URL */
  url: string;
}

/**
 * 安装指引接口
 */
export interface InstallationGuide {
  /** 关联的依赖项ID */
  dependency_id: string;
  /** 依赖名称(用于展示) */
  dependency_name: string;
  /** 指引标题 */
  title: string;
  /** 指引内容(Markdown格式步骤列表) */
  content: string;
  /** 相关链接(官方下载页/文档) */
  links: InstallationLink[];
  /** 适用的操作系统(空表示全平台通用) */
  target_os: string[];
  /** 语言版本(当前固定为"zh-CN") */
  language: string;
}

/**
 * 依赖重要性级别
 */
export type DependencyLevel = 'required' | 'optional';

interface InstallationGuideProps {
  /** 安装指引数据 */
  guide: InstallationGuide;
  /** 重新检测回调 */
  onRecheck: () => void;
  /** 依赖重要性级别 */
  level: DependencyLevel;
}

/**
 * 安装指引组件
 *
 * 用于显示依赖的手动安装指引，支持Markdown渲染和重新检测功能
 */
export const InstallationGuide: React.FC<InstallationGuideProps> = ({
  guide,
  onRecheck,
  level
}) => {
  const [isRechecking, setIsRechecking] = useState(false);

  /**
   * 处理重新检测按钮点击
   */
  const handleRecheck = async () => {
    if (isRechecking) return;

    setIsRechecking(true);
    try {
      // 调用后端触发手动检测
      await invoke('trigger_manual_check');
      // 调用父组件提供的回调
      onRecheck();
    } catch (error) {
      console.error('重新检测失败:', error);
    } finally {
      setIsRechecking(false);
    }
  };

  /**
   * 渲染Markdown内容 (简化版本，支持基础语法)
   */
  const renderMarkdown = (content: string) => {
    // 简单的Markdown渲染，支持标题、列表、链接、代码块
    const lines = content.split('\n');
    return lines.map((line, index) => {
      // 标题处理
      if (line.startsWith('## ')) {
        return (
          <h2 key={index} className="text-xl font-semibold text-gray-800 mt-6 mb-3">
            {line.replace('## ', '')}
          </h2>
        );
      }
      if (line.startsWith('### ')) {
        return (
          <h3 key={index} className="text-lg font-medium text-gray-700 mt-4 mb-2">
            {line.replace('### ', '')}
          </h3>
        );
      }

      // 有序列表处理
      if (/^\d+\.\s/.test(line)) {
        return (
          <li key={index} className="ml-6 mb-2 text-gray-600 leading-relaxed">
            <span className="font-medium text-gray-700">
              {line.match(/^\d+\.\s/)?.[0]}
            </span>
            {line.replace(/^\d+\.\s/, '')}
          </li>
        );
      }

      // 无序列表处理
      if (line.startsWith('- ')) {
        return (
          <li key={index} className="ml-6 mb-2 text-gray-600 leading-relaxed list-disc">
            {line.replace('- ', '')}
          </li>
        );
      }

      // 链接处理 (简化版)
      const linkRegex = /\[([^\]]+)\]\(([^)]+)\)/g;
      if (linkRegex.test(line)) {
        const parts = line.split(linkRegex);
        return (
          <p key={index} className="mb-3 text-gray-600 leading-relaxed">
            {parts.map((part, i) => {
              if (i % 3 === 1) {
                // 链接文本
                const url = parts[i + 1];
                return (
                  <a
                    key={i}
                    href={url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 hover:text-blue-800 underline transition-colors"
                  >
                    {part}
                  </a>
                );
              }
              if (i % 3 === 2) {
                // URL部分，跳过
                return null;
              }
              return part;
            })}
          </p>
        );
      }

      // 代码块处理
      if (line.startsWith('```')) {
        return null; // 简化处理，忽略代码块标记
      }

      // 内联代码处理
      if (line.includes('`')) {
        const parts = line.split(/(`[^`]+`)/);
        return (
          <p key={index} className="mb-3 text-gray-600 leading-relaxed">
            {parts.map((part, i) => {
              if (part.startsWith('`') && part.endsWith('`')) {
                return (
                  <code key={i} className="bg-gray-100 px-2 py-1 rounded text-sm font-mono text-gray-800">
                    {part.slice(1, -1)}
                  </code>
                );
              }
              return part;
            })}
          </p>
        );
      }

      // 普通段落
      if (line.trim()) {
        return (
          <p key={index} className="mb-3 text-gray-600 leading-relaxed">
            {line}
          </p>
        );
      }

      // 空行
      return <br key={index} />;
    });
  };

  /**
   * 获取重要性级别样式
   */
  const getLevelStyles = () => {
    switch (level) {
      case 'required':
        return {
          badge: 'bg-red-100 text-red-800 border-red-200',
          title: 'text-red-800',
          description: '此依赖为必需组件，必须完成安装才能继续使用应用。'
        };
      case 'optional':
        return {
          badge: 'bg-yellow-100 text-yellow-800 border-yellow-200',
          title: 'text-yellow-800',
          description: '此依赖为可选组件，建议安装以获得完整功能体验。'
        };
      default:
        return {
          badge: 'bg-gray-100 text-gray-800 border-gray-200',
          title: 'text-gray-800',
          description: ''
        };
    }
  };

  const levelStyles = getLevelStyles();

  return (
    <div className="max-w-4xl mx-auto p-6">
      {/* 主卡片容器 */}
      <div className="bg-white rounded-lg shadow-lg border border-gray-200 overflow-hidden">
        {/* 头部区域 */}
        <div className="bg-gradient-to-r from-blue-50 to-indigo-50 px-6 py-4 border-b border-gray-200">
          <div className="flex items-center justify-between">
            <div>
              <h1 className={`text-2xl font-bold ${levelStyles.title}`}>
                {guide.title}
              </h1>
              <p className="text-gray-600 mt-1">
                依赖: <span className="font-medium">{guide.dependency_name}</span>
              </p>
            </div>
            <div className={`px-3 py-1 rounded-full text-sm font-medium border ${levelStyles.badge}`}>
              {level === 'required' ? '必需依赖' : '可选依赖'}
            </div>
          </div>
        </div>

        {/* 重要提示 */}
        <div className="px-6 py-4 bg-amber-50 border-b border-amber-200">
          <div className="flex items-start">
            <svg
              className="w-5 h-5 text-amber-600 mt-0.5 mr-2 flex-shrink-0"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                clipRule="evenodd"
              />
            </svg>
            <div>
              <p className="text-amber-800 font-medium">安装说明</p>
              <p className="text-amber-700 text-sm mt-1">{levelStyles.description}</p>
            </div>
          </div>
        </div>

        {/* 安装指引内容 */}
        <div className="px-6 py-6">
          <div className="prose prose-sm max-w-none">
            {renderMarkdown(guide.content)}
          </div>

          {/* 相关链接 */}
          {guide.links && guide.links.length > 0 && (
            <div className="mt-8 p-4 bg-gray-50 rounded-lg">
              <h3 className="text-lg font-semibold text-gray-800 mb-3">相关链接</h3>
              <div className="space-y-2">
                {guide.links.map((link, index) => (
                  <a
                    key={index}
                    href={link.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center p-3 bg-white rounded-md border border-gray-200 hover:border-blue-300 hover:shadow-sm transition-all duration-200"
                  >
                    <svg
                      className="w-5 h-5 text-blue-600 mr-3 flex-shrink-0"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
                      />
                    </svg>
                    <div className="flex-1">
                      <p className="font-medium text-gray-800">{link.text}</p>
                      <p className="text-sm text-gray-500 truncate">{link.url}</p>
                    </div>
                    <svg
                      className="w-4 h-4 text-gray-400 ml-2"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                      />
                    </svg>
                  </a>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* 操作按钮区域 */}
        <div className="px-6 py-4 bg-gray-50 border-t border-gray-200">
          <div className="flex items-center justify-between">
            <div className="text-sm text-gray-600">
              完成安装后，点击"重新检测"按钮验证安装结果
            </div>
            <button
              onClick={handleRecheck}
              disabled={isRechecking}
              className={`
                px-6 py-2 rounded-md font-medium transition-all duration-200
                ${isRechecking
                  ? 'bg-gray-300 text-gray-500 cursor-not-allowed'
                  : 'bg-blue-600 text-white hover:bg-blue-700 active:scale-95 shadow-sm hover:shadow-md'
                }
              `}
            >
              {isRechecking ? (
                <span className="flex items-center">
                  <svg
                    className="animate-spin -ml-1 mr-2 h-4 w-4 text-white"
                    fill="none"
                    viewBox="0 0 24 24"
                  >
                    <circle
                      className="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      strokeWidth="4"
                    />
                    <path
                      className="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    />
                  </svg>
                  检测中...
                </span>
              ) : (
                '重新检测'
              )}
            </button>
          </div>
        </div>
      </div>

      {/* 平台兼容性信息 */}
      {guide.target_os && guide.target_os.length > 0 && (
        <div className="mt-4 text-center text-sm text-gray-500">
          支持平台: {guide.target_os.join(', ')}
        </div>
      )}
    </div>
  );
};

export default InstallationGuide;