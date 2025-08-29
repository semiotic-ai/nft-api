// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! NFT API Server
//!
//! A blockchain token management API service.

use anyhow::Result;
use api::{Server, ServerConfig, ShutdownConfig};
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting NFT API server with coordinated shutdown support");

    let config = ServerConfig::from_env()?;

    let shutdown_config = ShutdownConfig::default();

    let server = Server::new(config, shutdown_config).await?;

    // NOTE: the `#[tokio::main]` task does not run a worker future, we must spawn
    tokio::spawn(async move { server.run().await }).await??;

    Ok(())
}
