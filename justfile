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

# === Development Helpers ===

# Clean build artifacts
clean:
    cargo clean
