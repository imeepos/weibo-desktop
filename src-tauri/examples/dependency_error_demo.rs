//! DependencyError 使用示例
//!
//! 展示如何使用 DependencyError 枚举及其各种功能

use std::io::{self, ErrorKind};
use weibo_login::models::errors::{DependencyError, InstallErrorType};
use serde_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DependencyError 功能演示 ===\n");

    // 1. 创建不同类型的错误
    println!("1. 创建不同类型的 DependencyError:");

    let check_failed = DependencyError::CheckFailed(
        "Failed to execute command 'node --version': No such file or directory".to_string()
    );
    println!("   CheckFailed: {}", check_failed);

    let not_auto_installable = DependencyError::NotAutoInstallable("nodejs".to_string());
    println!("   NotAutoInstallable: {}", not_auto_installable);

    let install_failed = DependencyError::InstallFailed(InstallErrorType::NetworkError);
    println!("   InstallFailed: {}", install_failed);

    let already_satisfied = DependencyError::AlreadySatisfied("pnpm".to_string(), "8.10.0".to_string());
    println!("   AlreadySatisfied: {}", already_satisfied);

    let not_found = DependencyError::NotFound("nonexistent".to_string());
    println!("   NotFound: {}\n", not_found);

    // 2. JSON 序列化演示
    println!("2. JSON 序列化演示:");

    let error = DependencyError::CheckFailed(
        "Failed to check Redis: Connection refused to localhost:6379".to_string()
    );

    let json = serde_json::to_string_pretty(&error)?;
    println!("   JSON 格式:\n{}\n", json);

    // 3. JSON 反序列化演示
    println!("3. JSON 反序列化演示:");

    let json_str = r#"
    {
      "error": "CheckFailed",
      "details": "Failed to check Node.js version: Command 'node --version' timed out after 5 seconds"
    }
    "#;

    let deserialized: DependencyError = serde_json::from_str(json_str)?;
    println!("   反序列化结果: {}\n", deserialized);

    // 4. From<io::Error> 转换演示
    println!("4. From<io::Error> 转换演示:");

    let io_errors = vec![
        ("Permission Denied", ErrorKind::PermissionDenied),
        ("File Not Found", ErrorKind::NotFound),
        ("Connection Refused", ErrorKind::ConnectionRefused),
        ("Connection Timed Out", ErrorKind::TimedOut),
        ("Unexpected EOF", ErrorKind::UnexpectedEof),
        ("Generic I/O Error", ErrorKind::Other),
    ];

    for (desc, error_kind) in io_errors {
        let io_error = io::Error::new(error_kind, desc);
        let dep_error: DependencyError = io_error.into();
        println!("   {} -> {}", desc, dep_error);
    }
    println!();

    // 5. 错误类型分类演示
    println!("5. InstallErrorType 分类演示:");

    let install_errors = vec![
        InstallErrorType::NetworkError,
        InstallErrorType::PermissionError,
        InstallErrorType::DiskSpaceError,
        InstallErrorType::VersionConflictError,
        InstallErrorType::UnknownError,
    ];

    for error_type in install_errors {
        let error = DependencyError::InstallFailed(error_type.clone());
        let json = serde_json::to_string(&error)?;
        println!("   {} -> {}", error_type, json);
    }

    println!("\n=== 演示完成 ===");
    Ok(())
}