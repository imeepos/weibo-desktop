#!/usr/bin/env node
/**
 * 微博Cookies验证器 - Playwright脚本
 *
 * 功能: 使用浏览器自动化验证cookies的有效性
 * 输入: JSON格式的cookies数据
 * 输出: 验证结果,包含UID和有效性状态
 *
 * 使用示例:
 * ```bash
 * echo '{"cookies": "SUB=xxx; SUBP=yyy"}' | node validate-cookies.ts
 * ```
 */

import { chromium } from 'playwright';

interface CookiesInput {
  cookies: string;
}

interface ValidationResult {
  valid: boolean;
  uid?: string;
  error?: string;
}

async function validateCookies(cookiesString: string): Promise<ValidationResult> {
  const browser = await chromium.launch({ headless: true });

  try {
    const context = await browser.newContext();
    const page = await context.newPage();

    // 解析cookies字符串并注入到浏览器
    const cookiePairs = cookiesString.split(';').map(pair => pair.trim());
    const cookies = cookiePairs.map(pair => {
      const [name, value] = pair.split('=');
      return {
        name: name.trim(),
        value: value.trim(),
        domain: '.weibo.com',
        path: '/',
      };
    });

    await context.addCookies(cookies);

    // 访问微博个人资料API
    const response = await page.goto('https://weibo.com/ajax/profile/info', {
      waitUntil: 'networkidle',
      timeout: 10000,
    });

    if (!response) {
      return { valid: false, error: '无法访问API' };
    }

    // 检查响应状态
    if (response.status() !== 200) {
      return { valid: false, error: `HTTP ${response.status()}` };
    }

    // 解析响应JSON
    const data = await response.json();

    // 提取UID
    const uid = data?.data?.user?.id || data?.data?.user?.idstr;

    if (!uid) {
      return { valid: false, error: '无法提取UID' };
    }

    return { valid: true, uid: String(uid) };

  } catch (error) {
    return {
      valid: false,
      error: error instanceof Error ? error.message : String(error),
    };
  } finally {
    await browser.close();
  }
}

// 主入口
async function main() {
  // 从stdin读取JSON输入
  const chunks: Buffer[] = [];

  process.stdin.on('data', (chunk) => {
    chunks.push(chunk);
  });

  process.stdin.on('end', async () => {
    const input = Buffer.concat(chunks).toString('utf-8');

    try {
      const data: CookiesInput = JSON.parse(input);
      const result = await validateCookies(data.cookies);
      console.log(JSON.stringify(result));
      process.exit(result.valid ? 0 : 1);
    } catch (error) {
      console.error(JSON.stringify({
        valid: false,
        error: error instanceof Error ? error.message : String(error),
      }));
      process.exit(1);
    }
  });
}

// 如果是直接运行此脚本
if (require.main === module) {
  main();
}
