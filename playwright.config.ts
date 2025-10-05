import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright 配置 - Tauri 应用 UI 自动化测试
 *
 * 设计原则:
 * - Docker 无头环境友好
 * - 等待 Tauri 应用完全启动
 * - 清晰的测试输出
 * - 失败时自动截图和视频
 */
export default defineConfig({
  // 测试目录
  testDir: './e2e',

  // 测试文件匹配模式
  testMatch: '**/*.spec.ts',

  // 全局超时: 每个测试最多 60 秒
  timeout: 60 * 1000,

  // 期望超时: 每个断言最多 5 秒
  expect: {
    timeout: 5000,
  },

  // 失败时行为
  fullyParallel: false, // Tauri 单实例,不并行
  forbidOnly: !!process.env.CI, // CI 环境禁止 .only
  retries: process.env.CI ? 2 : 0, // CI 重试 2 次
  workers: 1, // 单个 worker,避免端口冲突

  // 测试报告
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['list'],
    ['json', { outputFile: 'playwright-report/results.json' }],
  ],

  // 输出目录
  outputDir: 'test-results/',

  use: {
    // 基础 URL: Tauri 开发服务器
    baseURL: 'http://localhost:1420',

    // 追踪配置: 失败时保留
    trace: 'retain-on-failure',

    // 截图配置: 失败时截图
    screenshot: 'only-on-failure',

    // 视频配置: 失败时录制
    video: 'retain-on-failure',

    // 浏览器上下文配置
    viewport: { width: 600, height: 700 }, // 匹配 Tauri 窗口大小

    // 无头模式: Docker 环境必须启用
    headless: true,

    // 操作超时
    actionTimeout: 10 * 1000,
    navigationTimeout: 30 * 1000,
  },

  // 项目配置: Chromium (Tauri 使用 WebView)
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // Docker 环境启动参数
        launchOptions: {
          args: [
            '--no-sandbox',
            '--disable-setuid-sandbox',
            '--disable-dev-shm-usage',
            '--disable-gpu',
          ],
        },
      },
    },
  ],

  // Web 服务器配置: 等待 Tauri 启动
  webServer: {
    command: 'pnpm tauri dev',
    url: 'http://localhost:1420',
    timeout: 120 * 1000, // 等待 2 分钟 (Rust 编译可能较慢)
    reuseExistingServer: !process.env.CI, // 本地开发可复用
    stdout: 'pipe',
    stderr: 'pipe',
  },
});
