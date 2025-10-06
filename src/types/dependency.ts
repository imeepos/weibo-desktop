export enum DependencyLevel {
  Required = 'required',
  Optional = 'optional',
}

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

export enum CheckStatus {
  Satisfied = 'satisfied',
  Missing = 'missing',
  VersionMismatch = 'version_mismatch',
  Corrupted = 'corrupted',
}

export interface DependencyCheckResult {
  dependency_id: string;
  checked_at: string;
  status: CheckStatus;
  detected_version?: string;
  error_details?: string;
  duration_ms: number;
}

export enum InstallStatus {
  Pending = 'pending',
  Downloading = 'downloading',
  Installing = 'installing',
  Success = 'success',
  Failed = 'failed',
}

export enum InstallErrorType {
  NetworkError = 'network_error',
  PermissionError = 'permission_error',
  DiskSpaceError = 'disk_space_error',
  VersionConflictError = 'version_conflict_error',
  UnknownError = 'unknown_error',
}

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

export interface DependencyCheckProgressEvent {
  current_index: number;
  total_count: number;
  current_dependency: string;
  status: string;
}

export interface InstallationProgressEvent {
  task_id: string;
  dependency_id: string;
  status: string;
  progress_percent: number;
  error_message?: string;
}