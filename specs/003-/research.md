# Research: 微博关键字增量爬取

**Feature**: 003- | **Date**: 2025-10-07
**Purpose**: 解决spec.md中的所有NEEDS CLARIFICATION,研究技术实现方案

---

## 1. 微博搜索URL结构和限制

### 决策: 使用微博移动端搜索API

**URL结构**:
```
https://m.weibo.cn/api/container/getIndex?containerid=100103type=1&q={keyword}&page={page}
```

**参数说明**:
- `containerid=100103type=1`: 固定值,表示综合搜索
- `q`: 搜索关键字 (需URL编码)
- `page`: 页码,从1开始
- 可选参数:
  - `starttime`: 开始时间 (格式: `YYYYMMDDhhmmss`,精度到小时)
  - `endtime`: 结束时间 (格式: `YYYYMMDDhhmmss`,精度到小时)

**限制**:
1. **50页限制**: 无论关键字热度,最多只能翻到第50页 (每页约20条帖子,共1000条)
2. **时间精度**: `starttime`和`endtime`仅支持到小时级别,秒和分钟会被忽略
3. **时间范围限制**: 单次搜索的时间范围不能超过365天
4. **结果排序**: 默认按时间倒序 (最新的在前)

**响应结构**:
```json
{
  "ok": 1,
  "data": {
    "cards": [
      {
        "mblog": {
          "id": "5008471234567890",
          "mid": "5008471234567890",
          "text": "帖子内容...",
          "created_at": "Mon Oct 07 12:34:56 +0800 2025",
          "user": {
            "id": 1234567890,
            "screen_name": "用户昵称"
          },
          "reposts_count": 123,
          "comments_count": 456,
          "attitudes_count": 789
        }
      }
    ]
  }
}
```

**理由**: 移动端API比网页端稳定,返回JSON格式易于解析,无需处理复杂的HTML。

**替代方案考虑**:
- 网页端搜索 (被拒绝): HTML结构复杂,易变动,反爬更严格
- 高级搜索API (被拒绝): 需要更高权限的cookies,普通用户无法访问

---

## 2. Redis存储策略

### 决策: 使用Sorted Set + Hash + Set组合存储

**存储结构设计**:

1. **任务信息** (Hash):
   - Key: `crawl:task:{task_id}`
   - Fields:
     ```
     keyword: "国庆"
     event_start_time: "1696118400"  # 时间戳
     status: "HistoryCrawling"
     created_at: "1696204800"
     updated_at: "1696204800"
     crawled_count: "12345"
     min_post_time: "1696118400"  # 已爬取的最小时间
     max_post_time: "1696204800"  # 已爬取的最大时间
     ```

2. **帖子存储** (Sorted Set):
   - Key: `crawl:posts:{task_id}`
   - Score: 帖子发布时间戳 (秒级)
   - Member: 序列化的帖子JSON
   - 优势:
     - `ZADD`: O(log N) 插入,自动按时间排序
     - `ZRANGEBYSCORE`: O(log N + M) 按时间范围查询
     - `ZCARD`: O(1) 获取总数
     - `ZREM`: O(log N) 删除指定帖子

3. **去重索引** (Set):
   - Key: `crawl:post_ids:{task_id}`
   - Members: 所有帖子ID (例如: `"5008471234567890"`)
   - 用途: O(1)时间检查帖子是否已存在,避免重复存储
   - 命令: `SADD` (自动去重)

4. **检查点** (Hash):
   - Key: `crawl:checkpoint:{task_id}`
   - Fields:
     ```
     start_time: "1696118400"
     end_time: "1696122000"
     current_page: "15"
     direction: "Backward"
     saved_at: "1696204800"
     ```

**TTL策略**:
- 任务信息: 90天 (任务完成后保留记录)
- 帖子数据: 90天 (长期存储,支持多次导出)
- 检查点: 与任务同生命周期
- 去重索引: 与任务同生命周期

**理由**:
- Sorted Set天然支持按时间排序和范围查询,无需额外索引
- Set提供O(1)去重检查,避免重复爬取
- Hash存储任务元数据,支持原子更新单个字段
- 总内存估算: 100万帖子 × 1KB (JSON) ≈ 1GB,Redis可承载

