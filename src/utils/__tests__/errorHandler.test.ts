/**
 * 错误处理工具测试
 * 验证前后端错误处理机制的一致性
 */

import {
  handleTauriError,
  extractErrorCode,
  hasErrorCode,
  getFriendlyErrorMessage,
  formatTauriError
} from '../errorHandler';
import { CrawlErrorCode, LoginErrorCode } from '../../types/error';

describe('错误处理工具测试', () => {
  describe('CommandError 结构处理', () => {
    test('应该正确处理标准的CommandError结构', () => {
      const commandError = {
        code: 'INVALID_KEYWORD',
        error: '关键字不能为空'
      };

      const result = handleTauriError(commandError);
      expect(result).toBe('关键字不能为空');
    });

    test('应该正确提取CommandError的错误代码', () => {
      const commandError = {
        code: 'COOKIES_EXPIRED',
        error: 'Cookies可能已过期,请重新登录'
      };

      const code = extractErrorCode(commandError);
      expect(code).toBe('COOKIES_EXPIRED');
    });

    test('应该正确匹配特定错误代码', () => {
      const commandError = {
        code: 'TASK_NOT_FOUND',
        error: '任务不存在'
      };

      const isTaskNotFound = hasErrorCode(commandError, CrawlErrorCode.TASK_NOT_FOUND);
      expect(isTaskNotFound).toBe(true);

      const isInvalidKeyword = hasErrorCode(commandError, CrawlErrorCode.INVALID_KEYWORD);
      expect(isInvalidKeyword).toBe(false);
    });
  });

  describe('JSON 字符串处理', () => {
    test('应该正确解析JSON格式的CommandError', () => {
      const jsonError = JSON.stringify({
        code: 'NO_DATA',
        error: '任务尚无数据可导出'
      });

      const result = handleTauriError(jsonError);
      expect(result).toBe('任务尚无数据可导出');
    });

    test('应该从JSON字符串中提取错误代码', () => {
      const jsonError = JSON.stringify({
        code: 'INVALID_FORMAT',
        error: '不支持的导出格式'
      });

      const code = extractErrorCode(jsonError);
      expect(code).toBe('INVALID_FORMAT');
    });
  });

  describe('传统错误处理兼容性', () => {
    test('应该处理包含错误代码的字符串', () => {
      const errorString = 'Redis连接失败: RedisConnectionFailed';
      const result = handleTauriError(errorString);
      expect(result).toBe('Redis连接失败,请检查服务状态');
    });

    test('应该处理TauriError结构', () => {
      const tauriError = {
        message: '登录失败',
        payload: JSON.stringify({
          code: 'LoginFailed',
          error: '登录失败,请重试'
        })
      };

      const result = handleTauriError(tauriError);
      expect(result).toBe('登录失败,请重试');
    });
  });

  describe('错误代码映射完整性测试', () => {
    test('所有爬取错误代码都应该有对应的友好消息', () => {
      for (const code of Object.values(CrawlErrorCode)) {
        const error = { code, error: '测试错误' };
        const result = handleTauriError(error);
        expect(result).not.toBe('测试错误');
        expect(result).not.toBe('未知错误,请稍后重试');
      }
    });

    test('所有登录错误代码都应该有对应的友好消息', () => {
      for (const code of Object.values(LoginErrorCode)) {
        const error = { code, error: '测试错误' };
        const result = handleTauriError(error);
        expect(result).not.toBe('测试错误');
        expect(result).not.toBe('未知错误,请稍后重试');
      }
    });
  });

  describe('边界情况处理', () => {
    test('应该处理未知错误', () => {
      const unknownError = { unknown: 'error' };
      const result = handleTauriError(unknownError);
      expect(result).toBe('未知错误,请稍后重试');
    });

    test('应该处理null和undefined', () => {
      expect(handleTauriError(null)).toBe('未知错误,请稍后重试');
      expect(handleTauriError(undefined)).toBe('未知错误,请稍后重试');
    });

    test('应该处理空字符串', () => {
      const result = handleTauriError('');
      expect(result).toBe('');
    });

    test('应该处理无效JSON', () => {
      const invalidJson = '{"invalid": json}';
      const result = handleTauriError(invalidJson);
      expect(result).toBe(invalidJson);
    });
  });

  describe('与后端错误结构对比测试', () => {
    test('应该匹配后端所有定义的错误代码', () => {
      // 后端定义的错误代码列表
      const backendErrorCodes = [
        'INVALID_KEYWORD',
        'INVALID_TIME',
        'COOKIES_NOT_FOUND',
        'COOKIES_EXPIRED',
        'TASK_NOT_FOUND',
        'INVALID_STATUS',
        'ALREADY_RUNNING',
        'NO_DATA',
        'INVALID_FORMAT',
        'STORAGE_ERROR',
        'FILE_SYSTEM_ERROR'
      ];

      for (const code of backendErrorCodes) {
        const error = { code, error: '测试消息' };
        const result = handleTauriError(error);
        expect(result).not.toBe('测试消息');
        expect(result).not.toBe('未知错误,请稍后重试');
      }
    });

    test('应该处理后端错误消息的动态内容', () => {
      // 模拟后端动态生成的错误消息
      const dynamicError = {
        code: 'COOKIES_NOT_FOUND',
        error: '未找到UID 12345 的Cookies,请先扫码登录'
      };

      const result = handleTauriError(dynamicError);
      expect(result).toBe('未找到Cookies,请先扫码登录');
    });

    test('应该处理后端时间相关的错误消息', () => {
      const timeError = {
        code: 'COOKIES_EXPIRED',
        error: 'Cookies可能已过期(验证时间>15天),请重新登录'
      };

      const result = handleTauriError(timeError);
      expect(result).toBe('Cookies可能已过期,请重新登录');
    });
  });
});