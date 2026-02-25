# ===== Stage 1: Build =====
FROM rust:1.88-bookworm AS builder

# Install protoc (required by prost-build)
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Replicate repo structure so relative paths in build.rs work:
#   /build/proto/          (from repo root proto/)
#   /build/server/         (from repo root server/)
# build.rs references ../../../proto from crates/agent-proto/

COPY proto/ ./proto/
COPY server/ ./server/

WORKDIR /build/server

# Build release binary
# sqlx::migrate! embeds migrations at compile time, no runtime files needed
RUN cargo build --release --bin agent-server

# ===== Stage 2: Runtime =====
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/server/target/release/agent-server /usr/local/bin/agent-server

EXPOSE 50051

ENTRYPOINT ["agent-server"]
