<!--
SPDX-FileCopyrightText: 2025 Semiotic Labs

SPDX-License-Identifier: Apache-2.0
-->

# nft-api

A secure multi-chain NFT API service built with Rust, designed for comprehensive blockchain token management across Ethereum, Polygon, Base, Avalanche, and Arbitrum networks with AI-powered spam detection, strict security and code quality standards.

## Architecture Overview

This project is organized as a Rust workspace with five main crates:

- **`api`** - Main HTTP server implementation with Axum, multi-chain request validation, configuration management, middleware, and graceful shutdown coordination
- **`api-client`** - Common client trait and types for external API integrations with chain-specific health checks
- **`external-apis`** - Multi-chain blockchain data provider integrations (Moralis, Pinax) with chain-specific optimizations and health checks
- **`shared-types`** - Common blockchain types, comprehensive chain definitions, and multi-chain capability validation
- **`spam-predictor`** - OpenAI-powered contract spam classification with caching, supporting all chain networks

## Docker Environment Isolation

This project uses **advanced Docker environment isolation** to prevent development and production conflicts:

- **Development Environment**: `nft-api-dev-*` containers with hot reload and full toolchain
- **Production Environment**: `nft-api-prod-*` containers with security hardening and optimization
- **Automatic Switching**: Commands automatically stop the other environment and clean up resources
- **Complete Isolation**: Separate containers, networks, and volumes prevent any interference

**Key Benefits**: No more build failures when switching between environments, automatic cleanup, and safe concurrent development workflows.

## Multi-Chain Support

The NFT API provides comprehensive support for major blockchain networks:

| Chain | Chain ID | Status | Moralis | Pinax | AI Spam Detection |
|-------|----------|---------|---------|-------|------------------|
| **Ethereum Mainnet** | `1` | ✅ Full | ✅ | ✅ | ✅ |
| **Polygon** | `137` | ✅ Full | ✅ | ✅ | ✅ |
| **Base** | `8453` | ✅ Full | ✅ | ✅ | ✅ |
| **Avalanche C-Chain** | `43114` | ✅ Full | ✅ | ✅ | ✅ |
| **Arbitrum One** | `42161` | ✅ Full | ✅ | ✅ | ✅ |

## API Endpoints

### Health Check
- **GET** `/health` - Server health status with chain-specific external API client health aggregation

### Multi-Chain Contract Analysis
- **POST** `/v1/contract/status` - Analyze contract addresses for spam classification on specific blockchain networks

### API Documentation
- **GET** `/swagger-ui` - Interactive Swagger UI for API exploration with multi-chain examples
- **GET** `/api-doc/openapi.json` - OpenAPI specification in JSON format

#### Contract Status Request Format
```json
{
  "chain_id": 1,
  "addresses": [
    "0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d",
    "0x1234567890abcdef1234567890abcdef12345678"
  ]
}
```

#### Multi-Chain Examples

**Ethereum (Chain ID: 1):**
```json
{
  "chain_id": 1,
  "addresses": ["0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d"]
}
```

**Polygon (Chain ID: 137):**
```json
{
  "chain_id": 137,
  "addresses": ["0x1234567890abcdef1234567890abcdef12345678"]
}
```

**Base (Chain ID: 8453):**
```json
{
  "chain_id": 8453,
  "addresses": ["0x60e4d786628fea6478f785a6d7e704777c86a7c6"]
}
```

**Avalanche (Chain ID: 43114):**
```json
{
  "chain_id": 43114,
  "addresses": ["0x071126cbec1c5562530ab85fd80dd3e3a42a70b8"]
}
```

**Arbitrum (Chain ID: 42161):**
```json
{
  "chain_id": 42161,
  "addresses": ["0x32400084c286cf3e17e7b677ea9583e60a000324"]
}
```

#### Contract Status Response Format
```json
{
  "0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d": {
    "chain_id": 1,
    "contract_spam_status": false,
    "message": "contract metadata found on Ethereum, AI analysis classified as legitimate"
  },
  "0x1234567890abcdef1234567890abcdef12345678": {
    "chain_id": 1,
    "contract_spam_status": true,
    "message": "contract metadata found on Ethereum, AI analysis classified as spam"
  }
}
```

