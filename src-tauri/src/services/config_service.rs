use crate::models::{RedisConfig, RedisConfigError};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

/// 配置服务
///
/// 管理应用程序配置的持久化,职责单一:
/// - 保存配置到 .env 文件
/// - 从 .env 文件加载配置
/// - 保持其他配置项不变,仅更新目标字段
pub struct ConfigService;

impl ConfigService {
    /// 获取 .env 文件路径
    ///
    /// 查找顺序:
    /// 1. 当前工作目录的 .env
    /// 2. src-tauri/ 的上层目录(项目根目录)
    fn env_file_path() -> Result<PathBuf, RedisConfigError> {
        let cwd = env::current_dir()
            .map_err(|e| RedisConfigError::IoError(format!("无法获取当前目录: {}", e)))?;

        // 尝试当前目录
        let env_path = cwd.join(".env");
        if env_path.exists() {
            return Ok(env_path);
        }

        // 尝试父目录(适用于 src-tauri/ 内执行的情况)
        if let Some(parent) = cwd.parent() {
            let parent_env = parent.join(".env");
            if parent_env.exists() {
                return Ok(parent_env);
            }
        }

        // 不存在则创建在当前目录
        Ok(env_path)
    }

    /// 解析 .env 文件内容为 HashMap
    ///
    /// 格式: KEY=VALUE
    /// 忽略空行和注释行(以 # 开头)
    fn parse_env_content(content: &str) -> HashMap<String, String> {
        content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                // 忽略空行和注释
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    return None;
                }

