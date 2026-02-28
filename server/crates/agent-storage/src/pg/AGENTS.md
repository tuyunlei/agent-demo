# agent-storage/src/pg/ — PostgreSQL store implementations

## What this is

Implement PostgreSQL-backed stores for domain capability traits.
Provide adapters for event streams, message projection workflows, and user auth records.
Keep this directory responsible for SQL, transactions, row decoding, and DB error mapping.

## Stores

Use `PostgresEventStore` for event-centric persistence.
Implement `EventStore` methods with explicit transaction boundaries:
- lock session row with `FOR UPDATE` before sequence allocation
- append events with session-local `sequence_number`
- update `sessions.last_sequence`, `event_count`, and `version`
- read events in ascending sequence order
- read recent events with bounded limit and return ascending order

Use `PostgresMessageStore` for message/session projection APIs.
Implement `MessageStore` methods for session CRUD-like access and ordered history reads.

Use `PostgresUserStore` for auth persistence.
Implement `AuthPort` methods for user creation, password verification, and duplicate-email mapping.

## Row mapping

Keep conversion helpers centralized in `event_store_row.rs`.
Use helper functions for UUID parsing and integer conversions.
Map rows with `row_to_event` and `row_to_session`.
Normalize SQL/driver errors with `db`, `map_append_error`, and `map_create_error`.
Map unique violations to domain conflicts (`SequenceConflict`, `SessionAlreadyExists`).

## Migration rules

Align implementation with the event-model migration path.
Treat `events` as source-of-truth for event history.
Treat `messages` as compatibility/projection during transition.
Preserve key differences from legacy storage:
- global `messages.sequence_num` is not session-local ordering
- event stream uses typed payload + envelope metadata

When schema evolves, update row mappers and SQL aliases together.
Keep compatibility behavior stable while legacy columns are still used.

## Constraints

Keep this directory infrastructure-only.
Do not implement lifecycle policies, compaction decisions, or handler orchestration here.
Do not leak SQL models outside adapter boundaries.
Keep ordering, bounds, and conflict handling explicit in each method.
Write/update `*_tests.rs` whenever adapter behavior changes.
