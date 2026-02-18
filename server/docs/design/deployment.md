# 部署方案设计（MVP→可演进）

> 目标系统：多用户托管 AI Agent 平台（SaaS）  
> 技术约束：Rust 单二进制 + PostgreSQL、MVP 单实例、Native gRPC、需访问 OpenAI/Anthropic 等外部 API  
> 团队/预算：一人团队，预算约 **$100/月**，强调低复杂度与高性价比。

---

## 1. 设计目标与原则

### 1.1 核心目标

1. **先活下来（MVP）**：用最少组件跑通生产可用链路。
2. **可维护**：一人团队可管理、可排障、可恢复。
3. **可扩展**：不推翻重来，后续可平滑走向多实例。
4. **可控成本**：总成本稳定在 **$100/月左右**（不含 LLM Token 消耗）。

### 1.2 设计原则

- **单机优先、分层清晰**：将“业务容器 / 数据 / 监控 / 备份”逻辑分离。
- **容器化但不过度编排**：MVP 用 Docker Compose，不引入 K8s 运维负担。
- **默认最小暴露面**：仅暴露必要端口（443/80）；数据库内网访问。
- **自动化优先**：构建、部署、证书更新、备份全部自动化。
- **可观测先行**：至少具备基础指标、日志、健康检查与告警。

---

## 2. MVP 单机部署拓扑（清晰版）

## 2.1 逻辑拓扑图（Mermaid）

```mermaid
flowchart TB
    U[Client / SDK] -->|gRPC over TLS:443| RP[Reverse Proxy\nCaddy/Nginx]
    RP --> APP[Rust Agent Platform\nSingle Binary Container]

    APP -->|SQL 5432 (private network)| PG[(PostgreSQL)]
    APP -->|HTTPS egress| LLM[OpenAI / Anthropic / Others]

    APP --> METRICS[/metrics]
    METRICS --> PROM[Prometheus]
    PROM --> GRAF[Grafana]

    subgraph VPS[Single VPS]
      RP
      APP
      PG
      PROM
      GRAF
      BK[Backup Job\n(pg_dump + WAL/archive optional)]
    end

    BK --> OBJ[(Object Storage / Backup Bucket)]
```

## 2.2 网络与端口建议

- 公网开放：
  - `80/tcp`（仅 ACME 或 80→443 跳转）
  - `443/tcp`（gRPC TLS 入口）
- 内网（Docker network）:
  - App: `8080`（容器内）
  - PostgreSQL: `5432`（不对公网）
  - Prometheus: `9090`（仅内网/受限）
  - Grafana: `3000`（建议只内网，必要时走反向代理 + Basic Auth/OAuth）

---

## 3. 基础设施选型（VPS/容器/云服务）

## 3.1 推荐基线（生产 MVP）

- **1 台 VPS**（Linux，2~4 vCPU / 8GB RAM / 120~160GB NVMe SSD）
- **Docker + Docker Compose** 部署全部服务
- **对象存储**（S3 兼容）用于异地备份
- **域名 + TLS 自动证书**（Caddy 或 Nginx+certbot）

### 选型理由

- 单机成本最低，结构最简单，适合一人维护。
- Compose 足够支撑 MVP，且未来迁移到 Swarm/K8s 成本较低。
- 数据与备份解耦，降低 VPS 故障导致数据不可恢复风险。

## 3.2 不推荐（MVP 阶段）

- 立即上 K8s：运维复杂度高，超出一人团队短期收益。
- 立即上托管 PG + 多机：成本和运维面扩大，不利于 $100 预算控制。

---

## 4. 容器化策略（Dockerfile + Compose）

## 4.1 Dockerfile 策略（Rust 单二进制）

采用**多阶段构建**：

1. `builder` 阶段：Rust toolchain 编译 release 二进制
2. `runtime` 阶段：最小化运行时镜像（如 debian-slim/distroless）
3. 仅复制可执行文件与必要 CA 证书
4. 非 root 用户运行

关键点：

- 启用 `HEALTHCHECK`（如 `/healthz`）
- 注入构建信息（git sha, build time）
- 默认只读文件系统（可选）+ 挂载日志目录
- 环境变量管理配置，不把密钥 baked 进镜像

## 4.2 docker-compose 结构建议

建议服务：

- `app`：Rust 平台主服务
- `db`：PostgreSQL
- `proxy`：Caddy/Nginx（TLS + gRPC 转发）
- `prometheus`
- `grafana`
- `backup`（可用 cron 容器或宿主机 systemd timer）

网络与卷：

- `frontend_net`：proxy 对外
- `backend_net`：app/db/monitoring 内网通信
- 数据卷：
  - `pg_data`
  - `prom_data`
  - `grafana_data`
  - `backup_data`（临时备份落盘）

## 4.3 发布策略

