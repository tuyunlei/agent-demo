# TM4.1: GRDB + SQLite Local Message Cache

## Goal

Use GRDB + SQLite for local message cache, enabling offline chat history viewing.

## Acceptance Criteria

- [ ] GRDB dependency introduced (SPM)
- [ ] Local database schema: messages table (id, session_id, role, content, timestamp)
- [ ] MessageStore protocol + GRDB implementation
- [ ] Write to local when receiving server messages
- [ ] Load local cache first on startup, then async fetch server delta
- [ ] View cached history messages when offline
- [ ] Unit tests: write, read, delta sync logic
- [ ] SwiftLint + SwiftFormat pass

## Context

- Design doc: `docs/design/foundation.md` (foundation layer design, includes persistence solution)
- Existing message loading: `ChatViewModel.loadHistory()` → `SessionServiceClient`
- Message model: `AgentDemo/ViewModels/ChatMessage.swift`

## Constraints

- Don't use CoreData or Realm, use GRDB
- Database file goes in Application Support directory
- Schema migration uses GRDB's DatabaseMigrator
