# NewClaw Production Dockerfile
# Multi-stage build for minimal image size

# Stage 1: Build
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source
COPY src ./src

# Build for release
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false newclaw

# Copy binary from builder
COPY --from=builder /app/target/release/newclaw /usr/local/bin/newclaw

# Copy default config
COPY config/default.toml /etc/newclaw/config.toml

# Create directories
RUN mkdir -p /var/lib/newclaw /var/log/newclaw && \
    chown -R newclaw:newclaw /var/lib/newclaw /var/log/newclaw

# Switch to non-root user
USER newclaw

# Expose ports
EXPOSE 3000 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Set environment
ENV RUST_LOG=info
ENV NEWCLAW_CONFIG=/etc/newclaw/config.toml

# Start
CMD ["newclaw", "gateway"]
