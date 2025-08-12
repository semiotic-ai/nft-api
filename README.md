<!--
SPDX-FileCopyrightText: 2025 Semiotic Labs

SPDX-License-Identifier: Apache-2.0
-->

# nft-api

A secure NFT API service built with Rust, designed for blockchain token management with strict security and code quality standards.

## API Endpoints

### Health Check
- **GET** `/health` - Server health status

### Contract Analysis (v1)
- **POST** `/v1/contract/status` - Analyze contract addresses for spam classification

#### Contract Status Request
```json
{
  "addresses": [
    "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
    "0x1234567890123456789012345678901234567890"
  ]
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

## Quick Start

1. **Build the project:**
   ```bash
   cargo build --release
   ```

2. **Run with default configuration:**
   ```bash
   cargo run
   ```

3. **Run with custom configuration:**
   ```bash
   export SERVER_PORT=8080
   export ENVIRONMENT=production
   cargo run
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
| `extensions` | Object | `{}` | Additional configuration parameters |

### Configuration Methods

#### 1. Environment Variables (Recommended)

Set environment variables with the `SERVER_` prefix:

```bash
export SERVER_HOST=0.0.0.0
export SERVER_PORT=8080
export SERVER_TIMEOUT_SECONDS=60
export ENVIRONMENT=production
```

#### 2. Configuration Files

Create configuration files in the project root:

**config.json:**
```json
{
  "host": "0.0.0.0",
  "port": 8080,
  "timeout_seconds": 60,
  "environment": "production",
  "extensions": {
    "log_level": "info",
    "database_url": "postgresql://localhost/nft_api"
  }
}
```

#### 3. Environment-Specific Configuration

Create environment-specific configuration files that are loaded based on the `ENVIRONMENT` variable:

- `config.production.toml` - Production settings
- `config.development.toml` - Development settings
- `config.testing.toml` - Testing settings

### Configuration Precedence

Configuration values are loaded in hierarchical order. For example, if you have:

1. Default port: `3000`
2. `config.json` port: `8080`
3. `config.production.json` port: `443`
4. `SERVER_PORT` environment variable: `9000`

The final port will be `9000` (environment variable takes highest precedence).

### Validation

All configuration is validated at startup:

- Port must be greater than 0 (except in testing environment)
- Timeout must be between 1-300 seconds
- Host must be a valid IP address
- Environment must be one of: `production`, `development`, `testing`

### Development

**Running Tests:**
```bash
cargo test --workspace --all-features
```

**Code Quality Checks:**
```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Security Scanning:**
```bash
cargo audit
cargo deny check
```

## License

Licensed under the Apache License, Version 2.0. See the LICENSE file for details.