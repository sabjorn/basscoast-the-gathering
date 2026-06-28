# Multi-stage build for Bass Coast: The Gathering web app
# Stage 1: Build the Rust application
FROM rust:1.96-bookworm AS builder

WORKDIR /app

# Install SQLx CLI for migrations
RUN cargo install sqlx-cli --no-default-features --features sqlite

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY static ./static

# Create temporary database and run migrations for SQLx compile-time verification
ENV DATABASE_URL=sqlite:///tmp/build.db
RUN sqlx database create
RUN sqlx migrate run

# Build for release (both binaries)
RUN cargo build --release --bin bctg
RUN cargo build --release --bin import-json

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    sqlite3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/bctg /usr/local/bin/bctg
COPY --from=builder /app/target/release/import-json /usr/local/bin/import-json

# Copy static files
COPY --from=builder /app/static /app/static

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

# Copy JSON data file
COPY data/bass_coast_artists_history.json /app/data/

# Copy entrypoint script
COPY docker-entrypoint.sh /app/
RUN chmod +x /app/docker-entrypoint.sh

# Create directory for database
RUN mkdir -p /app/data

# Set environment variables
ENV DATABASE_URL=sqlite:///app/data/basscoast.db
ENV SERVER_PORT=3000
ENV RUST_LOG=info

# Expose port
EXPOSE 3000

# Entrypoint handles database import, CMD specifies what to run
ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["bctg"]
