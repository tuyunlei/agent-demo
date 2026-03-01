# agent-demo/mobile — ROADMAP

> Phase 0 architecture design completed. Entering Phase 1 Walking Skeleton implementation.
> Detailed design docs in `docs/design/`
>
> **MVP product scope**: Login + Chat two pages, AI response returned as whole block (unary, no streaming)

---

## Tech Selection

- **gRPC library**: grpc-swift v2 (grpc-swift-protobuf + grpc-swift-nio-transport) — native Swift Concurrency
- **Server address**: See `deploy/.env` (gRPC over TLS, Caddy reverse proxy)

---

## Phase 0: Architecture Design (Completed)

<details>
<summary>Expand to view</summary>

| # | Task | Status | Location |
|---|------|--------|----------|
| M01 | Architecture principles + layering spec | ✅ | `docs/design/principles.md` |
| M02 | Tech selection + constraints | ✅ | `docs/design/tech-stack.md` |
| M03 | Foundation layer design | ✅ | `docs/design/foundation.md` |
| M04 | Service layer design | ✅ | `docs/design/services.md` |
| M05 | Business module division | ✅ | `docs/design/business-modules.md` |
| M06 | App integration layer design | ✅ | `docs/design/app-integration.md` |
| M07 | Proto definitions | ✅ | `../proto/` |

</details>

---

## Phase 1: Walking Skeleton

### Step 3: iOS Project Kickoff (Completed)

| # | Task | Status | Content |
|---|------|--------|---------|
| TM3.1a | Xcode project initialization | ✅ | SwiftUI, iOS 26.2 |
| TM3.1b | grpc-swift v2 configuration | ✅ | SPM dependency + protoc .pb.swift + TLS channel + APIClient |
| TM3.2 | GitHub Actions iOS CI | ✅ | macOS runner, Xcode 26.2, xcbeautify + raw log artifact |
| TM3.3 | Login page | ✅ | Login UI + AuthService.Login call |
| TM3.4 | Chat page | ✅ | Chat UI + SendMessage call |

---

## Quality Gate: Quality Assurance System

> All checks run in CI (macOS runner). VPS has no Xcode, only file size checks can run locally.

### Code Quality (Automatic Gates)

| # | Task | Status | Content |
|---|------|--------|---------|
| MQG1 | SwiftLint + SwiftFormat | ✅ | SwiftLint --strict + SwiftFormat --lint, CI gate (PR #7) |
| MQG2 | File size & complexity | ✅ | Single file ≤300 lines, single function ≤50 lines; script + CI gate (PR #8) |

### Infrastructure

| # | Task | Status | Content |
|---|------|--------|---------|
| MQG-infra | SPM Build Plugin replaces manual protoc | ✅ | PR #9, grpc-swift v2 SPM plugin, zero-cost sync on proto changes |

### Functional Quality (Test Assurance)

| # | Task | Status | Content |
|---|------|--------|---------|
| MQG3 | ViewModel refactoring + unit tests | ✅ | ChatViewModel + ChatServiceProtocol + 5 Swift Testing unit tests |
| MQG4 | Test completion + Protocol-ization | ✅ | PR #1, SessionServiceProtocol + loadHistory 4 tests + AppState 3 tests |

### Quality Iron Laws

- CI red = cannot merge
- New feature PRs must include corresponding tests
- SwiftLint warnings treated as errors (--strict)

---

## Fixes and Completion

> After quality gates complete, fix existing functional issues.

| # | Task | Status | Content |
|---|------|--------|---------|
| FIX-1 | Chat page displays AI response | ✅ | Parse TextBlock in assistantContent |

---

## Phase 1 (Continued): Feature Development

> Resume feature development after quality gates + fixes complete.
> **Mobile pauses after completing below content, waits for TuTu acceptance, then advances in sync with server.**

### Step 4: Registration + Persistence

| # | Task | Status | Content |
|---|------|--------|---------|
| TM-REG | User registration | ✅ | PR #7, LoginView Sign In/Sign Up toggle + AuthServiceClient.register |
| TM-PERSIST | Token + Session persistence | ✅ | PR #9, Keychain store token + UserDefaults store sessionID |
| TM-HISTORY | Chat history loading | ✅ | PR #11, SessionServiceClient + ChatView load history on launch |
| TM4.1 | GRDB + SQLite | 🔲 | Local message cache, offline history viewing |

### Step 5: Resilience

| # | Task | Status | Content |
|---|------|--------|---------|
| TM5.1 | Network error handling | ✅ | PR #8, friendly error prompts + failure rollback + error banner + Sign Out |
| TM5.2 | Token auto-refresh | 🔲 | RefreshToken logic, automatic renewal on expiration |

### Step 6: Session Management

| # | Task | Status | Content |
|---|------|--------|---------|
| TM6.1 | Session list page | 🔲 | (Post-MVP, not implemented yet) |

---

## Test System Construction

> Leverage full Mac environment (local + CI macOS runner) to establish multi-layer test assurance.

### XCUITest (UI Automation)

| # | Task | Status | Content |
|---|------|--------|---------|
| TUI-1 | XCUITest infrastructure | 🔲 | Mock network layer + LaunchArgument injection + test helper |
| TUI-2 | Login flow test | 🔲 | Register → Login → Enter chat page; error prompt verification |
| TUI-3 | Chat flow test | 🔲 | Send message → Receive reply → History loading |

### Integration Tests

| # | Task | Status | Content |
|---|------|--------|---------|
| TINT-1 | Mock gRPC Server | 🔲 | Local lightweight gRPC server, verify complete network chain |
| TINT-2 | Core scenario integration verification | 🔲 | Register/Login/Chat/Token refresh end-to-end mock verification |

### E2E (Full Chain, Low Frequency)

| # | Task | Status | Content |
|---|------|--------|---------|
| TE2E-1 | Real server E2E | 🔲 | App → Real server → Mock LLM, manual or CI low-frequency trigger |

---

## Future Construction (To Be Discussed)

| # | Task | Status | Content |
|---|------|--------|---------|
| OBS-1 | Logging and observability system | 🔲 | Client log collection, error reporting, performance monitoring. To be detailed after discussion with TuTu |

---

*Last updated: 2026-02-24*