#### Chain-Specific Error Handling
```json
{
  "error": "unsupported chain: chain 5 (Goerli Testnet) is planned for future implementation"
}
```

## Local Setup

Follow these steps to get the NFT API running locally with Docker:

### 1. Create Development Configuration

First, create a development configuration file based on the example:

```bash
cp config.example.json config.development.json
```

Edit `config.development.json` and update the API credentials. Ensure that you enable the external API needed for your use case, for example, `{external_apis.pinax.enabled: true}`.


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

# Test the contract status endpoint (defaults to Ethereum)
just local-test-status

# Test with custom contract address on Ethereum
just local-test-status "0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d" "1"

# Test with multiple addresses on Polygon (chain ID 137)
just local-test-status "0x1234567890abcdef1234567890abcdef12345678,0xabcdefabcdefabcdefabcdefabcdefabcdefabcd" "137"

# Test single address on Base (chain ID 8453)
just local-test-status "0x60e4d786628fea6478f785a6d7e704777c86a7c6" "8453"

# Test Avalanche C-Chain (chain ID 43114)
just local-test-status "0x071126cbec1c5562530ab85fd80dd3e3a42a70b8" "43114"

# Test Arbitrum One (chain ID 42161)
just local-test-status "0x32400084c286cf3e17e7b677ea9583e60a000324" "42161"
```

Expected responses:
- **Health**: JSON with server status
- **Contract Status**: JSON with chain-aware contract analysis results and detailed messages

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

2. **Run with default configuration:**
```bash
cargo run
```

3. **Run with custom configuration:**
```bash
export SERVER__PORT=8080
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

First, create a production configuration file based on the example:

```bash
cp config.example.json config.production.json
```

Edit `config.production.json` and update the API credentials. Ensure that you enable the external API needed for your use case, for example, `{external_apis.pinax.enabled: true}`.

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
4. **Environment Variables** - Runtime configuration with `SERVER__` prefix (use double underscores `__` between all keys)

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
| `spam_predictor.openai_api_key` | String | - | OpenAI API key for GPT model access (required) |
| `spam_predictor.openai_base_url` | String | `https://api.openai.com/v1` | OpenAI API base URL (optional) |
| `spam_predictor.openai_organization_id` | String | - | OpenAI organization ID (optional) |
| `spam_predictor.timeout_seconds` | Integer | `30` | OpenAI API request timeout |
| `spam_predictor.max_tokens` | Integer | `10` | Maximum tokens for AI responses |
| `spam_predictor.temperature` | Float | `0.0` | AI model temperature (0.0-2.0) |
| `spam_predictor.model_registry_path` | String | `assets/configs/models.yaml` | Path to model configuration file |
| `spam_predictor.prompt_registry_path` | String | `assets/prompts/ft_prompt.json` | Path to prompt configuration file |
| `spam_predictor.cache_ttl_seconds` | Integer | `3600` | Cache TTL for predictions in seconds |
| `spam_predictor.max_cache_size` | Integer | `10000` | Maximum number of cached predictions |
| `rate_limiting.enabled` | Boolean | `true` | Enable rate limiting |
| `rate_limiting.requests_per_minute` | Integer | `60` | Maximum requests per IP per minute |
| `chains.{chain_id}.enabled` | Boolean | `true` | Enable/disable specific blockchain chain |
| `chains.{chain_id}.moralis.timeout_seconds` | Integer | `30-45` | Chain-specific Moralis timeout (varies by chain) |
| `chains.{chain_id}.pinax.db_name` | String | - | Chain-specific Pinax database name |
| `extensions` | Object | `{}` | Additional configuration parameters |

### Prometheus Metrics

The server exposes Prometheus metrics for monitoring on a dedicated HTTP port.

- Endpoint path: configurable via `metrics.endpoint_path` (default: `/metrics`)
- Listening port: configurable via `metrics.port` (default: `9102`)



### Configuration Methods

#### 1. Environment Variables

Set environment variables with the `SERVER__` prefix (use double underscores `__` between all keys).

