# Deployment

## Architecture

```
server container (gRPC h2c) ← postgres container
       ↑
  localhost:${SERVER_PORT}
       ↑
your reverse proxy (TLS termination)
       ↑
    Internet
```

The server exposes plain gRPC (h2c) on a localhost port. TLS termination is handled externally by whatever reverse proxy you prefer (Caddy, Nginx, Traefik, etc.).

## Quick Start

```bash
cd deploy/
cp .env.example .env.prod
# Edit .env.prod with your values

sg docker -c "docker compose -p agent-prod --env-file .env.prod up -d"
sg docker -c "docker compose -p agent-prod --env-file .env.prod logs -f"
sg docker -c "docker compose -p agent-prod --env-file .env.prod down"

# Rebuild after code changes
sg docker -c "docker compose -p agent-prod --env-file .env.prod up -d --build"
```

Note: `sg docker -c "..."` may be needed depending on your Docker group setup.

## Multiple Environments

Run multiple environments on the same host by using different project names and ports:

```bash
# Preview on port 50051
docker compose -p agent-preview --env-file .env.preview up -d

# Production on port 50052
docker compose -p agent-prod --env-file .env.prod up -d
```

Each project gets its own isolated postgres and data volume.

## Reverse Proxy Example (Caddy)

```
agent.example.com {
    reverse_proxy localhost:50052 {
        transport http {
            versions h2c
        }
    }
}
```

## Docker Build

Build context is the repo root (not `server/`), because `proto/` lives at repo root:

```bash
docker build -f server/Dockerfile .
```

## Configuration

All environment-specific config lives in `.env.*` files (gitignored). See `.env.example` for the full list of variables.
