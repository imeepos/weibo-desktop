/**
 * 微博关键字增量爬取 - 前端类型定义
 *
 * Feature: 003-
 * 对应后端模型: src-tauri/src/models/crawl_*.rs
 */

/**
 * 爬取任务状态
 */
export type CrawlStatus =
  | 'Created'              // 已创建,未开始
  | 'HistoryCrawling'      // 历史回溯中
  | 'HistoryCompleted'     // 历史回溯完成
  | 'IncrementalCrawling'  // 增量更新中
  | 'Paused'               // 已暂停
  | 'Failed';              // 失败

/**
 * 爬取方向
 */
export type CrawlDirection =
  | 'Backward'  // 向后回溯 (从现在到事件开始时间)
  | 'Forward';  // 向前更新 (从最大时间到现在)

/**
 * 爬取任务
 */
export interface CrawlTask {
  /** 任务ID (UUID v4) */
  taskId: string;

  /** 搜索关键字 */
  keyword: string;

  /** 事件开始时间 (ISO 8601) */
  eventStartTime: string;

  /** 任务状态 */
  status: CrawlStatus;

  /** 已爬取的最小帖子时间 (ISO 8601) */
  minPostTime: string | null;

  /** 已爬取的最大帖子时间 (ISO 8601) */
  maxPostTime: string | null;

  /** 已爬取帖子总数 */
  crawledCount: number;

  /** 任务创建时间 (ISO 8601) */
  createdAt: string;

  /** 最后更新时间 (ISO 8601) */
  updatedAt: string;

  /** 失败原因 (仅当status=Failed时有值) */
  failureReason: string | null;
}

/**
 * 微博帖子
 */
export interface WeiboPost {
  /** 微博帖子ID */
  id: string;

  /** 所属任务ID */
  taskId: string;

  /** 帖子内容 */
  text: string;

  /** 发布时间 (ISO 8601) */
  createdAt: string;

  /** 作者UID */
  authorUid: string;

  /** 作者昵称 */
  authorScreenName: string;

  /** 转发数 */
  repostsCount: number;

  /** 评论数 */
  commentsCount: number;

  /** 点赞数 */
  attitudesCount: number;

  /** 爬取时间 (ISO 8601) */
  crawledAt: string;
}

/**
 * 爬取检查点
 */
export interface CrawlCheckpoint {
  /** 任务ID */
  taskId: string;

  /** 当前时间分片的起始时间 (ISO 8601) */
  shardStartTime: string;

  /** 当前时间分片的结束时间 (ISO 8601) */
  shardEndTime: string;

  /** 当前分片内的页码 (1-50) */
  currentPage: number;

  /** 爬取方向 */
  direction: CrawlDirection;

  /** 已完成的时间分片列表 */
  completedShards: Array<{
    start: string;
    end: string;
  }>;

  /** 检查点保存时间 (ISO 8601) */
  savedAt: string;
}

/**
 * 爬取进度事件
 */
export interface CrawlProgressEvent {
  /** 任务ID */
  taskId: string;

  /** 任务状态 */
  status: 'HistoryCrawling' | 'IncrementalCrawling';

  /** 当前时间范围 */
  currentTimeRange: {
    start: string;  // ISO 8601
    end: string;    // ISO 8601
  };

  /** 当前页码 */
  currentPage: number;

  /** 已爬取帖子数 */
  crawledCount: number;

  /** 事件时间戳 (ISO 8601) */
  timestamp: string;
}

/**
 * 爬取完成事件
 */
export interface CrawlCompletedEvent {
  /** 任务ID */
  taskId: string;

  /** 最终状态 */
  finalStatus: 'HistoryCompleted' | 'IncrementalCrawling';

  /** 总爬取数量 */
  totalCrawled: number;

  /** 持续时间 (秒) */
  duration: number;

  /** 事件时间戳 (ISO 8601) */
  timestamp: string;
}

/**
 * 爬取错误事件
 */
export interface CrawlErrorEvent {
  /** 任务ID */
  taskId: string;

  /** 错误消息 */
  error: string;

  /** 错误代码 */
  errorCode: 'CAPTCHA_DETECTED' | 'NETWORK_ERROR' | 'STORAGE_ERROR';

  /** 事件时间戳 (ISO 8601) */
  timestamp: string;
}

/**
 * 任务摘要 (用于列表展示)
 */
export interface CrawlTaskSummary {
  /** 任务ID */
  taskId: string;

  /** 搜索关键字 */
  keyword: string;

  /** 任务状态 */
  status: CrawlStatus;

  /** 事件开始时间 (ISO 8601) */
  eventStartTime: string;

  /** 已爬取帖子总数 */
  crawledCount: number;

  /** 任务创建时间 (ISO 8601) */
  createdAt: string;

  /** 最后更新时间 (ISO 8601) */
  updatedAt: string;

  /** 失败原因 (仅当status=Failed时有值) */
  failureReason: string | null;
}

/**
 * 任务进度信息
 */
export interface CrawlProgress {
  /** 任务基本信息 */
  task: CrawlTask;

  /** 检查点信息 (如果有) */
  checkpoint: CrawlCheckpoint | null;

  /** 进度百分比 (0-100) */
  progressPercentage: number;
}

/**
 * 导出数据请求
 */
export interface ExportDataRequest {
  /** 任务ID */
  taskId: string;

  /** 导出格式 */
  format: 'json' | 'csv';

  /** 时间范围 (可选) */
  timeRange?: {
    start: string;  // ISO 8601
    end: string;    // ISO 8601
  };
}

/**
 * 导出数据响应
 */
export interface ExportDataResponse {
  /** 导出文件路径 */
  filePath: string;

  /** 导出数据数量 */
  exportedCount: number;
}
