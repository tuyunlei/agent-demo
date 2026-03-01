# agent-demo — AI Agent Platform

Multi-tenant AI agent platform (SaaS) for emotional companionship and life assistance for general users.

## Project Structure

```
agent-demo/
├── proto/                    # gRPC Proto definitions (4 files: auth, chat, common, session)
├── server/                   # Rust server (Cargo workspace, 8 crates)
│   ├── crates/               # Hexagonal architecture: types → domain → app → channel/llm/storage → server
│   ├── crates/agent-e2e/tests/arch.rs  # Rust architecture/file size gate tests
│   ├── migrations/           # In crates/agent-storage/migrations/
│   ├── tasks/                # Task queue
│   └── KNOWN_ISSUES.md       # Known architecture issues
├── mobile/                   # iOS client (Swift + SwiftUI)
│   ├── AgentDemo/            # Xcode project
└── deploy/                   # Deployment config (Caddyfile + .env)
```

## Core Architecture

- **Server**: Hexagonal architecture (Ports & Adapters), Domain defines Port traits, adapters implement, dependencies can only flow inward
- **Client**: Four-layer architecture (App Integration → Business → Service → Foundation)
- **Communication**: gRPC (tonic/grpc-swift v2), Proto package `ai.agent.platform.v1`
- **LLM**: provider-agnostic, currently using Kimi K2.5 (volcengine OpenAI-compatible API)
- **Database**: PostgreSQL 16, sqlx

## Git Branch Strategy

- `feature/*` → `develop` (PR + CI, merge commit, no squash) → `main` (requires manual confirmation)
- **No direct commits to develop or main**
- Create PR immediately after feature branch creation (triggers CI)
- Delete feature branch after PR merge

## Development Environment

```bash
git config core.hooksPath .githooks   # Enable pre-push hook (fmt + clippy + test)
```

Skip hook (emergency): `git push --no-verify`

## Quality Standards (Non-negotiable)

- **CI red = cannot merge**, no exceptions
- New features must include corresponding tests
- Code must comply with architecture constraints (dependencies can only flow inward)

## Proto Key Conventions

- oneof field in `ContentBlock` is called `kind` (not `block`)
- package: `ai.agent.platform.v1`
- Proto files cannot be modified arbitrarily, changes require synchronization between server and client

## Deployment

- Service address see `deploy/.env` (gitignored)
- Caddy reverse proxy gRPC (TLS)
- All credentials via environment variables, not in repository

⚠️ **Repository is public** — Writing IP addresses, passwords, API keys, internal domain names and other sensitive information is prohibited.

## Current Progress

See `STATE.md` and `ROADMAP.md` in each directory for detailed server and client progress.
