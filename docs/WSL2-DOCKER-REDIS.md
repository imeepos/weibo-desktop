# WSL2 + Docker Desktop Redis 连接问题解决方案

## 问题描述

在 WSL2 环境中使用 Docker Desktop 时,Tauri 应用无法通过 `localhost:6379` 连接到 Docker 容器中的 Redis。

## 根本原因

1. **WSL2 网络架构**: Docker Desktop 运行在 Windows 主机上,端口映射到 Windows,而不是 WSL2
2. **网络隔离**: WSL2 和 Docker 容器运行在不同的网络命名空间
3. **端口转发失效**: 虽然 Docker 配置了 `0.0.0.0:6379->6379` 映射,但 WSL2 无法直接访问

## 解决方案

### 方案 A: 在 Docker 容器中运行 Tauri (推荐)

使用项目提供的 `tauri-dev` Docker 服务,确保所有组件在同一网络:

```bash
# 启动 Redis 和 Tauri 开发环境
docker compose --profile dev up redis tauri-dev

# 或启动所有服务
docker compose --profile dev up
```

**优势:**
- ✓ 网络配置自动化 (使用 Docker 服务名和内部 IP)
- ✓ 环境一致性 (开发/生产环境统一)
- ✓ 依赖隔离 (不污染宿主机)

### 方案 B: 使用 Docker 内部 IP (已配置)

如果必须在 WSL2 本地运行 Tauri,需要:

1. **确认 Redis 容器 IP**:
   ```bash
   docker inspect desktop-redis -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}'
   # 输出: 172.28.0.10
   ```

2. **修改 `.env` 文件** (已完成):
   ```bash
   REDIS_URL=redis://172.28.0.10:6379
   ```

3. **限制**:
   - ✗ WSL2 通常无法直接访问 Docker 自定义网络 (172.28.0.0/16)
   - ✗ 需要额外的路由配置或网桥设置

### 方案 C: 使用 host 网络模式 (不推荐)

修改 `docker-compose.yml` 的 Redis 服务:

```yaml
redis:
  network_mode: "host"  # 直接使用宿主机网络
  # 移除 networks 和 ports 配置
```

**缺点:**
- ✗ 破坏网络隔离
- ✗ 端口冲突风险
- ✗ 不符合容器化最佳实践

## 当前配置状态

### Docker Compose 配置

```yaml
# docker-compose.yml
redis:
  container_name: desktop-redis
  ports:
    - "6379:6379"
  networks:
    app-network:
      ipv4_address: 172.28.0.10

tauri-dev:
  environment:
    - REDIS_URL=redis://172.28.0.10:6379  # 使用容器内部 IP
  networks:
    - app-network
```

### 环境变量配置

```bash
# .env (已更新)
REDIS_URL=redis://172.28.0.10:6379
```

## 验证连接

### 1. 检查 Redis 容器状态

```bash
docker ps --filter "name=desktop-redis"
# 确认状态为 "healthy"
```

### 2. 从容器内部测试

```bash
docker exec desktop-redis redis-cli ping
# 应输出: PONG
```

### 3. 从 WSL2 测试 (预期失败)

```bash
# 方法 1: 使用 telnet
telnet 172.28.0.10 6379

# 方法 2: 使用 bash
timeout 2 bash -c "echo > /dev/tcp/172.28.0.10/6379"
```

**预期结果**: `Connection refused` (正常,因为 WSL2 无法直接访问 Docker 自定义网络)

### 4. 使用 Docker Compose 开发环境测试

```bash
# 启动 Tauri 开发容器
docker compose --profile dev up tauri-dev -d

# 查看日志验证 Redis 连接
docker compose logs tauri-dev | grep -i redis
```

## 故障排查

### 错误: "Connection refused"

**原因**: Tauri 在 WSL2 本地运行,无法访问 Docker 内部网络

**解决**: 使用方案 A (Docker 容器运行 Tauri)

### 错误: "No route to host"

**原因**: 网络路由配置问题

**解决**:
```bash
# 检查 Docker 网络
docker network inspect desktop_app-network

# 确认子网配置
# "Subnet": "172.28.0.0/16"
```

### 依赖检测显示 "缺少 Redis"

**原因**: `DependencyChecker` 使用 `localhost:6379` 检测

**临时解决**: 修改 `.env` 后重启应用

## 最佳实践

1. **开发环境**: 使用 `docker compose --profile dev up`
2. **测试环境**: 所有服务容器化
3. **生产环境**: Kubernetes/Docker Swarm 网络自动解析

## 相关文件

- `docker-compose.yml`: 服务编排配置
- `.env`: 环境变量 (已配置 Docker 内部 IP)
- `.env.example`: 配置模板 (包含两种模式注释)
- `src-tauri/src/main.rs:19-20`: Redis URL 读取逻辑
- `src-tauri/src/services/dependency_checker.rs:331-357`: 服务连接检测

## 后续改进

1. [ ] 添加自动检测运行环境 (容器内 vs WSL2 宿主机)
2. [ ] 根据环境自动选择 Redis URL
3. [ ] 提供健康检查端点暴露到 WSL2
4. [ ] 文档化端口转发配置 (Windows Firewall/WSL2 设置)
