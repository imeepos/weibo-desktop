//! 测试PostgreSQL数据库连接和初始化

use weibo_login::database::{init_database, get_database};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化数据库
    println!("正在初始化PostgreSQL数据库...");
    init_database().await?;

    // 健康检查
    println!("执行数据库健康检查...");
    get_database().health_check().await?;

    println!("PostgreSQL数据库连接和初始化成功！");
    Ok(())
}