```bash
# Basic server configuration (top-level keys)
export SERVER__HOST=0.0.0.0
export SERVER__PORT=8080
export SERVER__TIMEOUT_SECONDS=60
export ENVIRONMENT=production

# Metrics configuration
export SERVER__METRICS__ENDPOINT_PATH=/metrics
export SERVER__METRICS__PORT=9102

# External API configuration
export SERVER__EXTERNAL_APIS__MORALIS__ENABLED=true
export SERVER__EXTERNAL_APIS__MORALIS__API_KEY=your_moralis_api_key
export SERVER__EXTERNAL_APIS__PINAX__ENABLED=true
export SERVER__EXTERNAL_APIS__PINAX__API_USER=your_pinax_username
export SERVER__EXTERNAL_APIS__PINAX__API_AUTH=your_pinax_auth_token

# AI Spam Predictor configuration (top-level `spam_predictor`)
export SERVER__SPAM_PREDICTOR__OPENAI_API_KEY=sk-your-openai-api-key-here
export SERVER__SPAM_PREDICTOR__OPENAI_BASE_URL=https://api.openai.com/v1
export SERVER__SPAM_PREDICTOR__OPENAI_ORGANIZATION_ID=your-org-id
export SERVER__SPAM_PREDICTOR__TIMEOUT_SECONDS=30
export SERVER__SPAM_PREDICTOR__MAX_TOKENS=10
export SERVER__SPAM_PREDICTOR__TEMPERATURE=0.0

# Rate limiting
export SERVER__RATE_LIMITING__REQUESTS_PER_MINUTE=100

# Chain-specific configuration
export SERVER__CHAINS__1__ENABLED=true
export SERVER__CHAINS__1__MORALIS__TIMEOUT_SECONDS=45
export SERVER__CHAINS__1__PINAX__DB_NAME="mainnet:evm-nft-tokens@v0.6.2"

export SERVER__CHAINS__137__ENABLED=true
export SERVER__CHAINS__137__MORALIS__TIMEOUT_SECONDS=45
export SERVER__CHAINS__137__PINAX__DB_NAME="matic:evm-nft-tokens@v0.5.1"

export SERVER__CHAINS__8453__ENABLED=true
export SERVER__CHAINS__8453__MORALIS__TIMEOUT_SECONDS=30
export SERVER__CHAINS__8453__PINAX__DB_NAME="base:evm-nft-tokens@v0.5.1"
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
      "base_url": "https://deep-index.moralis.io/api/v2",
      "api_key": "your_moralis_api_key",
      "timeout_seconds": 30,
      "health_check_timeout_seconds": 5,
      "max_retries": 3,
      "enabled": true
    },
    "pinax": {
      "endpoint": "https://api.pinax.network/sql",
      "api_user": "your_pinax_username",
      "api_auth": "your_pinax_auth_token",
      "db_name": "mainnet:evm-nft-tokens@v0.6.2",
      "timeout_seconds": 30,
      "health_check_timeout_seconds": 5,
      "max_retries": 3,
      "enabled": true
    }
  },
  "spam_predictor": {
    "openai_api_key": "sk-your-openai-api-key-here",
    "openai_base_url": "https://api.openai.com/v1",
    "openai_organization_id": "your-org-id",
    "timeout_seconds": 30,
    "max_tokens": 10,
    "temperature": 0.0,
    "model_registry_path": "assets/configs/models.yaml",
    "prompt_registry_path": "assets/prompts/ft_prompt.json",
    "cache_ttl_seconds": 3600,
    "max_cache_size": 10000
  },
  "rate_limiting": {
    "enabled": true,
    "requests_per_minute": 60
  },
  "chains": {
    "1": {
      "enabled": true,
      "moralis": {
        "timeout_seconds": 45
      },
      "pinax": {
        "db_name": "mainnet:evm-nft-tokens@v0.6.2"
      }
    },
    "137": {
      "enabled": true,
      "moralis": {
        "timeout_seconds": 45
      },
      "pinax": {
        "db_name": "matic:evm-nft-tokens@v0.5.1"
      }
    },
    "8453": {
      "enabled": true,
      "moralis": {
        "timeout_seconds": 30
      },
      "pinax": {
        "db_name": "base:evm-nft-tokens@v0.5.1"
      }
    },
    "43114": {
      "enabled": true,
      "moralis": {
        "timeout_seconds": 30
      },
      "pinax": {
        "db_name": "avalanche:evm-nft-tokens@v0.5.1"
      }
    },
    "42161": {
      "enabled": true,
      "moralis": {
        "timeout_seconds": 30
      },
      "pinax": {
        "db_name": "arbitrum-one:evm-nft-tokens@v0.5.1"
      }
    }
  },
  "extensions": {}
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
4. `SERVER__PORT` environment variable: `9000`

The final port will be `9000` (environment variable takes highest precedence).

## Blockchain Data Provider Setup

The NFT API requires external blockchain data providers to fetch NFT metadata and analytics. All supported chains now have full production support with both Moralis and Pinax providers available.

### Supported Chains and Requirements

| Chain | Chain ID | Status | Moralis API Key | Pinax Access | Production Ready |
|-------|----------|--------|-----------------|--------------|------------------|
| Ethereum | 1 | ✅ Full Support | Required | Required | ✅ |
| Polygon | 137 | ✅ Full Support | Required | Required | ✅ |
| Base | 8453 | ✅ Full Support | Required | Required | ✅ |
| Avalanche | 43114 | ✅ Full Support | Required | Required | ✅ |
| Arbitrum One | 42161 | ✅ Full Support | Required | Required | ✅ |

### Quick Setup

1. **Get Moralis API Key**:
   ```bash
   # Visit https://admin.moralis.io/register
   # Create a new project and get your API key
   export SERVER__EXTERNAL_APIS__MORALIS__API_KEY=your-moralis-api-key
   export SERVER__EXTERNAL_APIS__MORALIS__ENABLED=true
   ```

2. **Get Pinax Access**:
   ```bash
   # Visit https://pinax.network for access
   # Configure database access credentials
   export SERVER__EXTERNAL_APIS__PINAX__API_USER=your-username
   export SERVER__EXTERNAL_APIS__PINAX__API_AUTH=your-auth-token
   export SERVER__EXTERNAL_APIS__PINAX__ENABLED=true
   ```

3. **Configure Chain-Specific Settings**:
   ```bash
   # All chains are enabled by default in config.example.json
   # Timeouts are optimized per chain (30-45 seconds)
   # Database names are pre-configured for each chain
   ```

### Testing Multi-Chain Support

Test all supported chains with real contract addresses:

```bash
# Test individual chains
just local-test-status-ethereum       # Ethereum mainnet
just local-test-status-polygon        # Polygon
just local-test-status-base           # Base
just local-test-status-avalanche      # Avalanche
just local-test-status-arbitrum       # Arbitrum One

