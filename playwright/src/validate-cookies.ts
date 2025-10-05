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
 * 验证cookies有效性
 */
async function validateCookies(inputCookies: InputCookies): Promise<ValidationResult> {
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox'],
  });

  try {
    const context = await browser.newContext({
      userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
    });

    // 设置cookies
    const cookies = convertToCookies(inputCookies);
    await context.addCookies(cookies);

    // 访问微博个人资料API
    // 注意: 这里使用的是微博移动端API,实际URL可能需要根据微博开放平台调整
    const page = await context.newPage();

    // 方案1: 访问个人主页,从HTML中提取信息
    const response = await page.goto('https://m.weibo.cn/profile/info', {
      waitUntil: 'networkidle',
      timeout: 10000,
    });

    if (!response || !response.ok()) {
      return {
        valid: false,
        error: `HTTP ${response?.status() || 'N/A'}: Failed to load profile`,
      };
    }

    // 尝试从页面中提取JSON数据
    const content = await page.content();

    // 微博移动端会在页面中嵌入JSON数据
    const jsonMatch = content.match(/\$render_data\s*=\s*(\[.*?\])\[0\]/s);
    if (!jsonMatch) {
      // 如果未找到JSON,可能是未登录
      return {
        valid: false,
        error: 'Not logged in or cookies invalid',
      };
    }

    const renderData = JSON.parse(jsonMatch[1])[0];
    const profileData = renderData?.status?.user || renderData?.user;

    if (!profileData) {
      return {
        valid: false,
        error: 'Failed to parse user profile',
      };
    }

    // 提取UID和昵称
    const uid = profileData.id?.toString() || profileData.idstr;
    const screenName = profileData.screen_name;

    if (!uid) {
      return {
        valid: false,
        error: 'UID not found in profile',
      };
    }

    return {
      valid: true,
      uid,
      screen_name: screenName || 'Unknown',
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
