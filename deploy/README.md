# Deployment

## Architecture

```
Internet → :443 TLS → Caddy (host network) → localhost:50051 h2c → server container → postgres container
```

- **Caddy**: TLS termination + reverse proxy, runs in host network mode
- **Server**: gRPC backend, port 50051 mapped to localhost only
- **PostgreSQL**: containerized, internal Docker network only

## Environments

| Environment | Domain | Compose file |
|---|---|---|
| Preview | `preview-agent.xclz.org` | `.env.preview` |
| Production | `agent.xclz.org` | `.env.prod` |

Both share port 443 — Caddy routes by SNI (domain name in TLS handshake).

## Commands

```bash
cd deploy/

# Preview
sg docker -c "docker compose --env-file .env.preview up -d"
sg docker -c "docker compose --env-file .env.preview logs -f"
sg docker -c "docker compose --env-file .env.preview down"

# Production
sg docker -c "docker compose --env-file .env.prod up -d"

# Rebuild after code changes
sg docker -c "docker compose --env-file .env.preview up -d --build"
```

Note: `sg docker -c "..."` is needed because the `openclaw` user accesses Docker via group membership.

## TLS Certificates

Managed automatically by Caddy via Let's Encrypt. Stored in Docker volume `caddy_data`.

## Docker Build

Build context is the repo root (not `server/`), because `proto/` is at repo root:

```bash
docker build -f server/Dockerfile .
```

## Configuration

All environment-specific config lives in `.env.*` files (gitignored). See `.env.example` for the full list of variables.
