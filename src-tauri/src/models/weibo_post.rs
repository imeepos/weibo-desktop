//! 微博帖子模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 微博帖子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeiboPost {
    pub id: String,
    pub task_id: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub author_uid: String,
    pub author_screen_name: String,
    pub reposts_count: u64,
    pub comments_count: u64,
    pub attitudes_count: u64,
    pub crawled_at: DateTime<Utc>,
}

impl WeiboPost {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        task_id: String,
        text: String,
        created_at: DateTime<Utc>,
        author_uid: String,
        author_screen_name: String,
        reposts_count: u64,
        comments_count: u64,
        attitudes_count: u64,
    ) -> Self {
        Self {
            id,
            task_id,
            text,
            created_at,
            author_uid,
            author_screen_name,
            reposts_count,
            comments_count,
            attitudes_count,
            crawled_at: Utc::now(),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("帖子ID不能为空".to_string());
        }

        if self.author_uid.trim().is_empty() {
            return Err("作者UID不能为空".to_string());
        }

        if self.created_at > self.crawled_at {
            return Err("帖子发布时间不能晚于爬取时间".to_string());
        }

        Ok(())
    }
}
