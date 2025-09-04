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
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Switching to development environment..."
    echo "Stopping production environment..."
    docker compose -f docker-compose.prod.yml down --remove-orphans || true
    echo "Cleaning up any existing development containers..."
    docker compose -f docker-compose.dev.yml down --remove-orphans || true
    echo "Starting development environment..."
    docker compose -f docker-compose.dev.yml up

# Start development environment in background
docker-dev-up-bg:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Switching to development environment..."
    echo "Stopping production environment..."
    docker compose -f docker-compose.prod.yml down --remove-orphans || true
    echo "Cleaning up any existing development containers..."
    docker compose -f docker-compose.dev.yml down --remove-orphans || true
    echo "Starting development environment in background..."
    docker compose -f docker-compose.dev.yml up -d

# Stop development environment
docker-dev-down:
    docker compose -f docker-compose.dev.yml down

# Start production environment
docker-prod-up:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Switching to production environment..."
    echo "Stopping development environment..."
    docker compose -f docker-compose.dev.yml down --remove-orphans || true
    echo "Cleaning up any existing production containers..."
    docker compose -f docker-compose.prod.yml down --remove-orphans || true
    echo "Starting production environment..."
    docker compose -f docker-compose.prod.yml up

# Start production environment in background
docker-prod-up-bg:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Switching to production environment..."
    echo "Stopping development environment..."
    docker compose -f docker-compose.dev.yml down --remove-orphans || true
    echo "Cleaning up any existing production containers..."
    docker compose -f docker-compose.prod.yml down --remove-orphans || true
    echo "Starting production environment in background..."
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
# Usage: just local-test-status [addresses] [chain_id]
# addresses: comma-separated Ethereum addresses (default: test address)
# chain_id: numeric chain ID (default: 1 for Ethereum mainnet)
local-test-status addresses="0xabcdefabcdefabcdefabcdefabcdefabcdefabcd" chain_id="1":
    #!/usr/bin/env bash
    set -euo pipefail
    # Convert comma-separated addresses to JSON array format
    addresses_json=$(echo "{{addresses}}" | sed 's/,/","/g' | sed 's/^/["/' | sed 's/$/"]/')
    curl -X POST 0:3000/v1/contract/status \
    -H "Content-Type: application/json" \
    --data "{\"addresses\": $addresses_json, \"chain_id\": {{chain_id}}}"

# Test local API health endpoint
local-test-health:
    curl 0:3000/health

# === Chain-Specific Testing Commands ===

# Test Ethereum mainnet with real contracts
local-test-status-ethereum:
    just local-test-status "0x2e3a0bbc85119aa0dde4825cec8d474f5b09b721,0x2e3a148e113e381e2b601a0c9914576ffe8d85d4" "1"

# Test Polygon with real contracts
local-test-status-polygon:
    just local-test-status "0xaffee4b319ba0f479085e5ea76c89c51aa4e67aa,0xafffaa7ee82c8d6066d7d7ab36c1ee6a1219571a" "137"

# Test Base with real contracts
local-test-status-base:
    just local-test-status "0x397823d28b62aa879f517675da859de2a0ec2abe,0x3992f0829292a4da4daacf40a8ead8733ac77cea" "8453"

# Test Avalanche with real contracts
local-test-status-avalanche:
    just local-test-status "0x0000000000771a79d0fc7f3b7fe270eb4498f20b,0x000b9a715122a2c91513f38f9322ef47dc97294a" "43114"

# Test Arbitrum with real contracts
local-test-status-arbitrum:
    just local-test-status "0x00000000001594c61dd8a6804da9ab58ed2483ce,0x00000000016c35e3613ad3ed484aa48f161b67fd" "42161"

# Test all supported chains sequentially
local-test-status-all-chains:
    #!/usr/bin/env bash
    echo "Testing all supported chains with real contracts..."
    echo ""
    echo "🔷 Testing Ethereum (Chain ID: 1)..."
    just local-test-status-ethereum
    echo ""
    echo "🟣 Testing Polygon (Chain ID: 137)..."
    just local-test-status-polygon
    echo ""
    echo "🔵 Testing Base (Chain ID: 8453)..."
    just local-test-status-base
    echo ""
    echo "🔴 Testing Avalanche (Chain ID: 43114)..."
    just local-test-status-avalanche
    echo ""
    echo "🟠 Testing Arbitrum (Chain ID: 42161)..."
    just local-test-status-arbitrum
    echo ""
    echo "✅ Multi-chain testing completed!"

# === Development Helpers ===

# Clean build artifacts
clean:
    cargo clean
