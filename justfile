# Default recipe - show available commands
default:
    @just --list

# === Code Quality ===

# Run cargo fmt to format code
fmt:
    cargo +nightly fmt --all

# Run cargo clippy for linting
clippy:
    cargo clippy --workspace --all-targets --all-features

# Run cargo check for compilation errors
check:
    cargo check --workspace --all-targets --all-features

# Organize Cargo.toml files (autoinherit and sort)
organize:
    cargo autoinherit
    cargo sort --workspace

# Run all linting and formatting checks
lint: fmt clippy check

# === Testing ===

# Run tests with nextest
test:
    cargo nextest run --workspace --all-features

# === Security & Dependencies ===

# Check licenses and dependencies with cargo-deny
deny:
    cargo deny check

# === Development Setup ===

# Install necessary cargo tools and setup pre-commit
prepare-dev-setup:
    cargo install cargo-nextest --locked
    cargo install cargo-deny --locked
    cargo install cargo-audit --locked
    cargo install cargo-autoinherit --locked
    cargo install cargo-sort --locked
    @if command -v pre-commit >/dev/null 2>&1; then \
    echo "Setting up pre-commit hooks..."; \
    pre-commit install; \
    else \
    echo "pre-commit not found. Please install it with: uv tool install pre-commit"; \
    fi

# === Docker ===

# Build production Docker image
docker-build:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Building production Docker image using docker-compose..."
    docker compose -f docker-compose.prod.yml build
    echo "Production build completed successfully!"

# Build development Docker image
docker-build-dev:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Building development Docker image using docker-compose..."
    docker compose -f docker-compose.dev.yml build
    echo "Development build completed successfully!"

# Start development environment
docker-dev-up:
    docker compose -f docker-compose.dev.yml up

# Start development environment in background
docker-dev-up-bg:
    docker compose -f docker-compose.dev.yml up -d

# Stop development environment
docker-dev-down:
    docker compose -f docker-compose.dev.yml down

# Start production environment
docker-prod-up:
    docker compose -f docker-compose.prod.yml up

# Start production environment in background
docker-prod-up-bg:
    docker compose -f docker-compose.prod.yml up -d

# Stop production environment
docker-prod-down:
    docker compose -f docker-compose.prod.yml down

# Clean Docker images and containers
docker-clean:
    #!/usr/bin/env bash
    echo "Cleaning Docker compose environments and containers..."
    # Clean up compose environments
    docker compose -f docker-compose.dev.yml down --remove-orphans --volumes || true
    docker compose -f docker-compose.prod.yml down --remove-orphans --volumes || true
    # Clean up Docker system
    docker container prune -f
    docker image prune -f
    docker builder prune -f
    docker volume prune -f
    echo "Docker cleanup completed!"

# Rebuild prod image without cache
docker-rebuild-prod:
    #!/usr/bin/env bash
    echo "Rebuilding images without cache..."
    docker compose -f docker-compose.prod.yml build --no-cache
    echo "Cache-free rebuild completed!"

# Rebuild development image without cache
docker-rebuild-dev:
    #!/usr/bin/env bash
    echo "Rebuilding development image without cache..."
    docker compose -f docker-compose.dev.yml build --no-cache
    echo "Development cache-free rebuild completed!"

# Test local API contract status endpoint
local-test-status:
    curl -X POST 0:3000/v1/contract/status \
    -H "Content-Type: application/json" \
    --data '{"addresses": ["0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"], "chain_id": 1}'

# Test local API health endpoint
local-test-health:
    curl 0:3000/health

# === Development Helpers ===

# Clean build artifacts
clean:
    cargo clean
