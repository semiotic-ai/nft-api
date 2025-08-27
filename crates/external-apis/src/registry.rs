// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! API client registry for managing multiple external API providers
//!
//! This module provides orchestration and fallback logic for multiple API clients,
//! enabling resilient data retrieval with automatic failover.

use std::collections::HashMap;

use alloy_primitives::Address;
use api_client::{ApiClient, ContractMetadata, HealthStatus};
use shared_types::ChainId;
use tracing::{debug, info, warn};

use crate::{MoralisClient, PinaxClient};

/// Registry for managing API clients with fallback logic
#[derive(Debug)]
pub struct ApiRegistry {
    moralis_client: Option<MoralisClient>,
    pinax_client: Option<PinaxClient>,
}

/// Error type for registry operations
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum RegistryError {
    /// No healthy API clients available
    #[error("No healthy API clients available")]
    NoHealthyClients,

    /// All API clients failed
    #[error("All API clients failed: {details}")]
    AllClientsFailed { details: String },

    /// No clients registered
    #[error("No API clients registered")]
    NoClients,
}

impl Default for ApiRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRegistry {
    /// Create a new empty API registry
    pub fn new() -> Self {
        Self {
            moralis_client: None,
            pinax_client: None,
        }
    }

    /// Create a new API registry with the specified clients
    pub fn with_clients(
        moralis_client: Option<MoralisClient>,
        pinax_client: Option<PinaxClient>,
    ) -> Self {
        Self {
            moralis_client,
            pinax_client,
        }
    }

    /// Get contract metadata using the first available healthy client
    ///
    /// # Arguments
    ///
    /// * `address` - The contract address to get metadata for
    /// * `chain_id` - The blockchain chain to query
    ///
    /// # Returns
    ///
    /// * `Ok(Some(metadata))` if metadata was retrieved successfully
    /// * `Ok(None)` if no metadata was found (but clients were healthy)
    /// * `Err(error)` if all clients failed or no clients are available
    pub async fn get_contract_metadata(
        &self,
        address: Address,
        chain_id: ChainId,
    ) -> Result<Option<ContractMetadata>, RegistryError> {
        if self.moralis_client.is_none() && self.pinax_client.is_none() {
            return Err(RegistryError::NoClients);
        }

        let mut errors = Vec::new();

        if let Some(result) = self
            .try_moralis_metadata(address, chain_id, &mut errors)
            .await
        {
            return Ok(result);
        }

        if let Some(result) = self
            .try_pinax_metadata(address, chain_id, &mut errors)
            .await
        {
            return Ok(result);
        }

        if errors.is_empty() {
            debug!(
                "No metadata found for address {} on chain {} in any client",
                address,
                chain_id.name()
            );
            Ok(None)
        } else {
            Err(RegistryError::AllClientsFailed {
                details: errors.join("; "),
            })
        }
    }

    /// Try to get contract metadata from Moralis client
    async fn try_moralis_metadata(
        &self,
        address: Address,
        chain_id: ChainId,
        errors: &mut Vec<String>,
    ) -> Option<Option<ContractMetadata>> {
        let moralis_client = self.moralis_client.as_ref()?;

        match self.is_moralis_healthy().await {
            Ok(true) => {
                debug!(
                    "Trying healthy Moralis client for chain {} (Note: chain_id not yet supported by client)",
                    chain_id.name()
                );
                // TODO: Update client interface to accept chain_id parameter
                // Currently chain_id is used for logging/tracking but not passed to client
                match moralis_client.get_contract_metadata(address).await {
                    Ok(Some(metadata)) => {
                        info!("Successfully retrieved metadata from Moralis client");
                        Some(Some(metadata))
                    }
                    Ok(None) => {
                        debug!("No metadata found in Moralis client");
                        None
                    }
                    Err(e) => {
                        warn!("Moralis client failed: {}", e);
                        errors.push(format!("moralis: {e}"));
                        None
                    }
                }
            }
            Ok(false) => {
                debug!("Skipping unhealthy Moralis client");
                None
            }
            Err(e) => {
                warn!("Health check failed for Moralis client: {}", e);
                errors.push(format!("moralis health check: {e}"));
                None
            }
        }
    }

