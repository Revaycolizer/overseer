# Multi-stage build for Overseer

# --- Stage 1: Build Frontend (Web Dioxus Client) ---
FROM rust:1.77-slim AS frontend-builder
WORKDIR /usr/src/app

# Install compilation dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*

# Install Dioxus CLI
RUN cargo install dioxus-cli

# Add wasm32 target
RUN rustup target add wasm32-unknown-unknown

# Copy source and config files
COPY . .

# Build the web application using Dioxus CLI
RUN dx build --release --platform web

# --- Stage 2: Build Backend Server ---
FROM rust:1.77-slim AS backend-builder
WORKDIR /usr/src/app

# Install compilation dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev sqlite3 libsqlite3-dev && rm -rf /var/lib/apt/lists/*

# Copy files
COPY . .

# Build the server binary
RUN cargo build --release --no-default-features --features web

# --- Stage 3: Runner (Nginx + Backend Server) ---
FROM debian:bookworm-slim
WORKDIR /app

# Install Nginx and other runtime dependencies
RUN apt-get update && apt-get install -y nginx sqlite3 libsqlite3-0 ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Copy backend binary
COPY --from=backend-builder /usr/src/app/target/release/overseer /app/overseer

# Copy static frontend assets to nginx html directory
COPY --from=frontend-builder /usr/src/app/dist /usr/share/nginx/html

# Copy nginx config
COPY nginx.conf /etc/nginx/nginx.conf

# Expose HTTP port
EXPOSE 80

# Script to run both backend server and nginx
COPY scripts/start.sh /app/start.sh
RUN chmod +x /app/start.sh

CMD ["/app/start.sh"]
