/**
 * 依赖检测与安装类型定义
 *
 * 与Rust后端完全对应的类型系统
 * 每个类型都承载着明确的语义
 */

/// 依赖重要性级别
export enum DependencyLevel {
  Required = 'required',
  Optional = 'optional',
}

/// 依赖检测方法类型
export interface CheckMethodExecutable {
  type: 'executable';
  name: string;
  version_args: string[];
}

export interface CheckMethodService {
  type: 'service';
  host: string;
  port: number;
}

export interface CheckMethodFile {
  type: 'file';
  path: string;
}

export type CheckMethod = CheckMethodExecutable | CheckMethodService | CheckMethodFile;

/// 依赖项定义
export interface Dependency {
  id: string;
  name: string;
  version_requirement: string;
  description: string;
  level: DependencyLevel;
  auto_installable: boolean;
  install_priority: number;
  check_method: CheckMethod;
  install_guide: string;
  install_command?: string;
}

/// 依赖检测状态
export enum CheckStatus {
  Satisfied = 'satisfied',
  Missing = 'missing',
  VersionMismatch = 'version_mismatch',
  Corrupted = 'corrupted',
}

/// 依赖检测结果
export interface DependencyCheckResult {
  dependency_id: string;
  checked_at: string;
  status: CheckStatus;
  detected_version?: string;
  error_details?: string;
  duration_ms: number;
}

/// 安装任务状态
export enum InstallStatus {
  Pending = 'pending',
  Downloading = 'downloading',
  Installing = 'installing',
  Success = 'success',
  Failed = 'failed',
}

/// 安装错误类型
export enum InstallErrorType {
  NetworkError = 'network_error',
  PermissionError = 'permission_error',
  DiskSpaceError = 'disk_space_error',
  VersionConflictError = 'version_conflict_error',
  UnknownError = 'unknown_error',
}

/// 安装任务
export interface InstallationTask {
  task_id: string;
  dependency_id: string;
  created_at: string;
  started_at?: string;
  completed_at?: string;
  status: InstallStatus;
  progress_percent: number;
  error_message?: string;
  install_log: string[];
  error_type?: InstallErrorType;
}

/// 依赖检测进度事件
export interface DependencyCheckProgressEvent {
  current_index: number;
  total_count: number;
  current_dependency: string;
  status: string;
}

/// 安装进度事件
export interface InstallationProgressEvent {
  task_id: string;
  dependency_id: string;
  status: string;
  progress_percent: number;
  error_message?: string;
}