# Test all chains sequentially
just local-test-status-all-chains
```

### API Key Security

- **Never commit API keys** to version control
- Use environment variables for production deployment
- Rotate keys regularly according to provider recommendations
- Monitor API usage and rate limits through provider dashboards

## AI Spam Predictor Setup

The NFT API includes an AI-powered spam prediction system using OpenAI's fine-tuned GPT models. This feature analyzes contract metadata to classify contracts as spam or legitimate.

### Prerequisites

1. **OpenAI API Key**: You need a valid OpenAI API key with access to fine-tuned models
2. **Model Configuration**: Proper model registry and prompt configuration files
3. **Asset Files**: Required configuration files in the `assets/` directory

### Quick Setup

1. **Get OpenAI API Key**:
   - Visit [OpenAI Platform](https://platform.openai.com/api-keys)
   - Create a new API key with the format `sk-...`
   - Store securely (never commit to version control)

2. **Enable Spam Predictor**:
   ```bash
   export SERVER__SPAM_PREDICTOR__OPENAI_API_KEY=sk-your-actual-openai-key
   ```

3. **Create Asset Files** (if not present):
   ```bash
   mkdir -p assets/configs assets/prompts
   # Add your model registry (models.yaml) and prompt configuration (ft_prompt.json)
   ```

### Configuration Options

#### Required Settings
- `openai_api_key`: Your OpenAI API key (starts with `sk-`)
- `enabled`: Set to `true` to activate spam prediction

#### Optional Settings
- `openai_base_url`: Custom OpenAI API endpoint (default: `https://api.openai.com/v1`)
- `openai_organization_id`: OpenAI organization ID for billing/usage tracking
- `timeout_seconds`: Request timeout in seconds (default: `30`)
- `max_tokens`: Maximum AI response tokens (default: `10`)
- `temperature`: AI model creativity level, 0.0-2.0 (default: `0.0` for consistent results)
- `model_registry_path`: Path to model configuration file (default: `assets/configs/models.yaml`)
- `prompt_registry_path`: Path to prompt configuration file (default: `assets/prompts/ft_prompt.json`)

