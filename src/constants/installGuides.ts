export const INSTALL_GUIDES: Record<string, string> = {
  redis: `## 安装 Redis Server

Redis 是内存数据库，用于存储用户会话和缓存数据。

### 方式1: Docker (推荐)
\`\`\`bash
docker run -d -p 6379:6379 redis:7-alpine
\`\`\`

### 方式2: 手动安装
1. 访问 [Redis官网](https://redis.io/download)
2. 下载适合您操作系统的版本
3. 按照官方文档完成安装
4. 启动Redis服务: \`redis-server\`

### 验证安装
\`\`\`bash
redis-cli ping
# 应该返回: PONG
\`\`\``,

  playwright: `## 安装 Playwright

Playwright 用于浏览器自动化测试。

### 安装命令
\`\`\`bash
pnpm install playwright
npx playwright install
\`\`\`

### 验证安装
\`\`\`bash
npx playwright --version
\`\`\``,

  node: `## 安装 Node.js

Node.js 是 JavaScript 运行时环境。

### 方式1: 官网下载
1. 访问 [Node.js官网](https://nodejs.org/)
2. 下载 LTS 版本 (推荐 20.x 或更高)
3. 运行安装程序并按提示完成安装

### 方式2: 包管理器
**macOS (使用 Homebrew):**
\`\`\`bash
brew install node
\`\`\`

**Linux (使用 apt):**
\`\`\`bash
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt-get install -y nodejs
\`\`\`

### 验证安装
\`\`\`bash
node --version
npm --version
\`\`\``,

  nodejs: `## 安装 Node.js

Node.js 是 JavaScript 运行时环境。

### 方式1: 官网下载
1. 访问 [Node.js官网](https://nodejs.org/)
2. 下载 LTS 版本 (推荐 20.x 或更高)
3. 运行安装程序并按提示完成安装

### 方式2: 包管理器
**macOS (使用 Homebrew):**
\`\`\`bash
brew install node
\`\`\`

**Linux (使用 apt):**
\`\`\`bash
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt-get install -y nodejs
\`\`\`

### 验证安装
\`\`\`bash
node --version
npm --version
\`\`\``,

  pnpm: `## 安装 pnpm

pnpm 是快速、节省磁盘空间的包管理器。

### 安装命令
\`\`\`bash
npm install -g pnpm
\`\`\`

### 验证安装
\`\`\`bash
pnpm --version
\`\`\``,

  'playwright-browsers': `## 安装 Playwright 浏览器

Playwright 浏览器引擎用于自动化测试。

### 安装命令
\`\`\`bash
npx playwright install
\`\`\`

### 验证安装
\`\`\`bash
npx playwright --version
\`\`\``
};