**替代方案考虑**:
- Hash存储所有帖子 (被拒绝): 无法按时间范围高效查询
- List存储 (被拒绝): 无排序能力,查询效率低
- PostgreSQL (被拒绝): 引入额外依赖,Redis已足够满足需求

---

## 3. 时间分片算法

### 决策: 递归二分时间范围 + 自适应分片

**核心算法**:

```rust
/// 时间分片策略
///
/// 输入: 时间范围 [start_time, end_time]
/// 输出: Vec<(start, end)> 子时间范围列表
///
/// 规则:
/// 1. 爬取时间范围内的第一页,检查总结果数
/// 2. 如果结果数 ≤ 1000 (50页 × 20条/页),无需分片,直接返回
/// 3. 如果结果数 > 1000,将时间范围二分:
///    - mid = (start + end) / 2
///    - 递归处理 [start, mid] 和 [mid, end]
/// 4. 递归终止条件:
///    - 时间范围 ≤ 1小时 且 结果数仍 > 1000: 记录警告,跳过该时间段
///    - 结果数 ≤ 1000: 返回当前时间范围
fn split_time_range_if_needed(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    keyword: &str,
) -> Result<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
    // 1. 爬取第一页,获取总结果数提示
    let first_page_result = crawl_page(keyword, start, end, 1)?;
    let total_results = estimate_total_results(first_page_result)?;

    // 2. 判断是否需要分片
    const MAX_RESULTS: usize = 1000;
    if total_results <= MAX_RESULTS {
        return Ok(vec![(start, end)]);
    }

    // 3. 检查是否已经无法再分片 (1小时是最小粒度)
    if end.signed_duration_since(start).num_hours() <= 1 {
        tracing::warn!(
            "时间范围 {}-{} 内结果数 {} 超过限制,但无法再分片,将跳过部分数据",
            start, end, total_results
        );
        return Ok(vec![(start, end)]);
    }

    // 4. 二分时间范围
    let mid = start + (end - start) / 2;
    let left_shards = split_time_range_if_needed(start, mid, keyword)?;
    let right_shards = split_time_range_if_needed(mid, end, keyword)?;

    Ok([left_shards, right_shards].concat())
}
```

**边界处理**:

1. **时间取整**:
   - `floor_to_hour(dt)`: 向下取整到小时 (例: `2025-10-07 12:34:56` → `2025-10-07 12:00:00`)
   - `ceil_to_hour(dt)`: 向上取整到小时 (例: `2025-10-07 12:34:56` → `2025-10-07 13:00:00`)
   - 理由: 微博API时间参数仅支持小时精度

2. **时间重叠处理**:
   - 相邻时间分片共享边界小时 (例: `[12:00-13:00]` 和 `[13:00-14:00]`)
   - 依赖帖子ID去重,自动过滤重复数据

3. **最小分片限制**:
   - 最小时间范围: 1小时
   - 如果1小时内仍>1000条结果,记录WARNING日志,只爬取前50页

**理由**:
- 递归二分是经典算法,简洁优雅,易于理解和维护
- 自适应分片避免过度拆分(冷门关键字可能无需分片)
- 最坏时间复杂度: O(log T),T为总时间跨度

**替代方案考虑**:
- 固定1小时分片 (被拒绝): 冷门关键字会产生大量空查询,浪费时间
- 固定1天分片 (被拒绝): 热门关键字仍会超过50页限制
- 预估结果数分片 (被拒绝): 微博API不提供精确总数,只能通过实际爬取判断

---

## 4. 断点续爬检查点设计

### 决策: 三级检查点 (任务级 + 分片级 + 页级)

