# CLAUDE.md — iOS Client Development Guide

## Project Overview

AI companion Agent platform iOS client. Swift + SwiftUI, gRPC communication.

- **Code**: Current directory (`mobile/`)
- **Server**: `../server/` (Rust, deployment address see `deploy/.env`)
- **Proto**: `../proto/` (SPM Build Plugin auto-generates Swift code)
- **Design docs**: `docs/design/`
- **CI**: GitHub Actions macOS runner

## Workflow

1. Read `tasks/QUEUE.md`, get first task name
2. Enter `tasks/<task-name>/`, read `brief.md` (task goal + acceptance criteria)
3. If `feedback.md` exists, read it first — that's CI/review issues from last time
4. Create `feature/*` branch
5. Code + local testing
6. commit + push + open PR:
   ```bash
   git push origin feature/<branch-name>
   gh pr create --base develop --fill
   ```
7. **Do not modify any files under `tasks/`** — tasks are managed by PM

## After Task Completion

After PR submission, PM will handle review + CI + merge + archive. You don't need to care about follow-up.

**When TuTu asks you to continue to next task:**

1. `git checkout develop && git pull origin develop`
2. If old feature branch still exists locally, can delete (remote branch already deleted by PM)
3. Restart workflow step 1 (read `tasks/` → pick next → open new branch)

**How to know last task completed**: If that directory disappeared from `tasks/` (archived to `tasks/done/`), it's merged.

## Branch Rules

- Develop on `feature/*` branches, direct commits to develop prohibited
- PR target branch: develop
- One task, one branch, one PR

## Local Build and Test

**Don't use `swift build` / `swift test`** — protoc plugin needs `-skipPackagePluginValidation`, only xcodebuild supports this.

```bash
# Build (first check available simulators: xcrun simctl list devices available | grep iPhone)
xcodebuild build \
  -project AgentDemo.xcodeproj -scheme AgentDemo \
  -destination 'platform=iOS Simulator,name=<simulator-name>' \
  -configuration Debug CODE_SIGNING_ALLOWED=NO -skipPackagePluginValidation

# Test
xcodebuild test \
  -project AgentDemo.xcodeproj -scheme AgentDemo \
  -destination 'platform=iOS Simulator,name=<simulator-name>' \
  -configuration Debug -skip-testing:AgentDemoUITests \
  CODE_SIGNING_ALLOWED=NO -skipPackagePluginValidation
```

Simulator names vary by machine (CI uses `iPhone 16 Pro`, local may differ). Specific names can be written in `CLAUDE.local.md`, not in repository.

## Quality Requirements

- **SwiftLint**: `--strict` mode, warnings treated as errors
- **SwiftFormat**: `--lint` mode
- **Tests**: New features must have corresponding Swift Testing unit tests
- **File size**: Single file ≤300 lines (business code), single function ≤50 lines
- **CI must be green**: PR won't be merged if CI fails

## Architecture Constraints

- Layered architecture: See `docs/design/principles.md`
- ViewModel injects dependencies through Protocol for easy testing
- Service layer abstracted as Protocol (e.g., ChatServiceProtocol, SessionServiceProtocol)
- Reference existing pattern: `ChatViewModel` + `ChatServiceProtocol` approach

## Tech Stack

- **UI**: SwiftUI
- **gRPC**: grpc-swift v2 (grpc-swift-protobuf + grpc-swift-nio-transport)
- **Proto generation**: SPM Build Plugin (zero-cost sync on proto changes)
- **Testing**: Swift Testing framework
- **Minimum version**: iOS 26.2
- **Dependency locking**: `Package.resolved` must be committed for reproducible builds

## Notes

- ⚠️ **Repository is public** — Writing IP addresses, passwords, API keys, internal domain names and other sensitive information is prohibited. Credentials go through environment variables, addresses go through config files (gitignored)
- `tasks/` is read-only — do not create, modify, or delete files in it
- For questions or uncertainties, write in PR description, PM will see it
- Server API docs: `../server/docs/design/` + `../proto/`
