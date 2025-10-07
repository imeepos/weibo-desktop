use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use thiserror::Error;

/// Playwright服务错误
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error")]
pub enum PlaywrightError {
    #[error("启动脚本不存在: {path}")]
    ScriptNotFound { path: String },

    #[error("脚本执行失败: {message}")]
    ExecutionFailed { message: String },

    #[error("PID文件不存在: {path}")]
    PidFileNotFound { path: String },

    #[error("进程不存在 (PID: {pid})")]
    ProcessNotFound { pid: u32 },

    #[error("权限不足: {message}")]
    PermissionDenied { message: String },

    #[error("端口已被占用: {port}")]
    PortInUse { port: u16 },

    #[error("日志文件不存在: {path}")]
    LogFileNotFound { path: String },

    #[error("I/O错误: {message}")]
    IoError { message: String },
}

impl From<std::io::Error> for PlaywrightError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match err.kind() {
            ErrorKind::NotFound => PlaywrightError::IoError {
                message: "文件或目录不存在".to_string(),
            },
            ErrorKind::PermissionDenied => PlaywrightError::PermissionDenied {
                message: err.to_string(),
            },
            _ => PlaywrightError::IoError {
                message: err.to_string(),
            },
        }
    }
}

/// Playwright服务状态
#[derive(Debug, Serialize)]
pub struct PlaywrightStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub port: u16,
    pub healthy: bool,
}

/// 启动结果
#[derive(Debug, Serialize)]
pub struct StartResult {
    pub success: bool,
    pub pid: Option<u32>,
    pub message: String,
}

/// 停止结果
#[derive(Debug, Serialize)]
pub struct StopResult {
    pub success: bool,
    pub message: String,
}

const PID_FILE: &str = "/tmp/playwright-server.pid";
const LOG_FILE: &str = "/tmp/playwright-server.log";
const PORT: u16 = 9223;

/// 启动Playwright服务
///
/// 执行启动脚本,该脚本会:
/// 1. 检查服务是否已运行
/// 2. 构建server (npm run build:server)
/// 3. 后台启动服务并保存PID
/// 4. 等待服务就绪 (健康检查)
///
/// 幂等性保证: 如果服务已运行,直接返回成功
#[tauri::command]
pub async fn start_playwright_server() -> Result<StartResult, PlaywrightError> {
    tracing::info!("启动Playwright服务");

    // 检查是否已运行
    if let Ok(status) = check_playwright_server().await {
        if status.running && status.healthy {
            tracing::info!(pid = ?status.pid, "Playwright服务已在运行");
            return Ok(StartResult {
                success: true,
                pid: status.pid,
                message: format!("服务已在运行 (PID: {:?})", status.pid),
            });
        }
    }

    // 执行启动脚本
    let script_path = "/home/ubuntu/worktrees/desktop/scripts/start-playwright-server.sh";
    if !std::path::Path::new(script_path).exists() {
        return Err(PlaywrightError::ScriptNotFound {
            path: script_path.to_string(),
        });
    }

    let output = Command::new("bash")
        .arg(script_path)
        .output()
        .map_err(|e| PlaywrightError::ExecutionFailed {
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PlaywrightError::ExecutionFailed {
            message: stderr.to_string(),
        });
    }

    // 读取PID
    let pid = read_pid().ok();

    tracing::info!(pid = ?pid, "Playwright服务启动成功");

    Ok(StartResult {
        success: true,
        pid,
        message: "服务启动成功".to_string(),
    })
}

/// 停止Playwright服务
///
/// 优雅地关闭服务:
/// 1. 读取PID文件
/// 2. 发送SIGTERM信号
/// 3. 清理PID文件
///
/// 幂等性保证: 如果服务未运行,返回成功(结果一致)
#[tauri::command]
pub async fn stop_playwright_server() -> Result<StopResult, PlaywrightError> {
    tracing::info!("停止Playwright服务");

    // 读取PID
    let pid = match read_pid() {
        Ok(pid) => pid,
        Err(_) => {
            tracing::info!("Playwright服务未运行 (PID文件不存在)");
            return Ok(StopResult {
                success: true,
                message: "服务未运行".to_string(),
            });
        }
    };

    // 检查进程是否存在
    if !is_process_running(pid) {
        tracing::info!(pid = %pid, "进程不存在,清理PID文件");
        let _ = fs::remove_file(PID_FILE);
        return Ok(StopResult {
            success: true,
            message: "服务未运行".to_string(),
        });
    }

    // 发送SIGTERM信号
    let output = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .output()
        .map_err(|e| PlaywrightError::ExecutionFailed {
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PlaywrightError::ExecutionFailed {
            message: stderr.to_string(),
        });
    }

    // 清理PID文件
    let _ = fs::remove_file(PID_FILE);

    tracing::info!(pid = %pid, "Playwright服务已停止");

    Ok(StopResult {
        success: true,
        message: format!("服务已停止 (PID: {})", pid),
    })
}

/// 检查Playwright服务状态
///
/// 多层验证:
/// 1. PID文件存在性
/// 2. 进程存活性 (ps -p $PID)
/// 3. 服务健康性 (HTTP健康检查)
///
/// 返回完整状态画像,用于UI展示和调试
#[tauri::command]
pub async fn check_playwright_server() -> Result<PlaywrightStatus, PlaywrightError> {
    tracing::debug!("检查Playwright服务状态");

    // 读取PID
    let pid = read_pid().ok();

    // 检查进程是否运行
    let running = match pid {
        Some(p) => is_process_running(p),
        None => false,
    };

    // 健康检查 (HTTP请求)
    let healthy = if running { check_health().await } else { false };

    tracing::debug!(
        running = %running,
        pid = ?pid,
        healthy = %healthy,
        "服务状态检查完成"
    );

    Ok(PlaywrightStatus {
        running,
        pid,
        port: PORT,
        healthy,
    })
}

/// 获取Playwright服务日志
///
/// 读取日志文件最后N行,用于调试和监控
/// 默认返回100行,前端可据此:
/// - 排查启动失败原因
/// - 监控运行时错误
/// - 查看请求日志
#[tauri::command]
pub async fn get_playwright_logs(lines: Option<usize>) -> Result<String, PlaywrightError> {
    let lines = lines.unwrap_or(100);

    tracing::debug!(lines = %lines, "获取Playwright服务日志");

    if !std::path::Path::new(LOG_FILE).exists() {
        return Err(PlaywrightError::LogFileNotFound {
            path: LOG_FILE.to_string(),
        });
    }

    let output = Command::new("tail")
        .arg("-n")
        .arg(lines.to_string())
        .arg(LOG_FILE)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PlaywrightError::ExecutionFailed {
            message: stderr.to_string(),
        });
    }

    let logs = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(logs)
}

/// 读取PID文件
fn read_pid() -> Result<u32, PlaywrightError> {
    if !std::path::Path::new(PID_FILE).exists() {
        return Err(PlaywrightError::PidFileNotFound {
            path: PID_FILE.to_string(),
        });
    }

    let content = fs::read_to_string(PID_FILE)?;
    let pid = content
        .trim()
        .parse::<u32>()
        .map_err(|_| PlaywrightError::IoError {
            message: "PID格式无效".to_string(),
        })?;

    Ok(pid)
}

/// 检查进程是否运行
fn is_process_running(pid: u32) -> bool {
    Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// HTTP健康检查
async fn check_health() -> bool {
    let url = format!("http://localhost:{}", PORT);
    reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .is_ok()
}
