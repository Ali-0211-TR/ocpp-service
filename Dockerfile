# ─────────────────────────────────────────────────────────────────
# Texnouz OCPP Central System — Multi-stage Docker Build
# ─────────────────────────────────────────────────────────────────

# ── Stage 1: Build ──────────────────────────────────────────────
FROM rust:1.82-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency build layer — copy only Cargo manifests first
COPY Cargo.toml Cargo.lock* ./
# Create a dummy main.rs so cargo can resolve dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs

# Build only dependencies (cached if Cargo.toml didn't change)
RUN cargo build --release --bin ocpp-service 2>/dev/null || true
RUN rm -rf src

# Copy the actual source
COPY src/ src/

# Touch main.rs to force rebuild of the application (not deps)
RUN touch src/main.rs src/lib.rs

# Build the final binary
RUN cargo build --release --bin ocpp-service

# ── Stage 2: Runtime ───────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN groupadd -r ocpp && useradd -r -g ocpp -m -d /home/ocpp ocpp

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ocpp-service /app/ocpp-service

# Create directories for config and data
RUN mkdir -p /app/data /app/config && chown -R ocpp:ocpp /app

USER ocpp

# Expose REST API and WebSocket ports
EXPOSE 8080 9000

# Environment defaults (can be overridden)
ENV OCPP_CONFIG=/app/config/config.toml \
    OCPP_LOG_FORMAT=json \
    OCPP_LOG_LEVEL=info \
    RUST_BACKTRACE=1

# Health check (REST API)
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/app/ocpp-service"]
