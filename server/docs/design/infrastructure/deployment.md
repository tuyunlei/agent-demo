# 部署设计（D-INFRA-03）

**状态**：Draft（用于 MVP 实施）  
**更新时间**：2026-02-26  
**适用范围**：agent-demo 单机 Docker Compose 部署

---

## 1. 概述

agent-demo 采用三环境模型：**开发（dev）→ 预览（preview）→ 生产（prod）**。开发环境为本机裸跑，预览和生产环境通过 Docker Compose 部署在同一台 VPS 上，网络和数据完全隔离。

### 1.1 设计目标

- **安全**：最小攻击面，仅反向代理暴露到公网
- **隔离**：预览和生产互不干扰，数据库独立
- **简单**：单机部署，无 K8s/Swarm 依赖
- **可复现**：从源码到运行只需 `docker compose up`
- **有品味**：multi-stage 镜像、healthcheck、auto migration

---

## 2. 三环境定义

### 2.1 开发环境（dev）

**运行方式**：VPS 本机裸跑

```
开发者 → cargo build/test → 本机 PG (127.0.0.1:5432)
```

- 不暴露任何端口到公网
- 直接使用 VPS 上已有的 PostgreSQL 实例（database: `agentdemo`）
- `cargo run` 启动时监听 `127.0.0.1:50051`，仅本机可达
- 测试：`cargo test --workspace --exclude agent-e2e`（单元 + 集成）
- e2e 测试需要 `DATABASE_URL` 环境变量

**关键约束**：
- 开发环境不对外暴露任何端口
- PG 绑定 127.0.0.1，不接受远程连接
- 敏感配置通过 `deploy/.env`（已 gitignore）或环境变量传入

### 2.2 预览环境（preview）

**运行方式**：Docker Compose

```
互联网 → VPS:8444 → Caddy (TLS) → agent-server:50051 → postgres
```

- 用途：开发完成后手机连上去验证功能
- 端口：8444（区别于生产的 8443）
- 域名：`preview-agent.xclz.org`（或端口区分）
- 日志级别：debug
- LLM 限额：低（省钱，可设 rate limit）
- 数据库：独立 database `agentdemo_preview`（同 PG 实例）
- TLS：Let's Encrypt 正式证书（Caddy 自动管理）

### 2.3 生产环境（prod）

**运行方式**：Docker Compose

```
互联网 → VPS:8443 → Caddy (TLS) → agent-server:50051 → postgres
```

- 端口：8443
- 域名：`agent.xclz.org`
- 日志级别：info
- 数据库：独立 database `agentdemo_prod`（同 PG 实例）
- TLS：Let's Encrypt 正式证书

---

## 3. 镜像构建

### 3.1 Multi-stage Dockerfile

```dockerfile
# ===== Stage 1: Build =====
FROM rust:1.85-bookworm AS builder

# 安装 protoc（proto 编译依赖）
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# 先拷贝依赖文件，利用 Docker layer cache
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/crates/ ./crates/
COPY proto/ /build/proto/

# 编译 release
RUN cargo build --release --bin agent-server

# ===== Stage 2: Runtime =====
FROM gcr.io/distroless/cc-debian12

COPY --from=builder /build/target/release/agent-server /agent-server

EXPOSE 50051

ENTRYPOINT ["/agent-server"]
```

**设计选择**：
- `distroless` 镜像：无 shell、无包管理器、攻击面极小（~30MB）
- proto 文件在编译阶段拷入，运行时不需要
- 依赖文件先拷贝利用 layer cache，源码变更不重新下载 crate

### 3.2 Dockerfile 位置

```
agent-demo/
  server/
    Dockerfile          # server 镜像定义
```

### 3.3 构建命令

```bash
# 从项目根目录构建（需要 proto/ 和 server/ 两个目录）
docker build -f server/Dockerfile -t agent-demo-server:latest .
```

---

## 4. Docker Compose 配置

### 4.1 文件结构

