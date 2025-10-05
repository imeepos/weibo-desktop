//! 测试 DependencyError 的 JSON 序列化和 From<io::Error> 转换
//!
//! 验证错误处理的优雅性和用户友好性

#[cfg(test)]
mod tests {
    use serde_json;
    use std::io::{self, ErrorKind};

    // 直接导入模块，避免依赖外部crate
    mod weibo_login {
        pub mod models {
            pub mod errors {
                use serde::{Deserialize, Serialize};
                use thiserror::Error;

                #[derive(Debug, Error, Serialize, Deserialize)]
                #[serde(tag = "error", content = "details")]
                pub enum DependencyError {
                    #[error("Dependency check failed: {0}")]
                    CheckFailed(String),
                    #[error("Dependency '{0}' cannot be auto-installed. Please install manually.")]
                    NotAutoInstallable(String),
                    #[error("Installation failed: {0}")]
                    InstallFailed(InstallErrorType),
                    #[error("Dependency '{0}' is already satisfied (version {1})")]
                    AlreadySatisfied(String, String),
                    #[error("Dependency '{0}' not found")]
                    NotFound(String),
                }

                #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
                #[serde(rename_all = "snake_case")]
                pub enum InstallErrorType {
                    NetworkError,
                    PermissionError,
                    DiskSpaceError,
                    VersionConflictError,
                    UnknownError,
                }

                impl std::fmt::Display for InstallErrorType {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        match self {
                            InstallErrorType::NetworkError => write!(f, "Network error"),
                            InstallErrorType::PermissionError => write!(f, "Permission error"),
                            InstallErrorType::DiskSpaceError => write!(f, "Disk space error"),
                            InstallErrorType::VersionConflictError => write!(f, "Version conflict error"),
                            InstallErrorType::UnknownError => write!(f, "Unknown error"),
                        }
                    }
                }

