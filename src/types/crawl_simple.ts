/**
 * 微博关键字增量爬取 - 简化版前端类型定义
 *
 * Feature: 003- PostgreSQL架构简化版本
 * 对应后端模型: src-tauri/src/models/postgres/
 */

/**
 * 简化的爬取任务状态（6种状态 -> 5种）
 */
export type CrawlStatusSimple =
  | 'Created'      // 已创建,未开始
  | 'Crawling'     // 爬取中（合并HistoryCrawling和IncrementalCrawling）
  | 'Completed'    // 已完成（合并HistoryCompleted）
  | 'Paused'       // 已暂停
  | 'Failed';      // 失败

/**
 * 简化的爬取任务
 */
export interface CrawlTaskSimple {
  /** 任务ID (UUID) */
  id: string;

  /** 搜索关键字 */
  keyword: string;

  /** 事件开始时间 (ISO 8601) */
  eventStartTime: string;

  /** 任务状态 */
  status: CrawlStatusSimple;

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
 * 简化的任务摘要 (用于列表展示)
 */
export interface CrawlTaskSummarySimple {
  /** 任务ID */
  id: string;

  /** 搜索关键字 */
  keyword: string;

  /** 任务状态 */
  status: CrawlStatusSimple;

  /** 事件开始时间 (ISO 8601) */
  eventStartTime: string;

  /** 已爬取帖子总数 */
  crawledCount: number;

  /** 任务创建时间 (ISO 8601) */
  createdAt: string;

  /** 最后更新时间 (ISO 8601) */
  updatedAt: string;

  /** 失败原因 */
  failureReason: string | null;
}

/**
 * 简化的微博帖子
 */
export interface WeiboPostSimple {
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
}

/**
 * 简化的任务进度信息
 */
export interface TaskProgressSimple {
  /** 任务基本信息 */
  task: CrawlTaskSimple;

  /** 实际帖子数量 */
  actualPostCount: number;

  /** 进度百分比 (0-100) */
  progressPercentage: number;
}

/**
 * 简化的创建任务请求
 */
export interface CreateTaskRequestSimple {
  /** 搜索关键字 */
  keyword: string;

  /** 事件开始时间 (ISO 8601) */
  eventStartTime: string;
}

/**
 * 简化的列出任务请求
 */
export interface ListTasksRequestSimple {
  /** 状态过滤 */
  status?: CrawlStatusSimple | null;

  /** 排序字段 */
  sortBy?: string | null;

  /** 排序顺序 */
  sortOrder?: string | null;

  /** 限制数量 */
  limit?: number | null;

  /** 偏移量 */
  offset?: number | null;
}

/**
 * 简化的列出任务响应
 */
export interface ListTasksResponseSimple {
  /** 任务列表 */
  tasks: CrawlTaskSummarySimple[];

  /** 总数 */
  total: number;
}

/**
 * 简化的爬取事件
 */
export interface SimpleCrawlEvent {
  /** 任务ID */
  taskId: string;

  /** 事件类型 */
  eventType: CrawlEventTypeSimple;

  /** 事件时间戳 (ISO 8601) */
  timestamp: string;

  /** 事件数据 */
  data?: any;
}

/**
 * 简化的爬取事件类型
 */
export type CrawlEventTypeSimple =
  | { type: 'Started', data: { keyword: string } }
  | { type: 'Progress', data: { currentPage: number; crawledCount: number; latestPostTime?: string } }
  | { type: 'PostsSaved', data: { count: number } }
  | { type: 'Completed', data: { totalPosts: number; durationSeconds: number } }
  | { type: 'Error', data: { error: string; errorCode: string } };

/**
 * 状态配置
 */
export const CRAWL_STATUS_CONFIG_SIMPLE: Record<CrawlStatusSimple, {
  label: string;
  color: string;
  animated: boolean;
  description: string;
}> = {
  Created: {
    label: '已创建',
    color: 'bg-gray-100 text-gray-700',
    animated: false,
    description: '任务已创建，尚未开始爬取'
  },
  Crawling: {
    label: '爬取中',
    color: 'bg-blue-100 text-blue-700',
    animated: true,
    description: '正在执行爬取任务'
  },
  Completed: {
    label: '已完成',
    color: 'bg-green-100 text-green-700',
    animated: false,
    description: '爬取任务已完成'
  },
  Paused: {
    label: '已暂停',
    color: 'bg-yellow-100 text-yellow-700',
    animated: false,
    description: '任务已暂停，可以继续执行'
  },
  Failed: {
    label: '失败',
    color: 'bg-red-100 text-red-700',
    animated: false,
    description: '任务执行失败，请检查错误信息'
  }
};

/**
 * 工具函数：检查状态是否可以开始爬取
 */
export function canStartCrawling(status: CrawlStatusSimple): boolean {
  return status === 'Created' || status === 'Paused';
}

/**
 * 工具函数：检查状态是否正在爬取
 */
export function isCrawling(status: CrawlStatusSimple): boolean {
  return status === 'Crawling';
}

/**
 * 工具函数：检查状态是否已完成
 */
export function isCompleted(status: CrawlStatusSimple): boolean {
  return status === 'Completed';
}

/**
 * 工具函数：检查状态是否失败
 */
export function isFailed(status: CrawlStatusSimple): boolean {
  return status === 'Failed';
}

/**
 * 工具函数：获取状态颜色类名
 */
export function getStatusColorClass(status: CrawlStatusSimple): string {
  return CRAWL_STATUS_CONFIG_SIMPLE[status].color;
}

/**
 * 工具函数：获取状态标签
 */
export function getStatusLabel(status: CrawlStatusSimple): string {
  return CRAWL_STATUS_CONFIG_SIMPLE[status].label;
}

/**
 * 工具函数：格式化时间
 */
export function formatDateTimeSimple(isoString: string): string {
  return new Date(isoString).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * 工具函数：计算相对时间
 */
export function getRelativeTime(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffDays = Math.floor(diffHours / 24);

  if (diffDays > 0) {
    return `${diffDays}天前`;
  } else if (diffHours > 0) {
    return `${diffHours}小时前`;
  } else {
    const diffMinutes = Math.floor(diffMs / (1000 * 60));
    return diffMinutes > 0 ? `${diffMinutes}分钟前` : '刚刚';
  }
}