- 镜像版本采用 `semver + git sha` 双标签
- `latest` 仅用于开发，不用于生产回滚
- 生产部署固定版本号，支持一键回滚至上个 tag

---

## 5. PostgreSQL 部署方案（同机 vs 托管）

## 5.1 MVP 推荐：同机 PostgreSQL（容器）

原因：

- 成本最低（无需额外托管 DB 费用）
- 延迟最小（本机网络）
- 运维链路短，故障定位简单

配置建议：

- PostgreSQL 版本：16（或当前稳定 LTS）
- `max_connections` 按单实例控制（如 100~200）
- 开启 `shared_buffers`（约内存 20~25%）
- 开启慢查询日志（阈值如 200ms）
- 仅监听内网网络，不暴露公网

## 5.2 何时迁移托管 PostgreSQL

当出现以下任意条件：

- RPO/RTO 要求变高（例如 RPO < 15 分钟）
- 单机 I/O 成为瓶颈
- 数据规模增长明显（如 >100GB 且持续增长）
- 运维精力不足以管理备份恢复演练

迁移方式：先做逻辑复制/定时全量+增量同步，切换窗口只读后切主。

---

## 6. 域名与 TLS 方案

## 6.1 推荐

- 域名：`api.yourdomain.com`
- TLS：Let’s Encrypt 自动签发
- 反向代理：
  - **优先 Caddy**（自动证书、配置简单）
  - 或 Nginx + certbot（更灵活，但运维略重）

## 6.2 gRPC 注意项

- 需确保代理支持 HTTP/2 + gRPC 转发
- 开启 TLS 强制，禁用明文外网 gRPC
- 建议开启请求体大小限制、连接数限制、速率限制

## 6.3 证书管理

- 自动续期（默认 60~90 天轮换）
- 续期失败告警（邮件/IM）
- 证书状态纳入监控面板

---

## 7. CI/CD 设计（GitHub Actions）

> 不写完整脚本，仅给可执行的流程设计。

## 7.1 分支与环境映射

- `main` → `production`
- `develop` → `staging`
- `feature/*` → `dev`（本地或临时环境）

## 7.2 Pipeline 分层

### A. CI（PR / Push）

1. 代码检查（fmt/lint）
2. 单元测试 + 基础集成测试
3. 构建 release 二进制
4. 构建 Docker 镜像
5. 镜像扫描（基础漏洞）
6. 推送镜像到 registry（按分支策略）

### B. CD（环境部署）

- Staging：自动部署（push develop）
- Production：手动审批后部署（push main 或 release tag）

部署动作建议：

1. 拉取目标版本镜像
2. `docker compose up -d` 滚动更新 app
3. 健康检查（/healthz + gRPC probe）
4. 失败自动回滚至前一版本

## 7.3 Secrets 管理

- GitHub Actions Secrets 存储部署密钥、镜像仓库凭据
- VPS 上使用 `.env` + 最小权限文件（`chmod 600`）
- API Key（OpenAI/Anthropic）只在服务器注入，不在 CI 日志输出

---

## 8. 备份与恢复（必须可演练）

## 8.1 备份策略（建议）

- **每日全量逻辑备份**：`pg_dump -Fc`
- **每 6 小时增量思路**（可选）：WAL 归档到对象存储
- **保留策略**：
  - 日备份保留 14 天
  - 周备份保留 8 周
  - 月备份保留 6 个月

## 8.2 存储策略

- 本地临时目录 + 上传对象存储（异地）
- 启用服务端加密（SSE）
- 备份文件命名含时间戳与校验信息

## 8.3 恢复策略（RTO/RPO）

MVP 目标：

- **RPO**：24 小时（仅日备）~ 6 小时（加 WAL）
- **RTO**：1~2 小时（单机恢复）

演练要求：

- 每月至少一次“从备份恢复到新库”演练
- 记录恢复耗时、失败点、操作手册

---

## 9. 监控与可观测性部署（Prometheus + Grafana）

## 9.1 指标体系（最小可用）

### 应用指标（Rust 暴露 /metrics）

- 请求总数、错误率、延迟分位（P50/P95/P99）
- 活跃租户数、队列长度、任务执行耗时
- 外部 LLM 调用成功率/失败率/延迟

### 系统指标

- 节点 CPU、内存、磁盘、负载（node exporter）
- 容器资源使用（cadvisor，可选）

### 数据库指标

- 连接数、TPS、慢查询、缓存命中率
- 表膨胀/锁等待（后续增强）

## 9.2 告警最小集

- 服务不可用（health probe fail）
- 5xx 错误率突增
- P95 延迟超阈值
- 磁盘使用率 > 80%
- PostgreSQL 不可连接
- 备份任务失败 / 超过 24h 无新备份
- TLS 证书临近过期

## 9.3 日志建议