    /// Try to get contract metadata from Pinax client
    async fn try_pinax_metadata(
        &self,
        address: Address,
        chain_id: ChainId,
        errors: &mut Vec<String>,
    ) -> Option<Option<ContractMetadata>> {
        let pinax_client = self.pinax_client.as_ref()?;

        match self.is_pinax_healthy().await {
            Ok(true) => {
                debug!(
                    "Trying healthy Pinax client for chain {} (Note: chain_id not yet supported by client)",
                    chain_id.name()
                );
                // TODO: Update client interface to accept chain_id parameter
                // Currently chain_id is used for logging/tracking but not passed to client
                match pinax_client.get_contract_metadata(address).await {
                    Ok(Some(metadata)) => {
                        info!("Successfully retrieved metadata from Pinax client");
                        Some(Some(metadata))
                    }
                    Ok(None) => {
                        debug!("No metadata found in Pinax client");
                        None
                    }
                    Err(e) => {
                        warn!("Pinax client failed: {}", e);
                        errors.push(format!("pinax: {e}"));
                        None
                    }
                }
            }
            Ok(false) => {
                debug!("Skipping unhealthy Pinax client");
                None
            }
            Err(e) => {
                warn!("Health check failed for Pinax client: {}", e);
                errors.push(format!("pinax health check: {e}"));
                None
            }
        }
    }

    /// Check if Moralis client is healthy
    async fn is_moralis_healthy(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(moralis_client) = &self.moralis_client {
            match moralis_client.health_check().await {
                Ok(status) => {
                    let is_healthy = matches!(status, HealthStatus::Up);
                    Ok(is_healthy)
                }
                Err(e) => Err(Box::new(e)),
            }
        } else {
            Err("Moralis client not available".into())
        }
    }

    /// Check if Pinax client is healthy
    async fn is_pinax_healthy(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(pinax_client) = &self.pinax_client {
            match pinax_client.health_check().await {
                Ok(status) => {
                    let is_healthy = matches!(status, HealthStatus::Up);
                    Ok(is_healthy)
                }
                Err(e) => Err(Box::new(e)),
            }
        } else {
            Err("Pinax client not available".into())
        }
    }

    /// Get the overall health status of all registered clients
    ///
    /// Health checks are performed concurrently for better performance.
    pub async fn get_overall_health(&self) -> HashMap<String, HealthStatus> {
        let mut health_status = HashMap::new();

        let moralis_future = async {
            if let Some(moralis_client) = &self.moralis_client {
                match moralis_client.health_check().await {
                    Ok(status) => Some(("moralis".to_string(), status)),
                    Err(e) => {
                        let status = HealthStatus::Down {
                            reason: format!("Health check failed: {e}"),
                        };
                        Some(("moralis".to_string(), status))
                    }
                }
            } else {
                None
            }
        };

        let pinax_future = async {
            if let Some(pinax_client) = &self.pinax_client {
                match pinax_client.health_check().await {
                    Ok(status) => Some(("pinax".to_string(), status)),
                    Err(e) => {
                        let status = HealthStatus::Down {
                            reason: format!("Health check failed: {e}"),
                        };
                        Some(("pinax".to_string(), status))
                    }
                }
            } else {
                None
            }
        };

        let (moralis_result, pinax_result) = tokio::join!(moralis_future, pinax_future);

        if let Some((name, status)) = moralis_result {
            health_status.insert(name, status);
        }

        if let Some((name, status)) = pinax_result {
            health_status.insert(name, status);
        }

        health_status
    }

    /// Get the number of registered clients
    pub fn client_count(&self) -> usize {
        let mut count = 0;
        if self.moralis_client.is_some() {
            count += 1;
        }
        if self.pinax_client.is_some() {
            count += 1;
        }
        count
    }

    /// Get the names of all registered clients
    pub fn client_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.moralis_client.is_some() {
            names.push("moralis");
        }
        if self.pinax_client.is_some() {
            names.push("pinax");
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn registry_creation() {
        let registry = ApiRegistry::new();
        assert_eq!(registry.client_count(), 0);
        assert!(registry.client_names().is_empty());
    }

    #[tokio::test]
    async fn get_contract_metadata_no_clients() {
        let registry = ApiRegistry::new();
        let result = registry
            .get_contract_metadata(Address::ZERO, ChainId::Polygon)
            .await;
        assert!(matches!(result, Err(RegistryError::NoClients)));
    }

    #[test]
    fn registry_error_display() {
        let error = RegistryError::NoClients;
        assert_eq!(error.to_string(), "No API clients registered");

        let error = RegistryError::AllClientsFailed {
            details: "all failed".to_string(),
        };
        assert_eq!(error.to_string(), "All API clients failed: all failed");

        let error = RegistryError::NoHealthyClients;
        assert_eq!(error.to_string(), "No healthy API clients available");
    }
}
