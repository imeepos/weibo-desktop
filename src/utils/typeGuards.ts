import { CommandError } from '../types/error';

/**
 * 类型守卫函数：检查对象是否为CommandError结构
 */
export function isCommandError(value: unknown): value is CommandError {
  return (
    value != null &&
    typeof value === 'object' &&
    'error' in value &&
    'code' in value &&
    typeof (value as CommandError).error === 'string' &&
    typeof (value as CommandError).code === 'string'
  );
}

/**
 * 解析可能的CommandError
 * 支持多种格式：
 * 1. CommandError对象
 * 2. JSON字符串
 * 3. 嵌套在其他对象中的CommandError
 */
export function parseCommandError(error: unknown): CommandError | null {
  // 1. 直接是CommandError对象
  if (isCommandError(error)) {
    return error;
  }

  // 2. 是字符串，尝试解析JSON
  if (typeof error === 'string') {
    try {
      const parsed = JSON.parse(error);
      if (isCommandError(parsed)) {
        return parsed;
      }
    } catch {
      // 不是有效的JSON，返回null
      return null;
    }
  }

  // 3. 嵌套在对象中（如TauriError的payload）
  if (error != null && typeof error === 'object') {
    // 检查message字段
    if ('message' in error && typeof error.message === 'string') {
      try {
        const parsed = JSON.parse(error.message);
        if (isCommandError(parsed)) {
          return parsed;
        }
      } catch {
        // 不是JSON，继续检查其他字段
      }
    }

    // 检查payload字段
    if ('payload' in error) {
      const payload = (error as { payload: unknown }).payload;
      if (isCommandError(payload)) {
        return payload;
      }

      // payload可能是字符串
      if (typeof payload === 'string') {
        try {
          const parsed = JSON.parse(payload);
          if (isCommandError(parsed)) {
            return parsed;
          }
        } catch {
          // 忽略
        }
      }
    }
  }

  return null;
}