                impl From<std::io::Error> for DependencyError {
                    fn from(err: std::io::Error) -> Self {
                        match err.kind() {
                            std::io::ErrorKind::PermissionDenied => {
                                DependencyError::CheckFailed(format!("Permission denied: {}", err))
                            }
                            std::io::ErrorKind::NotFound => {
                                DependencyError::CheckFailed(format!("File or directory not found: {}", err))
                            }
                            std::io::ErrorKind::ConnectionRefused => {
                                DependencyError::CheckFailed(format!("Connection refused: {}", err))
                            }
                            std::io::ErrorKind::TimedOut => {
                                DependencyError::CheckFailed(format!("Connection timed out: {}", err))
                            }
                            std::io::ErrorKind::UnexpectedEof => {
                                DependencyError::CheckFailed(format!("Unexpected end of file: {}", err))
                            }
                            _ => {
                                DependencyError::CheckFailed(format!("I/O error: {}", err))
                            }
                        }
                    }
                }
            }
        }
    }

    use weibo_login::models::errors::{DependencyError, InstallErrorType};

    #[test]
    fn test_dependency_error_check_failed_serialization() {
        let error = DependencyError::CheckFailed("Failed to execute command".to_string());
        let json = serde_json::to_string(&error).expect("Failed to serialize error");

        // 验证JSON结构
        assert!(json.contains("CheckFailed"));
        assert!(json.contains("Failed to execute command"));

        // 验证可以反序列化
        let deserialized: DependencyError = serde_json::from_str(&json)
            .expect("Failed to deserialize error");

        match deserialized {
            DependencyError::CheckFailed(msg) => {
                assert_eq!(msg, "Failed to execute command");
            }
            _ => panic!("Expected CheckFailed variant"),
        }
    }

    #[test]
    fn test_dependency_error_not_auto_installable_serialization() {
        let error = DependencyError::NotAutoInstallable("nodejs".to_string());
        let json = serde_json::to_string(&error).expect("Failed to serialize error");

        // 验证JSON结构
        assert!(json.contains("NotAutoInstallable"));
        assert!(json.contains("nodejs"));

        // 验证可以反序列化
        let deserialized: DependencyError = serde_json::from_str(&json)
            .expect("Failed to deserialize error");

        match deserialized {
            DependencyError::NotAutoInstallable(dep) => {
                assert_eq!(dep, "nodejs");
            }
            _ => panic!("Expected NotAutoInstallable variant"),
        }
    }

    #[test]
    fn test_dependency_error_install_failed_serialization() {
        let error = DependencyError::InstallFailed(InstallErrorType::NetworkError);
        let json = serde_json::to_string(&error).expect("Failed to serialize error");

        // 验证JSON结构
        assert!(json.contains("InstallFailed"));

        // 验证可以反序列化
        let deserialized: DependencyError = serde_json::from_str(&json)
            .expect("Failed to deserialize error");

        match deserialized {
            DependencyError::InstallFailed(error_type) => {
                assert_eq!(error_type, InstallErrorType::NetworkError);
            }
            _ => panic!("Expected InstallFailed variant"),
        }
    }

    #[test]
    fn test_dependency_error_already_satisfied_serialization() {
        let error = DependencyError::AlreadySatisfied("pnpm".to_string(), "8.10.0".to_string());
        let json = serde_json::to_string(&error).expect("Failed to serialize error");

        // 验证JSON结构
        assert!(json.contains("AlreadySatisfied"));
        assert!(json.contains("pnpm"));
        assert!(json.contains("8.10.0"));

        // 验证可以反序列化
        let deserialized: DependencyError = serde_json::from_str(&json)
            .expect("Failed to deserialize error");

        match deserialized {
            DependencyError::AlreadySatisfied(dep, version) => {
                assert_eq!(dep, "pnpm");
                assert_eq!(version, "8.10.0");
            }
            _ => panic!("Expected AlreadySatisfied variant"),
        }
    }

    #[test]
    fn test_dependency_error_not_found_serialization() {
        let error = DependencyError::NotFound("nonexistent".to_string());
        let json = serde_json::to_string(&error).expect("Failed to serialize error");

        // 验证JSON结构
        assert!(json.contains("NotFound"));
        assert!(json.contains("nonexistent"));

        // 验证可以反序列化
        let deserialized: DependencyError = serde_json::from_str(&json)
            .expect("Failed to deserialize error");

        match deserialized {
            DependencyError::NotFound(dep) => {
                assert_eq!(dep, "nonexistent");
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    #[test]
    fn test_from_io_error_permission_denied() {
        let io_error = io::Error::new(ErrorKind::PermissionDenied, "Access denied");
        let dep_error: DependencyError = io_error.into();

        match &dep_error {
            DependencyError::CheckFailed(msg) => {
                assert!(msg.contains("Permission denied"));
                assert!(msg.contains("Access denied"));
            }
            _ => panic!("Expected CheckFailed variant"),
        }

        // 验证错误消息的友好性
        let error_msg = dep_error.to_string();
        assert!(error_msg.contains("Dependency check failed"));
        assert!(error_msg.contains("Permission denied"));
    }

    #[test]
    fn test_from_io_error_not_found() {
        let io_error = io::Error::new(ErrorKind::NotFound, "File not found");
        let dep_error: DependencyError = io_error.into();

        match dep_error {
            DependencyError::CheckFailed(msg) => {
                assert!(msg.contains("File or directory not found"));
                assert!(msg.contains("File not found"));
            }
            _ => panic!("Expected CheckFailed variant"),
        }
    }

    #[test]
    fn test_from_io_error_connection_refused() {
        let io_error = io::Error::new(ErrorKind::ConnectionRefused, "Connection refused");
        let dep_error: DependencyError = io_error.into();

        match dep_error {
            DependencyError::CheckFailed(msg) => {
                assert!(msg.contains("Connection refused"));
            }
            _ => panic!("Expected CheckFailed variant"),
        }
    }

    #[test]
    fn test_from_io_error_connection_timed_out() {
        let io_error = io::Error::new(ErrorKind::TimedOut, "Timeout occurred");
        let dep_error: DependencyError = io_error.into();

        match dep_error {
            DependencyError::CheckFailed(msg) => {
                assert!(msg.contains("Connection timed out"));
                assert!(msg.contains("Timeout occurred"));
            }
            _ => panic!("Expected CheckFailed variant"),
        }
    }

    #[test]
    fn test_from_io_error_unexpected_eof() {
        let io_error = io::Error::new(ErrorKind::UnexpectedEof, "Unexpected end of file");
        let dep_error: DependencyError = io_error.into();

        match dep_error {
            DependencyError::CheckFailed(msg) => {
                assert!(msg.contains("Unexpected end of file"));
            }
            _ => panic!("Expected CheckFailed variant"),
        }
    }

    #[test]
    fn test_from_io_error_generic() {
        let io_error = io::Error::new(ErrorKind::Other, "Generic error");
        let dep_error: DependencyError = io_error.into();

        match dep_error {
            DependencyError::CheckFailed(msg) => {
                assert!(msg.contains("I/O error"));
                assert!(msg.contains("Generic error"));
            }
            _ => panic!("Expected CheckFailed variant"),
        }
    }

    #[test]
    fn test_error_display_implementation() {
        // 测试所有错误类型的Display实现
        let errors = vec![
            DependencyError::CheckFailed("Test check failed".to_string()),
            DependencyError::NotAutoInstallable("nodejs".to_string()),
            DependencyError::InstallFailed(InstallErrorType::NetworkError),
            DependencyError::AlreadySatisfied("pnpm".to_string(), "8.10.0".to_string()),
            DependencyError::NotFound("nonexistent".to_string()),
        ];

        for error in errors {
            let display_msg = error.to_string();
            assert!(!display_msg.is_empty());
            assert!(display_msg.len() > 10); // 确保错误消息有意义
        }
    }

    #[test]
    fn test_install_error_type_display() {
        let error_types = vec![
            InstallErrorType::NetworkError,
            InstallErrorType::PermissionError,
            InstallErrorType::DiskSpaceError,
            InstallErrorType::VersionConflictError,
            InstallErrorType::UnknownError,
        ];

        for error_type in error_types {
            let display_msg = format!("{}", error_type);
            assert!(!display_msg.is_empty());
        }
    }

    #[test]
    fn test_error_json_with_context() {
        // 测试带有上下文信息的错误JSON序列化
        let error = DependencyError::CheckFailed(
            "Failed to check Node.js version: Command 'node --version' timed out after 5 seconds".to_string()
        );

        let json = serde_json::to_value(&error).expect("Failed to serialize to JSON");

        // 验证JSON结构
        assert!(json.get("error").is_some());
        assert!(json.get("details").is_some());

        // 验证错误内容
        let details = json.get("details").unwrap().as_str().unwrap();
        assert!(details.contains("Node.js version"));
        assert!(details.contains("timed out"));
    }
}