# Docker 中间件配置说明

本目录包含微博扫码登录桌面应用的所有Docker中间件配置和初始化脚本。

## 📁 目录结构

```
docker/
├── postgres/
│   └── init/
│       └── 01-init-database.sql    # PostgreSQL 数据库初始化脚本
├── mongodb/
│   └── init/
│       └── 01-init-database.js     # MongoDB 数据库初始化脚本
├── rabbitmq/
│   ├── rabbitmq.conf               # RabbitMQ 配置文件
│   └── definitions.json            # RabbitMQ 队列和交换机定义
└── README.md                        # 本文件
```

## 🚀 快速开始

### 1. 启动所有基础服务

```bash
docker compose up redis postgres mongodb rabbitmq -d
```

### 2. 启动特定服务

```bash
# 只启动 Redis
docker compose up redis -d

# 启动 Redis + PostgreSQL
docker compose up redis postgres -d
```

### 3. 启动开发环境 (包含Tauri)

```bash
docker compose --profile dev up tauri-dev
```

### 4. 运行E2E测试

```bash
docker compose --profile test up playwright-tests
```

### 5. 启动MongoDB管理界面 (调试用)

```bash
docker compose --profile debug up mongo-express
```

访问: http://localhost:8081 (用户名: admin, 密码: admin123)

## 📊 服务端口映射

| 服务 | 端口 | 说明 |
|------|------|------|
| Redis | 6379 | 缓存和会话存储 |
| PostgreSQL | 5432 | 关系型数据库 |
| MongoDB | 27017 | 文档数据库 |
| RabbitMQ | 5672 | AMQP协议端口 |
| RabbitMQ Management | 15672 | 管理界面 (http://localhost:15672) |
| Mongo Express | 8081 | MongoDB Web UI (需启用debug profile) |
| Tauri Dev | 1420 | 开发服务器 (需启用dev profile) |

## 🔐 默认凭据

### PostgreSQL
- 数据库: `weibo_desktop`
- 用户名: `desktop_user`
- 密码: `desktop_pass`

### MongoDB
- 数据库: `weibo_desktop`
- Root用户名: `root`
- Root密码: `root_pass`

### RabbitMQ
- Virtual Host: `/weibo_desktop`
- 用户名: `desktop_user`
- 密码: `desktop_pass`
- 管理界面: http://localhost:15672

## 🔧 数据初始化

### PostgreSQL 初始化

初始化脚本位于 `postgres/init/01-init-database.sql`,会自动创建:
- 基础表结构 (accounts, login_history, cookies_backup)
- 索引和触发器
- 必要的PostgreSQL扩展

### MongoDB 初始化

初始化脚本位于 `mongodb/init/01-init-database.js`,会自动创建:
- 集合 (activity_logs, qrcode_records, system_events)
- 索引 (包括TTL索引自动清理过期数据)
- 数据验证规则

### RabbitMQ 初始化

配置文件 `rabbitmq/definitions.json` 会自动创建:
- 交换机: `weibo.events` (topic), `weibo.direct` (direct)
- 队列: `login_events`, `qrcode_events`, `cookies_validation`, `system_notifications`
- 绑定关系

## 📝 常用命令

### 查看日志

```bash
# 查看所有服务日志
docker compose logs -f

# 查看特定服务日志
docker compose logs -f redis
docker compose logs -f postgres
docker compose logs -f mongodb
docker compose logs -f rabbitmq
```

### 停止服务

```bash
# 停止所有服务
docker compose down

# 停止并删除数据卷 (⚠️ 会丢失所有数据)
docker compose down -v
```

### 重启服务

```bash
# 重启所有服务
docker compose restart

# 重启特定服务
docker compose restart redis
```

### 进入容器

```bash
# PostgreSQL
docker compose exec postgres psql -U desktop_user -d weibo_desktop

# MongoDB
docker compose exec mongodb mongosh -u root -p root_pass --authenticationDatabase admin

# Redis
docker compose exec redis redis-cli

# RabbitMQ
docker compose exec rabbitmq rabbitmqctl status
```

## 🔍 健康检查

所有服务都配置了健康检查,可以查看服务状态:

```bash
docker compose ps
```

## 🌐 网络配置

所有容器都在 `app-network` 网络中,使用固定IP:
- Redis: 172.28.0.10
- PostgreSQL: 172.28.0.11
- MongoDB: 172.28.0.12
- RabbitMQ: 172.28.0.13

## 💾 数据持久化

数据存储在Docker volumes中:
- `redis-data`: Redis 数据
- `postgres-data`: PostgreSQL 数据
- `mongodb-data`: MongoDB 数据
- `rabbitmq-data`: RabbitMQ 数据

## 🐛 故障排查

### 服务无法启动

```bash
# 查看详细日志
docker compose logs [service_name]

# 检查端口是否被占用
netstat -tulpn | grep [port]

# 删除并重建
docker compose down -v
docker compose up -d
```

### 数据初始化失败

```bash
# 删除volumes重新初始化
docker compose down -v
docker compose up -d
```

### WSL2 网络问题

如果在WSL2中无法从宿主机访问容器端口,使用容器内部IP (172.28.0.x) 访问。

## 📚 参考文档

- [Docker Compose 文档](https://docs.docker.com/compose/)
- [PostgreSQL 官方文档](https://www.postgresql.org/docs/)
- [MongoDB 官方文档](https://www.mongodb.com/docs/)
- [RabbitMQ 官方文档](https://www.rabbitmq.com/documentation.html)
- [Redis 官方文档](https://redis.io/documentation)
