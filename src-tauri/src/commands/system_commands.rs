//! 系统相关命令
//!
//! 当前仅提供打开文件位置的功能,用于导出完成后快速定位文件。

use std::path::Path;
use std::process::Command;

/// 打开文件所在位置
///
/// 根据不同平台调用系统指令:
/// - macOS: `open -R`
/// - Windows: `explorer /select,`
/// - Linux: `xdg-open`
///
/// 返回:
/// - 成功: `Ok(())`
/// - 失败: `Err(String)` 包含错误信息
#[tauri::command]
pub async fn open_file_location(path: String) -> Result<(), String> {
    let path_obj = Path::new(&path);

    if !path_obj.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开路径失败: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer");
        if path_obj.is_file() {
            command.arg("/select,").arg(&path);
        } else {
            command.arg(&path);
        }

        command
            .spawn()
            .map_err(|e| format!("打开路径失败: {}", e))?;
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let target = if path_obj.is_file() {
            path_obj
                .parent()
                .unwrap_or(path_obj)
                .to_path_buf()
                .into_os_string()
        } else {
            path_obj.as_os_str().to_os_string()
        };

        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("打开路径失败: {}", e))?;
    }

    Ok(())
}
