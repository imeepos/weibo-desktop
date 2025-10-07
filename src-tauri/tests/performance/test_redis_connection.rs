//! Redis连接测试 - 用于验证测试环境

use weibo_login::services::RedisService;

#[tokio::test]
#[ignore]
async fn test_redis_connection() {
    // 尝试多个Redis URL
    let urls = vec![
        "redis://localhost:6379",
        "redis://127.0.0.1:6379",
        "redis://172.28.0.10:6379", // Docker network IP
    ];

    for redis_url in urls {
        println!("\n尝试连接到: {}", redis_url);

        match RedisService::new(redis_url) {
            Ok(redis) => {
                println!("  ✓ Redis连接创建成功");

                // 尝试获取连接
                match redis.get_connection().await {
                    Ok(mut conn) => {
                        println!("  ✓ Redis连接获取成功");

                        // 尝试执行ping命令
                        match redis::cmd("PING").query_async::<String>(&mut *conn).await {
                            Ok(response) => {
                                println!("  ✓ PING响应: {}", response);
                                assert_eq!(response, "PONG");
                                println!("\n成功! 使用URL: {}", redis_url);
                                return;
                            }
                            Err(e) => {
                                println!("  ✗ PING命令失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("  ✗ 获取连接失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  ✗ Redis连接创建失败: {}", e);
            }
        }
    }

    panic!("所有Redis URL都无法连接");
}
