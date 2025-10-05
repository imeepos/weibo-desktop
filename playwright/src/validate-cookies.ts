/**
 * 微博Cookies验证脚本
 *
 * 使用Playwright启动headless浏览器,设置cookies后访问微博个人资料API,
 * 验证cookies是否有效,并提取UID和用户昵称。
 *
 * 输入: JSON格式的cookies对象 (通过命令行参数或stdin)
 * 输出: JSON格式的验证结果
 *
 * 示例:
 * ```bash
 * node dist/validate-cookies.js '{"SUB":"xxx","SUBP":"yyy"}'
 * ```
 */

import { chromium, Cookie } from 'playwright';

/// 输入的cookies格式
interface InputCookies {
  [key: string]: string;
}

/// 验证结果
interface ValidationResult {
  valid: boolean;
  uid?: string;
  screen_name?: string;
  error?: string;
}

/**
 * 将HashMap<String, String>转换为Playwright的Cookie格式
 */
function convertToCookies(inputCookies: InputCookies): Cookie[] {
  const cookies: Cookie[] = [];

  for (const [name, value] of Object.entries(inputCookies)) {
    cookies.push({
      name,
      value,
      domain: '.weibo.com',
      path: '/',
      expires: -1,
      httpOnly: false,
      secure: true,
      sameSite: 'Lax',
    });
  }

  return cookies;
}

/**
 * 微博VIP中心API响应格式
 */
interface VipCenterResponse {
  code: number;
  data?: {
    uid?: string;
    nickname?: string;
  };
  msg?: string;
}

/**
 * 验证cookies有效性
 *
 * 使用微博VIP中心API验证cookies,该API返回用户的uid和昵称。
 * 成功标准: code === 100000 且存在 data.uid
 */
async function validateCookies(inputCookies: InputCookies): Promise<ValidationResult> {
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox'],
  });

  try {
    const context = await browser.newContext({
      userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
    });

    const cookies = convertToCookies(inputCookies);
    await context.addCookies(cookies);

    // 使用context.request直接调用VIP中心API,优雅且高效
    const response = await context.request.get('https://vip.weibo.com/aj/vipcenter/user', {
      headers: {
        'accept': 'application/json, text/plain, */*',
        'referer': 'https://vip.weibo.com/home',
      },
      timeout: 10000,
    });

    if (!response.ok()) {
      return {
        valid: false,
        error: `HTTP ${response.status()}: Failed to access VIP center`,
      };
    }

    const vipData: VipCenterResponse = await response.json();

    // 验证响应格式和必要字段
    if (vipData.code !== 100000 || !vipData.data?.uid) {
      return {
        valid: false,
        error: vipData.msg || 'Invalid cookies or missing uid',
      };
    }

    return {
      valid: true,
      uid: vipData.data.uid,
      screen_name: vipData.data.nickname || 'Unknown',
    };

  } catch (error) {
    return {
      valid: false,
      error: error instanceof Error ? error.message : String(error),
    };
  } finally {
    await browser.close();
  }
}

/**
 * 主函数
 */
async function main() {
  try {
    // 从命令行参数读取cookies JSON
    const cookiesJson = process.argv[2];

    if (!cookiesJson) {
      console.error(JSON.stringify({
        valid: false,
        error: 'Missing cookies argument. Usage: node validate-cookies.js \'{"SUB":"xxx"}\'',
      }));
      process.exit(1);
    }

    // 解析JSON
    const inputCookies: InputCookies = JSON.parse(cookiesJson);

    // 验证cookies
    const result = await validateCookies(inputCookies);

    // 输出结果到stdout
    console.log(JSON.stringify(result));

    // 退出码: 0=成功, 1=失败
    process.exit(result.valid ? 0 : 1);

  } catch (error) {
    console.error(JSON.stringify({
      valid: false,
      error: error instanceof Error ? error.message : String(error),
    }));
    process.exit(1);
  }
}

// 运行主函数
main().catch((error) => {
  console.error(JSON.stringify({
    valid: false,
    error: error.message,
  }));
  process.exit(1);
});
