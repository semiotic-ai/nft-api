<!--
SPDX-FileCopyrightText: 2025 Semiotic Labs

SPDX-License-Identifier: Apache-2.0
-->

# nft-api

A secure NFT API service built with Rust, designed for blockchain token management with strict security and code quality standards.

## Architecture Overview

This project is organized as a Rust workspace with three main crates:

- **`api`** - Main HTTP server implementation with Axum, configuration management, middleware, and graceful shutdown coordination
- **`api-client`** - Common client trait and types for external API integrations
- **`external-apis`** - Blockchain data provider integrations (Moralis, Pinax)


## API Endpoints

### Health Check
- **GET** `/health` - Server health status with external API client health aggregation

### Contract Analysis
- **POST** `/v1/contract/status` - Analyze contract addresses for spam classification

### API Documentation
- **GET** `/swagger-ui` - Interactive Swagger UI for API exploration
- **GET** `/api-doc/openapi.json` - OpenAPI specification in JSON format

#### Contract Status Request
```json
{
  "addresses": [
    "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
    "0x1234567890123456789012345678901234567890"
  ],
  "chain": 1
}
```

#### Contract Status Response
```json
{
  "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd": {
    "contract_spam_status": true,
    "message": "Contract analysis result"
  },
  "0x1234567890123456789012345678901234567890": {
    "contract_spam_status": false,
    "message": "Contract analysis result"
  }
}
```

## Local Setup

Follow these steps to get the NFT API running locally with Docker:

### 1. Create Development Configuration

First, create a development configuration file based on the example:

```bash
cp config.example.json config.development.json
```

Edit `config.development.json` and update the API credentials.


### 2. Start Development Environment

Build and start the development container:

```bash
just docker-dev-up
```

This will:
- Build the development Docker image with full toolchain
- Start the API server with hot reloading enabled
- Mount your source code for live editing
- Use your `config.development.json` configuration

### 3. Verify Setup

Test that the API is working correctly:

```bash
# Test the health endpoint
just local-test-health

# Test the contract status endpoint
just local-test-status

# Test with custom contract address
just local-test-status "0x1234567890123456789012345678901234567890"

# Test with multiple addresses on Polygon (chain ID 137)
just local-test-status "0xabc123...,0xdef456...,0x789abc..." "137"

# Test with single address on Arbitrum (chain ID 42161)
just local-test-status "0x1234567890123456789012345678901234567890" "42161"
```

Expected responses:
- **Health**: JSON with server status and API client health
- **Contract Status**: JSON with contract analysis results

### 4. Development Workflow

- **Edit code**: Changes are automatically reloaded in the container
- **Update config**: Modify `config.development.json` for immediate effect
- **View logs**: Container logs show debug-level information
- **API docs**: Visit `http://localhost:3000/swagger-ui` for interactive documentation

### 5. Stop Environment

When finished developing:

```bash
just docker-dev-down
```

## Quick Start

### Local Development

1. **Setup development environment:**
```bash
just prepare-dev-setup
```

2. **Build the project:**
```bash
cargo build --release
# or
just check
```

3. **Run with default configuration:**
```bash
cargo run
```

4. **Run with custom configuration:**
```bash
export SERVER_PORT=8080
export ENVIRONMENT=development
cargo run
```

### Docker Development

1. **Start development environment:**
```bash
just docker-dev-up
# or in background
just docker-dev-up-bg
```

2. **Stop development environment:**
```bash
just docker-dev-down
```

### Docker Production

1. **Build production image:**
```bash
just docker-build
```

2. **Start production environment:**
```bash
just docker-prod-up
# or in background
just docker-prod-up-bg
```

3. **Stop production environment:**
```bash
just docker-prod-down
```

## Configuration

