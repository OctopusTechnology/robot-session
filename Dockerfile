# Build stage
FROM rust:1.87 AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY session-manager/ ./session-manager/
COPY microservice-sdk/ ./microservice-sdk/

# Build the session-manager application
RUN cargo build --release --bin session-manager

# Runtime stage
FROM ubuntu:24.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    net-tools \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Copy the binary
COPY --from=builder /app/target/release/session-manager /usr/local/bin/session-manager

# Create config directory
RUN mkdir -p /etc/session-manager && chown appuser:appuser /etc/session-manager

# Copy default config
COPY session-manager/config/docker.toml /etc/session-manager/config.toml

# Switch to app user
USER appuser

# Expose port
EXPOSE 8080

# Run the application
CMD ["session-manager", "--config", "/etc/session-manager/config.toml"]