**检查点结构**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlCheckpoint {
    /// 当前时间分片的起始时间
    pub shard_start_time: DateTime<Utc>,

    /// 当前时间分片的结束时间
    pub shard_end_time: DateTime<Utc>,

    /// 当前分片内的页码 (1-50)
    pub current_page: u32,

    /// 爬取方向
    pub direction: CrawlDirection,

    /// 检查点保存时间
    pub saved_at: DateTime<Utc>,

    /// 已完成的时间分片列表 (用于跳过已完成的分片)
    pub completed_shards: Vec<(DateTime<Utc>, DateTime<Utc>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrawlDirection {
    /// 向后回溯 (从现在到事件开始时间)
    Backward,

    /// 向前更新 (从最大时间到现在)
    Forward,
}
```

**保存策略**:
- 频率: 每页爬取成功后立即保存 (覆盖旧检查点)
- 存储: Redis Hash (`crawl:checkpoint:{task_id}`)
- 原子性: 使用Redis事务保证检查点和帖子数据同步

**恢复逻辑**:

```rust
async fn resume_crawl(task_id: &str) -> Result<()> {
    // 1. 从Redis加载检查点
    let checkpoint = load_checkpoint(task_id).await?;

    // 2. 根据方向决定恢复策略
    match checkpoint.direction {
        CrawlDirection::Backward => {
            // 从检查点的当前页+1继续爬取当前分片
            crawl_shard(
                checkpoint.shard_start_time,
                checkpoint.shard_end_time,
                checkpoint.current_page + 1,
            ).await?;

            // 继续处理未完成的时间分片
            continue_backward_crawl(task_id, checkpoint).await?;
        }
        CrawlDirection::Forward => {
            // 增量爬取: 从最大时间到现在
            crawl_shard(
                checkpoint.shard_start_time,
                Utc::now(),
                checkpoint.current_page + 1,
            ).await?;
        }
    }

    Ok(())
}
```

**理由**:
- 三级检查点保证任意阶段中断都能精确恢复
- `completed_shards`避免重复处理已完成的时间分片
- 每页保存检查点,最多浪费1页的爬取时间(约20秒)

**替代方案考虑**:
- 仅保存任务级进度 (被拒绝): 恢复时无法知道当前时间分片和页码,可能重复爬取
- 每条帖子保存检查点 (被拒绝): 过于频繁,增加Redis压力,收益不大

---

## 5. Playwright爬取脚本架构

### 决策: 复用001-cookies的Playwright server架构

**脚本结构** (`playwright/src/weibo-crawler.ts`):

```typescript
import { chromium, Page } from 'playwright';

/**
 * 微博爬取脚本
 *
 * 通过WebSocket连接接收爬取任务,使用登录态cookies执行搜索,返回帖子列表
 */

interface CrawlRequest {
  keyword: string;
  startTime?: string;  // YYYYMMDDhhmmss
  endTime?: string;
  page: number;
  cookies: Record<string, string>;
}

interface CrawlResult {
  posts: WeiboPost[];
  hasMore: boolean;
  totalResults?: number;
  captchaDetected?: boolean;
}

async function crawlWeiboSearch(request: CrawlRequest): Promise<CrawlResult> {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();

  // 1. 设置cookies
  await context.addCookies(
    Object.entries(request.cookies).map(([name, value]) => ({
      name,
      value,
      domain: '.weibo.cn',
      path: '/',
    }))
  );

  // 2. 构建搜索URL
  const url = buildSearchUrl(request);
  const page = await context.newPage();

  try {
    await page.goto(url, { waitUntil: 'networkidle' });

    // 3. 检测验证码
    if (await page.locator('text=验证码').isVisible()) {
      await page.screenshot({ path: `captcha-${Date.now()}.png` });
      return { posts: [], hasMore: false, captchaDetected: true };
    }

    // 4. 等待搜索结果加载
    await page.waitForSelector('.card-wrap', { timeout: 10000 });

    // 5. 提取帖子数据
    const posts = await page.$$eval('.card-wrap', (cards) => {
      return cards.map((card) => {
        const mblog = card.querySelector('.weibo-text');
        return {
          id: card.getAttribute('mid') || '',
          text: mblog?.textContent || '',
          createdAt: card.querySelector('.from')?.textContent || '',
          // ... 其他字段
        };
      });
    });

    // 6. 检查是否还有更多结果
    const hasMore = await page.locator('text=没有更多了').isVisible() === false;

    return { posts, hasMore };
  } finally {
    await browser.close();
  }
}

function buildSearchUrl(req: CrawlRequest): string {
  const params = new URLSearchParams({
    containerid: '100103type=1',
    q: req.keyword,
    page: req.page.toString(),
  });

  if (req.startTime) params.set('starttime', req.startTime);
  if (req.endTime) params.set('endtime', req.endTime);

  return `https://m.weibo.cn/api/container/getIndex?${params}`;
}
```

**WebSocket通信协议**:

```json
// Rust → Playwright (请求)
{
  "action": "crawl_weibo_search",
  "payload": {
    "keyword": "国庆",
    "startTime": "20251001000000",
    "endTime": "20251001010000",
    "page": 1,
    "cookies": { "SUB": "...", "SUBP": "..." }
  }
}

// Playwright → Rust (响应)
{
  "success": true,
  "data": {
    "posts": [...],
    "hasMore": true
  }
}
```

**理由**:
- 复用已有的Playwright server,避免重复构建基础设施
- WebSocket通信解耦Rust和Node.js,各自独立升级
- Playwright提供稳定的浏览器自动化能力,处理动态加载和验证码

**替代方案考虑**:
- 直接HTTP请求 (被拒绝): 微博返回的是HTML,解析复杂,且容易触发反爬
- Selenium (被拒绝): Playwright性能更好,API更现代
- 直接调用微博API (被拒绝): 需要逆向分析,风险高,且API可能随时变化

---

## 6. 与001-cookies的集成方式

### 决策: 通过`query_cookies` Tauri command获取登录态

**集成流程**:

1. **前端触发爬取**:
   ```typescript
   // 前端代码
   import { invoke } from '@tauri-apps/api/core';

   // 用户可选择使用哪个账号的cookies (如果有多个账号)
   const uids = await invoke<string[]>('list_all_uids');
   const selectedUid = uids[0];

   // 创建爬取任务 (内部会调用query_cookies)
   const taskId = await invoke<string>('create_crawl_task', {
     keyword: '国庆',
     eventStartTime: '2025-10-01T00:00:00Z',
     uid: selectedUid,  // 指定使用哪个账号的cookies
   });
   ```

2. **后端获取cookies**:
   ```rust
   #[tauri::command]
   async fn create_crawl_task(
       keyword: String,
       event_start_time: DateTime<Utc>,
       uid: String,
       state: State<'_, AppState>,
   ) -> Result<String, String> {
       // 1. 从Redis获取cookies
       let cookies_data = state.redis_service
           .query_cookies(&uid)
           .await
           .map_err(|e| format!("获取cookies失败: {}", e))?;

       // 2. 验证cookies是否过期 (检查validated_at)
       let now = Utc::now();
       let age = now.signed_duration_since(cookies_data.validated_at);
       if age.num_days() > 7 {
           return Err("Cookies可能已过期,请重新登录".to_string());
       }

       // 3. 创建爬取任务
       let task = CrawlTask::new(keyword, event_start_time, cookies_data.cookies);
       let task_id = task.id.clone();

       // 4. 保存任务到Redis
       state.redis_service.save_crawl_task(&task).await
           .map_err(|e| format!("保存任务失败: {}", e))?;

       Ok(task_id)
   }
   ```

3. **依赖关系**:
   - CrawlService依赖RedisService (复用连接池)
   - CrawlService不依赖CookiesService (直接使用CookiesData模型)
   - 前端提示用户: "需要先通过扫码登录获取cookies"

**Cookies有效期处理**:
- 001-cookies保存时记录`validated_at`
- 003-爬取任务启动时检查cookies年龄
- 如果>7天,提示用户重新验证 (调用001的`validate_cookies`命令)

**理由**:
- 松耦合: 003功能不依赖001的业务逻辑,仅复用数据
- 用户体验: 无需每次爬取都扫码,一次登录多次使用
- 数据一致性: 所有cookies统一由001管理,避免重复存储

**替代方案考虑**:
- 爬取时重新扫码 (被拒绝): 用户体验差,且增加微博风控风险
- 复制cookies到爬取任务 (被拒绝): 数据冗余,cookies更新时需同步多处

---

## 7. 导出格式

### 决策: 支持JSON和CSV两种格式

**JSON格式** (结构化,支持嵌套):

```json
{
  "task_id": "crawl_20251007_abc123",
  "keyword": "国庆",
  "exported_at": "2025-10-07T12:34:56Z",
  "total_posts": 12345,
  "posts": [
    {
      "id": "5008471234567890",
      "text": "帖子内容...",
      "created_at": "2025-10-07T12:34:56Z",
      "author_uid": "1234567890",
      "author_screen_name": "用户昵称",
      "reposts_count": 123,
      "comments_count": 456,
      "attitudes_count": 789,
      "crawled_at": "2025-10-07T12:35:00Z"
    }
  ]
}
```

**CSV格式** (兼容Excel/数据分析工具):

```csv
post_id,text,created_at,author_uid,author_screen_name,reposts,comments,likes,crawled_at
5008471234567890,"帖子内容...",2025-10-07T12:34:56Z,1234567890,用户昵称,123,456,789,2025-10-07T12:35:00Z
```

**导出实现**:

```rust
#[tauri::command]
async fn export_crawl_data(
    task_id: String,
    format: String,  // "json" | "csv"
    state: State<'_, AppState>,
) -> Result<String, String> {
    // 1. 从Redis读取所有帖子 (使用ZRANGE)
    let posts = state.redis_service
        .get_all_posts(&task_id)
        .await
        .map_err(|e| format!("读取帖子失败: {}", e))?;

    // 2. 根据格式序列化
    let content = match format.as_str() {
        "json" => serde_json::to_string_pretty(&posts)?,
        "csv" => serialize_to_csv(&posts)?,
        _ => return Err("不支持的格式".to_string()),
    };

    // 3. 保存到文件
    let downloads_dir = tauri::api::path::download_dir()
        .ok_or("无法获取下载目录")?;
    let filename = format!("weibo_{}_{}.{}", task_id, Utc::now().timestamp(), format);
    let file_path = downloads_dir.join(filename);

    std::fs::write(&file_path, content)
        .map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(file_path.display().to_string())
}
```

**理由**:
- JSON适合程序处理和数据交换
- CSV适合Excel分析和可视化
- 两种格式覆盖绝大部分使用场景

**替代方案考虑**:
- Excel格式 (被拒绝): 需要额外依赖(xlsx库),增加复杂度
- 数据库导出 (被拒绝): 用户不一定有数据库环境

---

## 8. NEEDS CLARIFICATION解决方案

### FR-018: 爬取cookies从何处获取?

**答案**: 复用001-cookies功能
- 用户先通过001扫码登录,cookies保存在Redis (`weibo:cookies:{uid}`)
- 003创建爬取任务时,通过`query_cookies` Tauri command从Redis读取
- 前端展示可用账号列表 (调用`list_all_uids`),用户选择使用哪个账号

### FR-025: 网络请求重试间隔是多少?

**答案**: 指数退避策略,间隔1秒/2秒/4秒
```rust
const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

async fn crawl_with_retry(url: &str) -> Result<Response> {
    for (attempt, delay) in RETRY_DELAYS.iter().enumerate() {
        match reqwest::get(url).await {
            Ok(resp) => return Ok(resp),
            Err(e) if attempt < 2 => {
                tracing::warn!("第{}次请求失败,{}秒后重试: {}", attempt + 1, delay.as_secs(), e);
                tokio::time::sleep(*delay).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!()
}
```

**理由**: 指数退避避免过快重试触发限流,3次重试覆盖临时网络抖动。

### FR-027: 请求延迟时长是固定还是可配置?

**答案**: 固定随机延迟1-3秒,暂不可配置
```rust
use rand::Rng;

async fn random_delay() {
    let delay_ms = rand::thread_rng().gen_range(1000..=3000);
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
}
```

**理由**:
- 随机延迟模拟人类行为,降低反爬风险
- 1-3秒是经验值,既不会太慢影响效率,也不会太快触发限流
- 暂不可配置,避免过早优化,后续可根据实际情况调整

### 并发策略: 是否支持多任务并行爬取?

**答案**: 单任务顺序执行,不支持并行
- 同一时刻只允许一个任务处于`HistoryCrawling`或`IncrementalCrawling`状态
- 前端尝试启动第二个任务时,返回错误提示
- 理由: 避免并发爬取触发微博限流,保护账号安全

**未来扩展**: 可支持多任务管理 (创建/暂停/恢复),但同一时刻仅一个任务执行。

---

## 研究总结

所有技术方案已明确,所有NEEDS CLARIFICATION已解决,准备进入Phase 1设计阶段。

**关键技术决策**:
1. 时间分片算法突破50页限制
2. Redis Sorted Set实现高效时间范围查询
3. 三级检查点保证断点续爬精确性
4. 复用001-cookies避免重复登录
5. Playwright脚本复用已有架构

**风险与应对**:
- 反爬虫: 随机延迟 + 验证码检测 + 优雅暂停
- 时间精度: 帖子ID去重 + 时间边界重叠处理
- 数据量: Redis Sorted Set支持百万级数据,分页导出避免内存溢出