```
deploy/
  docker-compose.yml            # 基础服务定义
  docker-compose.preview.yml    # 预览环境覆盖
  docker-compose.prod.yml       # 生产环境覆盖
  .env.preview                  # 预览环境变量（gitignore）
  .env.prod                     # 生产环境变量（gitignore）
  Caddyfile.preview             # 预览 Caddy 配置
  Caddyfile.prod                # 生产 Caddy 配置
  .env                          # 开发环境变量（已有，gitignore）
```

### 4.2 基础 Compose（docker-compose.yml）

```yaml
services:
  postgres:
    image: postgres:16-alpine
    restart: unless-stopped
    volumes:
      - pgdata:/var/lib/postgresql/data
    environment:
      POSTGRES_USER: ${PG_USER:-agentdemo}
      POSTGRES_PASSWORD: ${PG_PASSWORD}
      POSTGRES_DB: ${PG_DATABASE}
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${PG_USER:-agentdemo}"]
      interval: 5s
      timeout: 3s
      retries: 5
    # 不暴露 ports → 仅内部网络可达

  server:
    build:
      context: ..
      dockerfile: server/Dockerfile
    restart: unless-stopped
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://${PG_USER:-agentdemo}:${PG_PASSWORD}@postgres:5432/${PG_DATABASE}
      JWT_SECRET: ${JWT_SECRET}
      LLM_API_KEY: ${LLM_API_KEY}
      LLM_BASE_URL: ${LLM_BASE_URL}
      LLM_MODEL: ${LLM_MODEL}
      RUST_LOG: ${RUST_LOG:-info}
    # 不暴露 ports → 仅 Caddy 可达

  caddy:
    image: caddy:2-alpine
    restart: unless-stopped
    depends_on:
      - server
    volumes:
      - ${CADDYFILE:-./Caddyfile.prod}:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    ports:
      - "${EXTERNAL_PORT:-8443}:${EXTERNAL_PORT:-8443}"

volumes:
  pgdata:
  caddy_data:
  caddy_config:
```

### 4.3 预览覆盖（docker-compose.preview.yml）

```yaml
services:
  server:
    environment:
      RUST_LOG: debug

  caddy:
    ports:
      - "8444:8444"
    volumes:
      - ./Caddyfile.preview:/etc/caddy/Caddyfile:ro
```

### 4.4 生产覆盖（docker-compose.prod.yml）

```yaml
services:
  server:
    environment:
      RUST_LOG: info

  caddy:
    ports:
      - "8443:8443"
    volumes:
      - ./Caddyfile.prod:/etc/caddy/Caddyfile:ro
```

### 4.5 启动命令

```bash
# 预览
cd deploy
docker compose -f docker-compose.yml -f docker-compose.preview.yml --env-file .env.preview up -d

# 生产
cd deploy
docker compose -f docker-compose.yml -f docker-compose.prod.yml --env-file .env.prod up -d

# 停止
docker compose -f docker-compose.yml -f docker-compose.preview.yml down
```

---

## 5. 网络架构

### 5.1 Docker 网络拓扑

```
互联网
  │
  ▼
VPS eth0 (公网 IP)
  │
  ├── :8443 ──► [prod 网络]
  │              ├── caddy-prod ──► server-prod:50051
  │              └── postgres-prod:5432
  │
  └── :8444 ──► [preview 网络]
                 ├── caddy-preview ──► server-preview:50051
                 └── postgres-preview:5432
```

### 5.2 隔离策略

- 预览和生产使用 **独立的 Compose project**（通过 `-p` 参数或不同目录）
- 每个 project 自动创建独立的 bridge 网络
- 容器间通过服务名 DNS 通信，不跨网络
- **只有 Caddy 暴露端口**，server 和 postgres 完全不可从外部访问

### 5.3 备选：共用 PG 实例

MVP 阶段可以让预览和生产共用 VPS 上已有的 PostgreSQL（非容器化），通过不同 database name 隔离：
- 预览：`agentdemo_preview`
- 生产：`agentdemo_prod`

