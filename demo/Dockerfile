# FastC Playground - Multi-stage Docker build
# Stage 1: Build frontend
FROM node:20-alpine AS frontend-builder

WORKDIR /app/frontend
COPY crates/fastc-playground/frontend/package*.json ./
RUN npm ci
COPY crates/fastc-playground/frontend/ ./
RUN npm run build

# Stage 2: Build Rust backend
FROM rust:1.85-bookworm AS rust-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/fastc/Cargo.toml crates/fastc/
COPY crates/fastc-lsp/Cargo.toml crates/fastc-lsp/
COPY crates/fastc-playground/Cargo.toml crates/fastc-playground/

# Create dummy source files for dependency compilation
RUN mkdir -p crates/fastc/src crates/fastc-lsp/src crates/fastc-playground/src && \
    echo "pub fn dummy() {}" > crates/fastc/src/lib.rs && \
    echo "fn main() {}" > crates/fastc-lsp/src/main.rs && \
    echo "fn main() { println!(\"dummy\"); }" > crates/fastc-playground/src/main.rs && \
    echo "pub fn dummy() {}" > crates/fastc-playground/src/lib.rs

# Build dependencies (this layer gets cached)
RUN cargo build --release -p fastc-playground 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/
COPY runtime/ runtime/

# Copy built frontend assets
COPY --from=frontend-builder /app/frontend/dist crates/fastc-playground/frontend/dist

# Touch source files to force rebuild (not just dependency cache)
RUN touch crates/fastc/src/lib.rs crates/fastc-playground/src/main.rs crates/fastc-playground/src/lib.rs

# Build the actual binary
RUN cargo build --release -p fastc-playground

# Stage 3: Runtime image
FROM debian:bookworm-slim

# Install C compiler for running user code
RUN apt-get update && apt-get install -y \
    gcc \
    libc6-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary
COPY --from=rust-builder /app/target/release/fastc-playground /app/

# Copy the runtime headers (needed for compiling user code)
COPY --from=rust-builder /app/runtime /app/runtime

# Set environment variable for runtime location
ENV FASTC_RUNTIME=/app/runtime

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/ || exit 1

# Run the playground
CMD ["/app/fastc-playground", "--host", "0.0.0.0", "--port", "3000"]
