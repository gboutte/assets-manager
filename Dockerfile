# syntax=docker/dockerfile:1

# --- Build stage: full Rust toolchain, only used to compile ---
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Cache mounts keep the cargo registry and build artifacts between builds,
# so only changed code is recompiled
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/assets-manager /usr/local/bin/assets-manager

# --- Runtime stage: just the binary on a small base image ---
FROM debian:bookworm-slim

RUN useradd --system --uid 10001 --no-create-home app \
    && mkdir /data \
    && chown app /data

COPY --from=builder /usr/local/bin/assets-manager /usr/local/bin/assets-manager

# ROCKET_LOG_LEVEL: a release build defaults to "critical", which hides info logs
# (uploads, deleted tags, cleanup runs)
ENV HOST=0.0.0.0 \
    PORT=8000 \
    STORAGE_TYPE=filesystem \
    STORAGE_PATH=/data \
    ROCKET_LOG_LEVEL=normal

VOLUME /data
EXPOSE 8000
USER app

# API_TOKEN (or any API_TOKEN* variable) must be given at runtime
CMD ["assets-manager"]