这样不需要在 Docker 里跑 PG，减少资源占用。Compose 里去掉 postgres service，DATABASE_URL 指向 `host.docker.internal` 或 VPS 内网 IP。

---

## 6. TLS 配置

### 6.1 Caddy 自动 HTTPS

Caddy 内置 ACME 客户端，自动申请和续期 Let's Encrypt 证书。

**Caddyfile.prod**：
```
agent.xclz.org:8443 {
    reverse_proxy server:50051 {
        transport http {
            versions h2c
        }
    }
}
```

**Caddyfile.preview**：
```
preview-agent.xclz.org:8444 {
    reverse_proxy server:50051 {
        transport http {
            versions h2c
        }
    }
}
```

### 6.2 DNS 配置

在 Cloudflare 添加 A 记录：
- `agent.xclz.org` → VPS IP
- `preview-agent.xclz.org` → VPS IP

---

## 7. 数据库 Migration

### 7.1 策略：应用启动时自动执行

在 `agent-server` 启动代码中使用 `sqlx::migrate!()` 宏：

```rust
// server 启动时
let pool = PgPool::connect(&database_url).await?;
sqlx::migrate!("./migrations").run(&pool).await?;
```

**优势**：
- 部署时无需手动跑 migration
- 保证 schema 与代码版本一致
- 回滚靠部署回滚（重新部署旧版本）

**约束**：
- migration 必须向前兼容（不能破坏旧版本正在运行的查询）
- 大表变更需要评估锁时间

### 7.2 Migration 文件位置

```
server/migrations/
  20260217_001_create_users.sql
  20260217_002_create_sessions.sql
  ...
```

---

## 8. 健康检查

### 8.1 PostgreSQL

```yaml
healthcheck:
  test: ["CMD-SHELL", "pg_isready -U agentdemo"]
  interval: 5s
  timeout: 3s
  retries: 5
```

### 8.2 agent-server

MVP 阶段可用 TCP 检查：

```yaml
healthcheck:
  test: ["CMD-SHELL", "nc -z localhost 50051 || exit 1"]
  interval: 10s
  timeout: 3s
  retries: 3
```

后续可实现 gRPC Health Checking Protocol（`grpc.health.v1.Health`）。

### 8.3 启动顺序

Compose `depends_on` + `condition: service_healthy` 保证：
1. PostgreSQL 先启动并通过 healthcheck
2. agent-server 启动，执行 migration，开始监听
3. Caddy 启动，反代到 server

---

## 9. 环境变量

### 9.1 完整变量清单

| 变量 | 说明 | 示例 | 敏感 |
|------|------|------|------|
| `DATABASE_URL` | PG 连接串 | `postgres://user:pass@postgres:5432/db` | ✅ |
| `JWT_SECRET` | JWT 签名密钥 | 随机 32 字节 base64 | ✅ |
| `LLM_API_KEY` | LLM Provider API Key | `sk-...` | ✅ |
| `LLM_BASE_URL` | LLM API 地址 | `https://api.openai.com/v1` | ❌ |
| `LLM_MODEL` | 默认模型 | `gpt-4o` | ❌ |
| `RUST_LOG` | 日志级别 | `info` / `debug` | ❌ |
| `PG_USER` | PG 用户名 | `agentdemo` | ❌ |
| `PG_PASSWORD` | PG 密码 | 随机生成 | ✅ |
| `PG_DATABASE` | 数据库名 | `agentdemo_prod` | ❌ |
| `EXTERNAL_PORT` | 对外端口 | `8443` / `8444` | ❌ |

### 9.2 敏感变量管理

- 所有 `.env*` 文件加入 `.gitignore`
- 生产密钥在 VPS 上手动创建，不经过 git
- 禁止在代码、日志、commit message 中出现密钥值

---

## 10. 客户端环境切换

### 10.1 iOS 客户端

在 app 内提供环境选择（Debug 构建可见）：

```swift
enum ServerEnvironment {
    case preview  // preview-agent.xclz.org:8444
    case production  // agent.xclz.org:8443
}
```