- 容器 stdout/stderr 结构化 JSON 日志
- 保留 7~14 天滚动日志
- 关键操作（租户、账单、权限变更）需审计日志字段

---

## 10. 环境管理（dev / staging / production）

## 10.1 环境定义

- **dev**：本地 Compose，开发联调
- **staging**：与生产近似配置，用于回归和预发布验证
- **production**：真实租户流量

## 10.2 配置分离

- 使用 `.env.dev / .env.staging / .env.prod`
- 严格禁止跨环境共用数据库
- 使用不同 API Key 与不同回调域名

## 10.3 数据策略

- staging 使用脱敏数据或合成数据
- 禁止直接把生产快照无脱敏恢复到 staging

---

## 11. 成本估算（$100/月内）

> 以下为“数量级”估算，具体因区域与厂商波动。

## 11.1 MVP 方案（月度）

1. **VPS（2~4 vCPU, 8GB RAM, 160GB NVMe）**：$35 ~ $60
2. **对象存储（备份 100~300GB）**：$5 ~ $15
3. **监控额外成本**：$0（同机部署）
4. **域名摊销**：$1 ~ $2
5. **流量与杂费预留**：$10 ~ $20

**合计**：约 **$51 ~ $97 / 月**（不含 LLM token）

## 11.2 成本控制建议

- 监控与业务同机，先不拆监控节点
- 日志保留期控制在 7~14 天
- 备份按分层保留，避免无上限累积
- 夜间低峰可降频某些非关键任务

---

## 12. 演进路径（单机 → 编排 → 多区域）

## Phase 0（现在，0~3 个月）

- 单 VPS + Compose + 本机 PostgreSQL + 基础监控备份
- 达成稳定上线与付费验证

## Phase 1（3~6 个月）

- 增加一台“备机/预发布机”
- PostgreSQL 考虑主从或托管迁移评估
- 将备份恢复流程标准化（runbook）

## Phase 2（6~12 个月）

- 拆分服务：App 与 DB 分机
- 引入托管 PostgreSQL（如预算允许）
- 部署层从 Compose 过渡到轻量编排（Swarm/Nomad/K8s 任选其一）

## Phase 3（12 个月+）

- 多可用区/多区域部署
- 全局流量调度（GeoDNS/Anycast/边缘入口）
- 数据层跨区容灾（逻辑复制/多活策略按业务一致性要求设计）

---

## 13. 风险清单与规避

1. **单点故障（VPS 整机故障）**  
   - 规避：异地备份 + 快速重建脚本 + DNS 快速切换预案

2. **数据库磁盘打满**  
   - 规避：磁盘告警 + 日志/备份生命周期 + 定期 VACUUM 策略

3. **证书过期导致服务不可用**  
   - 规避：自动续期 + 到期告警 + 手工兜底证书流程

4. **CI/CD 误发布**  
   - 规避：生产手动审批 + 健康检查 + 自动回滚

5. **LLM 外部依赖波动**  
   - 规避：重试/熔断/限流 + 多 provider 抽象（已有 ADR trait 边界）

---

## 14. 推荐的最终落地组合（结论）

在当前约束下，推荐以下 MVP 生产组合：

- **基础设施**：1 台 8GB 内存 VPS
- **部署方式**：Docker Compose（app + db + proxy + prometheus + grafana + backup）
- **数据库**：同机 PostgreSQL 16（内网访问）
- **入口安全**：Caddy 自动 TLS，gRPC over HTTPS
- **CI/CD**：GitHub Actions（CI 自动，Prod 手动审批部署）
- **备份**：每日 pg_dump + 对象存储异地保留 + 月度恢复演练
- **预算**：控制在 **$60~$95/月**（不含模型调用费用）

该方案满足：

- ✅ 拓扑清晰
- ✅ 选型有理由
- ✅ 容器化完整
- ✅ 数据库与备份可执行
- ✅ CI/CD 流水线可落地
- ✅ 成本在预算内
- ✅ 演进路径明确

---

## 附录 A：Compose 目录结构建议（示意）

```text
infra/
  compose/
    docker-compose.yml
    docker-compose.prod.yml
  caddy/
    Caddyfile
  prometheus/
    prometheus.yml
  grafana/
    provisioning/
  backup/
    backup.sh
    restore.md
app/
  Dockerfile
  .env.example
ops/
  runbooks/
    incident.md
    backup-restore.md
    deploy-rollback.md
```

## 附录 B：生产上线前检查清单（简版）

- [ ] 443 TLS + gRPC 连通性验证通过
- [ ] 数据库不暴露公网
- [ ] 健康检查与告警规则生效
- [ ] 备份任务连续 3 天成功
- [ ] 完成至少 1 次恢复演练
- [ ] 部署支持回滚并已演练
- [ ] 成本面板与阈值告警已配置
