# agent-proto/ — Generated protobuf contract crate

## What this is

Generate and expose Rust gRPC/protobuf types used across the workspace.
Treat this crate as shared protocol code, not business logic.

## How it works

Run code generation in `build.rs` via `tonic_build`.
Compile proto files from `../../../proto/`: `common.proto`, `auth.proto`, `chat.proto`, `session.proto`, `health.proto`.
Generate both server and client code.
Include modules through `tonic::include_proto!("ai.agent.platform.v1")`.
Re-export `ai::agent::platform::v1::*` from `src/lib.rs`.

Keep crate-level `#![allow(...)]` for generated-code lint noise only.

## Constraints

Do not add domain/business logic here.
Do not hand-edit generated output.
Change contracts in `.proto` files, then regenerate via build.
Keep build paths valid when moving files.
