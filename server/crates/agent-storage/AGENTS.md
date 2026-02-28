# agent-storage/ — Infrastructure adapters for persistence

## What this is

Implement persistence adapters for capability traits using PostgreSQL.
Expose concrete stores that satisfy domain/capability ports.
Keep this crate focused on I/O and data mapping.

This crate currently exports storage adapters through `pg` and re-exports `PostgresEventStore`.
The `storage.rs` placeholder is intentionally minimal and should stay non-authoritative.

## Layer

Treat this crate as **Infrastructure layer (Layer 4)**.
Implement capability/domain traits here; do not define business policies here.
Translate between database rows and domain objects.
Encapsulate SQL, transactions, indexing assumptions, and database error mapping.

Keep dependency direction one-way:
- depend on `agent-domain` trait contracts
- optionally use shared utility crates
- never pull orchestration or channel behavior into storage adapters

## Constraints

Implement traits exactly as contracts specify.
Keep all business decisions out of this crate:
- no compaction policy selection
- no session lifecycle transition rules
- no turn orchestration

Use this crate to perform:
- transaction boundaries
- row insert/select/update
- serialization/deserialization
- conflict/error mapping

Map infra failures into trait-level error enums consistently.
Prefer small helper functions for conversion and SQL error translation.
Preserve tenant/user scoping in queries whenever trait contracts require it.

Keep adapter behavior deterministic and testable.
Return data in contract-defined order (for example sequence ordering) rather than DB incidental order.

## Notes

Treat `src/pg/` as the authoritative PostgreSQL implementation namespace.
Read and maintain `src/pg/AGENTS.md` for concrete store-specific rules.
Keep crate-level guidance generic; keep Postgres details in the `pg` subdirectory doc.

When adding new adapters:
- add them as trait implementations first
- keep constructor shape simple (`new(pool)` pattern)
- re-export intentional public adapter types from `lib.rs`