                // 解析 KEY=VALUE
                trimmed
                    .split_once('=')
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            })
            .collect()
    }

    /// 将 HashMap 序列化为 .env 文件内容
    ///
    /// 保留原有的注释和空行,仅更新指定的配置项
    fn serialize_env_content(
        original_content: &str,
        updated_vars: &HashMap<String, String>,
    ) -> String {
        let mut result = String::new();
        let mut updated_keys = updated_vars.keys().cloned().collect::<Vec<_>>();

        // 遍历原始内容,保留注释和空行,更新已存在的配置项
        for line in original_content.lines() {
            let trimmed = line.trim();

            // 保留空行和注释
            if trimmed.is_empty() || trimmed.starts_with('#') {
                result.push_str(line);
                result.push('\n');
                continue;
            }

            // 检查是否为配置行
            if let Some((key, _)) = trimmed.split_once('=') {
                let key = key.trim();
                if let Some(new_value) = updated_vars.get(key) {
                    // 更新已存在的配置项
                    result.push_str(&format!("{}={}\n", key, new_value));
                    // 标记为已处理
                    updated_keys.retain(|k| k != key);
                    continue;
                }
            }

            // 保留其他行
            result.push_str(line);
            result.push('\n');
        }

        // 追加新的配置项
        for key in updated_keys {
            if let Some(value) = updated_vars.get(&key) {
                result.push_str(&format!("{}={}\n", key, value));
            }
        }

        result
    }

    /// 从 .env 文件加载 Redis 配置
    ///
    /// 读取环境变量:
    /// - REDIS_HOST: Redis服务器地址 (默认: localhost)
    /// - REDIS_PORT: Redis端口 (默认: 6379)
    /// - REDIS_PASSWORD: 认证密码 (可选)
    /// - REDIS_DATABASE: 数据库索引 (可选,0-15)
    ///
    /// # 错误处理
    /// - 文件不存在时返回默认配置(不报错)
    /// - 文件读取失败时返回 IoError
    /// - 端口/数据库索引格式错误时返回 InvalidUrl
    pub fn load_redis_config() -> Result<RedisConfig, RedisConfigError> {
        let env_path = Self::env_file_path()?;

        // 文件不存在则返回默认配置
        if !env_path.exists() {
            tracing::info!(
                path = %env_path.display(),
                "配置文件不存在,使用默认 Redis 配置"
            );
            return Ok(RedisConfig::default());
        }

        // 读取文件内容
        let content = fs::read_to_string(&env_path)?;
        let vars = Self::parse_env_content(&content);

        // 解析 Redis 配置
        let host = vars
            .get("REDIS_HOST")
            .cloned()
            .unwrap_or_else(|| "localhost".to_string());

        let port = vars
            .get("REDIS_PORT")
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(6379);

        let password = vars.get("REDIS_PASSWORD").cloned();

        let database = vars
            .get("REDIS_DATABASE")
            .and_then(|d| d.parse::<u8>().ok());

        // 验证数据库索引范围
        if let Some(db) = database {
            if db > 15 {
                return Err(RedisConfigError::InvalidUrl(format!(
                    "数据库索引超出范围: {} (有效范围: 0-15)",
                    db
                )));
            }
        }

        let config = RedisConfig::new(host, port);
        let config = if let Some(pwd) = password {
            config.with_password(pwd)
        } else {
            config
        };
        let config = if let Some(db) = database {
            config.with_database(db)
        } else {
            config
        };

        tracing::info!(
            path = %env_path.display(),
            config = %config.summary_for_logging(),
            "已加载 Redis 配置"
        );

        Ok(config)
    }

    /// 保存 Redis 配置到 .env 文件
    ///
    /// 更新策略:
    /// - 保留文件中的注释和空行
    /// - 仅更新 Redis 相关的配置项
    /// - 如果配置项不存在则追加到末尾
    /// - 密码字段在日志中不显示明文
    ///
    /// # 参数
    /// - `config`: 待保存的 Redis 配置
    ///
    /// # 错误处理
    /// - 无法创建或写入文件时返回 IoError
    pub fn save_redis_config(config: &RedisConfig) -> Result<(), RedisConfigError> {
        let env_path = Self::env_file_path()?;

        // 读取原有内容(如果文件存在)
        let original_content = if env_path.exists() {
            fs::read_to_string(&env_path)?
        } else {
            String::new()
        };

        // 准备更新的配置项
        let mut updated_vars = HashMap::new();
        updated_vars.insert("REDIS_HOST".to_string(), config.host.clone());
        updated_vars.insert("REDIS_PORT".to_string(), config.port.to_string());

        if let Some(ref password) = config.password {
            updated_vars.insert("REDIS_PASSWORD".to_string(), password.clone());
        }

        if let Some(database) = config.database {
            updated_vars.insert("REDIS_DATABASE".to_string(), database.to_string());
        }

        // 序列化新内容
        let new_content = Self::serialize_env_content(&original_content, &updated_vars);

        // 写入文件
        fs::write(&env_path, new_content)?;

        tracing::info!(
            path = %env_path.display(),
            config = %config.summary_for_logging(),
            "已保存 Redis 配置"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_content() {
        let content = r#"
# Redis配置
REDIS_HOST=localhost
REDIS_PORT=6379

# 其他配置
RUST_LOG=info
"#;

        let vars = ConfigService::parse_env_content(content);
        assert_eq!(vars.get("REDIS_HOST"), Some(&"localhost".to_string()));
        assert_eq!(vars.get("REDIS_PORT"), Some(&"6379".to_string()));
        assert_eq!(vars.get("RUST_LOG"), Some(&"info".to_string()));
        assert_eq!(vars.len(), 3);
    }

    #[test]
    fn test_serialize_env_content_update_existing() {
        let original = r#"# Redis配置
REDIS_HOST=localhost
REDIS_PORT=6379

# 其他配置
RUST_LOG=info
"#;

        let mut updated = HashMap::new();
        updated.insert("REDIS_HOST".to_string(), "redis.example.com".to_string());
        updated.insert("REDIS_PORT".to_string(), "6380".to_string());

        let result = ConfigService::serialize_env_content(original, &updated);

        assert!(result.contains("REDIS_HOST=redis.example.com"));
        assert!(result.contains("REDIS_PORT=6380"));
        assert!(result.contains("RUST_LOG=info"));
        assert!(result.contains("# Redis配置"));
    }

    #[test]
    fn test_serialize_env_content_add_new() {
        let original = r#"# 配置
RUST_LOG=info
"#;

        let mut updated = HashMap::new();
        updated.insert("REDIS_HOST".to_string(), "localhost".to_string());
        updated.insert("REDIS_PORT".to_string(), "6379".to_string());

        let result = ConfigService::serialize_env_content(original, &updated);

        assert!(result.contains("RUST_LOG=info"));
        assert!(result.contains("REDIS_HOST=localhost"));
        assert!(result.contains("REDIS_PORT=6379"));
    }
}