### Security Considerations

- **Never commit API keys**: Use environment variables or secure secret stores
- **Validate keys**: Ensure API key format starts with `sk-` and is properly scoped
- **Monitor usage**: Track OpenAI API calls and costs in production
- **Rate limiting**: Built-in request throttling prevents API abuse

### Troubleshooting

**SpamPredictor disabled**: If spam prediction is not working, check:
1. API key is valid and properly formatted
2. Asset files exist at configured paths
3. OpenAI API has sufficient quota/credits
4. Network connectivity to OpenAI API

**Health check failures**: The `/health` endpoint includes spam predictor status:
```json
{
  "status": "Up",
  "api_clients": {
    "spam_predictor": "Up"  // or "Down" if issues exist
  }
}
```

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
# Environment Management (with automatic switching)
just docker-dev-up          # Switch to dev environment (stops prod, starts dev)
just docker-dev-up-bg       # Switch to dev environment in background
just docker-prod-up         # Switch to prod environment (stops dev, starts prod)
just docker-prod-up-bg      # Switch to prod environment in background

# Environment Control
just docker-dev-down        # Stop dev environment only
just docker-prod-down       # Stop prod environment only
just docker-clean           # Clean all environments and containers

# Image Building
just docker-build           # Build production image (cached)
just docker-build-dev       # Build development image (cached)
just docker-rebuild-prod    # Rebuild production image without cache
just docker-rebuild-dev     # Rebuild development image without cache
```

**Local Testing:**
```bash
just local-test-health                                    # Test health endpoint
just local-test-status                                    # Test contract status with defaults
just local-test-status "0x123..."                       # Test with custom address
just local-test-status "0x123...,0x456..." "137"        # Test multiple addresses on Polygon
just local-test-status "0x123..." "42161"               # Test single address on Arbitrum

# Chain-specific testing with real contracts
just local-test-status-ethereum                          # Test Ethereum mainnet (chain 1)
just local-test-status-polygon                           # Test Polygon (chain 137)
just local-test-status-base                              # Test Base (chain 8453)
just local-test-status-avalanche                         # Test Avalanche (chain 43114)
just local-test-status-arbitrum                          # Test Arbitrum One (chain 42161)
just local-test-status-all-chains                        # Test all supported chains sequentially
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
- **Configuration**: Can be disabled in production by setting `SERVER__EXTENSIONS__ENABLE_SWAGGER=false`

### OpenAPI Specification

The complete OpenAPI 3.0 specification is available in JSON format:

- **URL**: `http://localhost:3000/api-doc/openapi.json`
- **Usage**: API client generation, integration testing, documentation tooling
- **Content**: Complete endpoint documentation with request/response schemas

## Production Deployment

### Security Considerations

- **API Credentials**: Set real Moralis, Pinax, and OpenAI API keys, remove placeholder values
- **AI Security**: Monitor OpenAI API usage and costs, validate API key scoping, never commit AI credentials
- **Rate Limiting**: Always enabled in production (validates to prevent DoS attacks)
- **HTTPS**: Enforce HTTPS URLs for external API endpoints (including OpenAI API)
- **Host Binding**: Consider firewall/proxy configuration when binding to `0.0.0.0`
- **Secrets Management**: Use environment variables or secure secret stores, never commit API keys

### Health Monitoring

The `/health` endpoint provides comprehensive multi-chain service health information:

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "environment": "production",
  "timestamp": "2025-01-22T10:30:00Z",
  "external_apis": {
    "moralis": {
      "status": "healthy",
      "chains": {
        "1": {"name": "Ethereum", "status": "operational"},
        "137": {"name": "Polygon", "status": "operational"},
        "8453": {"name": "Base", "status": "operational"},
        "43114": {"name": "Avalanche", "status": "operational"},
        "42161": {"name": "Arbitrum", "status": "operational"}
      }
    },
    "pinax": {
      "status": "healthy",
      "supported_chains": [1, 137, 8453, 43114, 42161]
    }
  },
  "spam_predictor": {
    "status": "healthy",
    "model_status": "operational"
  }
}
```


## License

Licensed under the Apache License, Version 2.0. See the LICENSE file for details.