The NFT API uses a hierarchical configuration system powered by the [`config`](https://crates.io/crates/config) crate. Configuration is loaded in the following order, with later sources overriding earlier ones:

1. **Default Values** - Built-in sensible defaults
2. **Configuration Files** - Optional JSON files
3. **Environment-Specific Files** - Environment-based configuration files (`config.development.json`)
4. **Environment Variables** - Runtime configuration with `SERVER_` prefix

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `host` | IP Address | `127.0.0.1` | Server bind address |
| `port` | Integer | `3000` | Server port number |
| `timeout_seconds` | Integer | `30` | Request timeout in seconds |
| `environment` | String | `development` | Environment type (`production`, `development`, `testing`) |
| `external_apis.moralis.enabled` | Boolean | `false` | Enable Moralis API client |
| `external_apis.moralis.api_key` | String | - | Moralis API key for authentication |
| `external_apis.moralis.base_url` | String | `https://deep-index.moralis.io/api/v2` | Moralis API base URL |
| `external_apis.moralis.timeout_seconds` | Integer | `30` | Moralis request timeout |
| `external_apis.pinax.enabled` | Boolean | `false` | Enable Pinax API client |
| `external_apis.pinax.api_user` | String | - | Pinax API username |
| `external_apis.pinax.api_auth` | String | - | Pinax API authentication token |
| `external_apis.pinax.endpoint` | String | `https://api.pinax.network/sql` | Pinax API endpoint |
| `rate_limiting.enabled` | Boolean | `true` | Enable rate limiting |
| `rate_limiting.requests_per_minute` | Integer | `60` | Maximum requests per IP per minute |
| `extensions` | Object | `{}` | Additional configuration parameters |

### Configuration Methods

#### 1. Environment Variables

Set environment variables with the `SERVER_` prefix:

```bash
# Basic server configuration
export SERVER_HOST=0.0.0.0
export SERVER_PORT=8080
export SERVER_TIMEOUT_SECONDS=60
export ENVIRONMENT=production

# External API configuration
export SERVER_EXTERNAL_APIS_MORALIS_ENABLED=true
export SERVER_EXTERNAL_APIS_MORALIS_API_KEY=your_moralis_api_key
export SERVER_EXTERNAL_APIS_PINAX_ENABLED=true
export SERVER_EXTERNAL_APIS_PINAX_API_USER=your_pinax_username
export SERVER_EXTERNAL_APIS_PINAX_API_AUTH=your_pinax_auth_token

# Rate limiting
export SERVER_RATE_LIMITING_REQUESTS_PER_MINUTE=100
```

#### 2. Configuration Files

Create configuration files in the project root:

**config.production.json:**
```json
{
  "host": "0.0.0.0",
  "port": 8080,
  "timeout_seconds": 60,
  "environment": "production",
  "external_apis": {
    "moralis": {
      "enabled": true,
      "api_key": "your_moralis_api_key",
      "base_url": "https://deep-index.moralis.io/api/v2",
      "timeout_seconds": 30,
      "health_check_timeout_seconds": 5,
      "max_retries": 3
    },
    "pinax": {
      "enabled": true,
      "api_user": "your_pinax_username",
      "api_auth": "your_pinax_auth_token",
      "endpoint": "https://api.pinax.network/sql",
      "db_name": "mainnet:evm-nft-tokens@v0.6.2",
      "timeout_seconds": 30,
      "health_check_timeout_seconds": 5,
      "max_retries": 3
    }
  },
  "rate_limiting": {
    "enabled": true,
    "requests_per_minute": 60
  },
  "extensions": {
    "log_level": "info",
    "enable_cors": true,
    "enable_swagger": true
  }
}
```

#### 3. Environment-Specific Configuration

Create environment-specific configuration files that are loaded based on the `ENVIRONMENT` variable:

- `config.production.json` - Production settings
- `config.development.json` - Development settings
- `config.testing.json` - Testing settings

Example `config.development.json` with disabled external APIs:
```json
{
"host": "127.0.0.1",
"port": 3000,
"external_apis": {
"moralis": {
"enabled": false
},
"pinax": {
"enabled": false
}
},
"rate_limiting": {
"enabled": false
},
"extensions": {
"log_level": "debug"
}
}
```

### Configuration Precedence

Configuration values are loaded in hierarchical order. For example, if you have:

1. Default port: `3000`
2. `config.json` port: `8080`
3. `config.production.json` port: `443`
4. `SERVER_PORT` environment variable: `9000`

The final port will be `9000` (environment variable takes highest precedence).


## Development

### Available Commands

The project uses [`just`](https://github.com/casey/just) for task management. Run `just` to see all available commands.

**Testing:**
```bash
just test           # Run tests with nextest (preferred)
cargo test --workspace --all-features  # Alternative
```

**Code Quality:**
```bash
just lint           # Run all checks (format, clippy, compilation)
just fmt            # Format code with nightly rustfmt
just clippy         # Run clippy linting alone
just check          # Check compilation without building
just organize       # Auto-organize Cargo.toml files
```

**Security & Dependencies:**
```bash
just deny           # Check licenses and dependencies
cargo audit         # Security audit of dependencies
```

**Docker Development:**
```bash
just docker-build           # Build production image
just docker-build-dev       # Build development image
just docker-rebuild-prod    # Rebuild production image without cache
just docker-rebuild-dev     # Rebuild development image without cache
just docker-dev-up          # Start dev environment
just docker-dev-up-bg       # Start dev environment in background
just docker-dev-down        # Stop dev environment
just docker-prod-up         # Start production environment
just docker-prod-up-bg      # Start production in background
just docker-prod-down       # Stop production environment
just docker-clean           # Clean Docker images and containers
```

**Local Testing:**
```bash
just local-test-health                                    # Test health endpoint
just local-test-status                                    # Test contract status with defaults
just local-test-status "0x123..."                       # Test with custom address
just local-test-status "0x123...,0x456..." "137"        # Test multiple addresses on Polygon
just local-test-status "0x123..." "42161"               # Test single address on Arbitrum
```

**Utilities:**
```bash
just clean          # Clean build artifacts
just prepare-dev-setup  # Install tools and setup pre-commit hooks
```

## API Documentation

### Swagger UI

The API includes interactive Swagger UI documentation available when the server is running:

- **URL**: `http://localhost:3000/swagger-ui`
- **Features**: Interactive API exploration, request/response examples, schema documentation
- **Configuration**: Can be disabled in production by setting `SERVER_EXTENSIONS_ENABLE_SWAGGER=false`

### OpenAPI Specification

The complete OpenAPI 3.0 specification is available in JSON format:

- **URL**: `http://localhost:3000/api-doc/openapi.json`
- **Usage**: API client generation, integration testing, documentation tooling
- **Content**: Complete endpoint documentation with request/response schemas

## Production Deployment

### Security Considerations

- **API Credentials**: Set real Moralis and Pinax API keys, remove placeholder values
- **Rate Limiting**: Always enabled in production (validates to prevent DoS attacks)
- **HTTPS**: Enforce HTTPS URLs for external API endpoints
- **Host Binding**: Consider firewall/proxy configuration when binding to `0.0.0.0`
- **Secrets Management**: Use environment variables or secure secret stores, never commit API keys

### Health Monitoring

The `/health` endpoint provides comprehensive service health information:

```json
{
  "status": "Up",
  "version": "0.1.0",
  "environment": "production",
  "timestamp": "2025-01-22T10:30:00Z",
  "api_clients": {
    "moralis": "Up",
    "pinax": "Up"
  }
}
```


## License

Licensed under the Apache License, Version 2.0. See the LICENSE file for details.
