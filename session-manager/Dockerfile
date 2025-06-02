# Build stage
FROM rust:latest as builder

WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Copy the binary
COPY --from=builder /app/target/release/session-manager /usr/local/bin/session-manager

# Create config directory
RUN mkdir -p /etc/session-manager && chown appuser:appuser /etc/session-manager

# Copy default config
COPY config/docker.toml /etc/session-manager/config.toml

# Switch to app user
USER appuser

# Expose port
EXPOSE 8080

# Run the application
CMD ["session-manager", "--config", "/etc/session-manager/config.toml"]