- Debug 构建：Settings 页面可切换环境
- Release 构建：固定指向生产环境

### 10.2 gRPC 连接配置

客户端根据选择的环境构建 gRPC channel：
- host + port 从环境配置读取
- TLS 始终开启（两个环境都有证书）

---

## 11. 日常操作手册

### 11.1 部署新版本

```bash
cd ~/code/misc/agent-demo

# 1. 拉最新代码
git pull

# 2. 重新构建并启动（以生产为例）
cd deploy
docker compose -f docker-compose.yml -f docker-compose.prod.yml --env-file .env.prod up -d --build

# 镜像构建 + migration 自动执行 + 服务启动
```

### 11.2 查看日志

```bash
# 所有服务
docker compose logs -f

# 只看 server
docker compose logs -f server

# 最近 100 行
docker compose logs --tail 100 server
```

### 11.3 进入容器排查

```bash
# distroless 没有 shell，用 debug 变体临时排查
docker compose exec server /bin/sh  # 不可用（distroless）

# 查看 PG
docker compose exec postgres psql -U agentdemo -d agentdemo_prod
```

### 11.4 数据备份

```bash
docker compose exec postgres pg_dump -U agentdemo agentdemo_prod > backup.sql
```

---

## 12. 安全清单

- [ ] 仅 Caddy 端口暴露到公网（8443/8444）
- [ ] PostgreSQL 不暴露端口
- [ ] agent-server 不暴露端口
- [ ] TLS 证书自动管理（Let's Encrypt）
- [ ] 所有敏感配置走环境变量
- [ ] `.env*` 文件在 .gitignore 中
- [ ] distroless 运行时镜像（无 shell）
- [ ] gRPC 接口有认证（JWT）
- [ ] 生产数据库密码独立生成，与开发不同
- [ ] Docker 网络隔离（预览/生产各自 bridge）

---

## 13. 未来演进

### 13.1 短期（MVP 后）

- 添加 rate limiting（Caddy 层或 server 层）
- 添加 IP 白名单（MVP 阶段只允许特定设备）
- gRPC Health Checking Protocol 替代 TCP 检查
- 日志聚合（stdout → 文件轮转或外部服务）

### 13.2 中期

- CI/CD 自动构建镜像（GitHub Actions → push image → deploy）
- Container Registry（GitHub Packages 或 Docker Hub）
- 蓝绿部署或滚动更新
- 监控告警（Prometheus + Grafana 或轻量级方案）

### 13.3 长期

- 多节点部署时迁移到 Kubernetes 或 Fly.io
- 数据库迁移到托管服务（RDS 等）
- CDN + 边缘缓存（静态资源）

---

## 附录 A：目录结构总览

```
agent-demo/
  proto/                    # protobuf 定义
  server/
    Dockerfile              # multi-stage 构建
    Cargo.toml
    migrations/             # sqlx migration
    crates/                 # 13 crate workspace
  mobile/                   # iOS 客户端
  deploy/
    docker-compose.yml      # 基础 Compose
    docker-compose.preview.yml
    docker-compose.prod.yml
    Caddyfile.preview
    Caddyfile.prod
    .env                    # 开发变量（gitignore）
    .env.preview            # 预览变量（gitignore）
    .env.prod               # 生产变量（gitignore）
  .gitignore                # 包含 deploy/.env*
```

## 附录 B：与现有部署的差异

| | 之前（已关闭） | 新方案 |
|---|---|---|
| 运行方式 | systemd user service | Docker Compose |
| PG | VPS 本机 | 容器化（或复用本机） |
| TLS | Caddy systemd | Caddy 容器 |
| 网络隔离 | 无（全在 host） | Docker bridge 隔离 |
| 环境数 | 1（开发=生产） | 3（dev/preview/prod） |
| 镜像大小 | N/A（直接跑 binary） | ~30MB（distroless） |
| 部署命令 | `systemctl restart` | `docker compose up -d --build` |
