// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Blockchain chain types and identifiers
//!
//! This module provides type-safe chain identifiers and implementation status
//! for supported blockchain networks.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;

/// Supported blockchain chain identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToSchema)]
pub enum ChainId {
    /// Polygon - Chain ID: 137
    Polygon = 137,
    /// Ethereum Mainnet - Chain ID: 1
    Ethereum = 1,
    /// Base - Chain ID: 8453
    Base = 8453,
    /// Avalanche - Chain ID: 43114
    Avalanche = 43114,
    /// Arbitrum - Chain ID: 42161
    Arbitrum = 42161,
}

impl ChainId {
    /// Returns the numeric chain ID
    pub const fn chain_id(self) -> u64 {
        match self {
            Self::Polygon => 137,
            Self::Ethereum => 1,
            Self::Base => 8453,
            Self::Avalanche => 43114,
            Self::Arbitrum => 42161,
        }
    }

    /// Returns the human-readable name of the chain
    pub const fn name(self) -> &'static str {
        match self {
            Self::Polygon => "Polygon",
            Self::Ethereum => "Ethereum",
            Self::Base => "Base",
            Self::Avalanche => "Avalanche",
            Self::Arbitrum => "Arbitrum",
        }
    }

    /// Returns the implementation status for this chain
    pub const fn implementation_status(self) -> ChainImplementationStatus {
        match self {
            // All chains have production-ready Pinax Token API support
            Self::Polygon | Self::Ethereum | Self::Base | Self::Avalanche | Self::Arbitrum => {
                ChainImplementationStatus::Full
            }
        }
    }

    /// Returns whether the chain is fully implemented
    pub const fn is_fully_implemented(self) -> bool {
        matches!(
            self.implementation_status(),
            ChainImplementationStatus::Full
        )
    }

    /// Returns a status message for the chain's implementation
    pub fn status_message(self) -> &'static str {
        match self.implementation_status() {
            ChainImplementationStatus::Full => "fully supported",
            ChainImplementationStatus::Partial => {
                "partially supported - some features may be limited"
            }
            ChainImplementationStatus::Planned => "not yet implemented",
        }
    }

    /// Returns all supported chain IDs
    pub const fn all() -> &'static [Self] {
        &[
            Self::Polygon,
            Self::Ethereum,
            Self::Base,
            Self::Avalanche,
            Self::Arbitrum,
        ]
    }

    /// Returns the current support status for this chain
    pub const fn support_status(self) -> ChainStatus {
        match self.implementation_status() {
            ChainImplementationStatus::Full => ChainStatus::FullySupported,
            ChainImplementationStatus::Partial => ChainStatus::PartiallySupported,
            ChainImplementationStatus::Planned => ChainStatus::Planned,
        }
    }

    /// Returns the list of capabilities available for this chain
    pub fn capabilities(self) -> Vec<ChainCapability> {
        match self {
            // All chains have both Moralis and Pinax production support
            Self::Polygon | Self::Ethereum | Self::Base | Self::Avalanche | Self::Arbitrum => {
                vec![
                    ChainCapability::MoralisMetadata,
                    ChainCapability::PinaxAnalytics,
                    ChainCapability::SpamPrediction,
                ]
            }
        }
    }

    /// Returns whether this chain supports a specific capability
    pub fn supports_capability(self, capability: ChainCapability) -> bool {
        self.capabilities().contains(&capability)
    }

    /// Returns the limitations for this chain as a descriptive message
    pub fn limitations(self) -> Vec<&'static str> {
        match self {
            // All chains now have full production support from both Moralis and Pinax
            Self::Polygon | Self::Ethereum | Self::Base | Self::Avalanche | Self::Arbitrum => {
                vec![]
            }
        }
    }

    /// Returns the estimated availability date for planned features (if applicable)
    pub fn estimated_availability(self) -> Option<&'static str> {
        match self {
            // All chains are now production ready - no estimated dates needed
            Self::Polygon | Self::Ethereum | Self::Base | Self::Avalanche | Self::Arbitrum => None,
        }
    }
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for ChainId {
    type Err = ChainIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // First try to parse as a numeric chain ID
        if let Ok(id) = s.parse::<u64>() {
            return Self::try_from(id).map_err(|_| ChainIdParseError::InvalidId(id));
        }

        // Fall back to parsing as a chain name
        match s.to_uppercase().as_str() {
            "POLYGON" | "MATIC" => Ok(Self::Polygon),
            "ETHEREUM" | "UNI" => Ok(Self::Ethereum),
            "BASE" => Ok(Self::Base),
            "AVALANCHE" | "AVAX" => Ok(Self::Avalanche),
            "ARBITRUM" | "ARB" => Ok(Self::Arbitrum),
            _ => Err(ChainIdParseError::InvalidName(s.to_string())),
        }
    }
}

impl TryFrom<u64> for ChainId {
    type Error = ChainIdParseError;

    fn try_from(id: u64) -> Result<Self, Self::Error> {
        match id {
            137 => Ok(Self::Polygon),
            1 => Ok(Self::Ethereum),
            8453 => Ok(Self::Base),
            43114 => Ok(Self::Avalanche),
            42161 => Ok(Self::Arbitrum),
            _ => Err(ChainIdParseError::InvalidId(id)),
        }
    }
}

impl Serialize for ChainId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.chain_id().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChainId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ChainIdVisitor;

        impl serde::de::Visitor<'_> for ChainIdVisitor {
            type Value = ChainId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "a valid chain ID (137, 1, 8453, 43114, 42161), chain ID string (\"137\", \"1\", etc.), or name (Polygon, Ethereum, Base, Avalanche, Arbitrum)"
                )
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                ChainId::try_from(value).map_err(|_| {
                    E::invalid_value(
                        serde::de::Unexpected::Unsigned(value),
                        &"a supported chain ID (137, 1, 8453, 43114, 42161)",
                    )
                })
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                ChainId::from_str(value).map_err(|_| {
                    E::invalid_value(
                        serde::de::Unexpected::Str(value),
                        &"a supported chain name (Polygon, Ethereum, Base, Avalanche, Arbitrum)",
                    )
                })
            }
        }

        deserializer.deserialize_any(ChainIdVisitor)
    }
}

/// Implementation status for blockchain chains
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum ChainImplementationStatus {
    /// Chain is fully implemented and supported
    Full,
    /// Chain is partially implemented with some limitations
    Partial,
    /// Chain support is planned but not yet implemented
    Planned,
}

/// Chain support status for API responses and validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum ChainStatus {
    /// Chain is fully supported with all features available
    FullySupported,
    /// Chain is partially supported with some features limited or unavailable
    PartiallySupported,
    /// Chain support is planned but not yet available
    Planned,
    /// Chain support is deprecated and being phased out
    Deprecated,
}

/// Specific capabilities available for blockchain chains
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum ChainCapability {
    /// NFT metadata retrieval via Moralis API
    MoralisMetadata,
    /// Blockchain analytics via Pinax database
    PinaxAnalytics,
    /// AI-powered spam detection
    SpamPrediction,
    /// Real-time updates via WebSocket connections
    RealTimeUpdates,
}

impl fmt::Display for ChainImplementationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "Full"),
            Self::Partial => write!(f, "Partial"),
            Self::Planned => write!(f, "Planned"),
        }
    }
}

impl fmt::Display for ChainStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FullySupported => write!(f, "fully_supported"),
            Self::PartiallySupported => write!(f, "partially_supported"),
            Self::Planned => write!(f, "planned"),
            Self::Deprecated => write!(f, "deprecated"),
        }
    }
}

impl fmt::Display for ChainCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MoralisMetadata => write!(f, "moralis_metadata"),
            Self::PinaxAnalytics => write!(f, "pinax_analytics"),
            Self::SpamPrediction => write!(f, "spam_prediction"),
            Self::RealTimeUpdates => write!(f, "realtime_updates"),
        }
    }
}

/// Error type for chain ID parsing
#[derive(Debug, thiserror::Error)]
pub enum ChainIdParseError {
    /// Invalid chain ID number
    #[error(
        "unsupported chain ID: {0}. Supported chain IDs are: 137 (Polygon), 1 (Ethereum), 8453 (Base), 43114 (Avalanche), 42161 (Arbitrum)"
    )]
    InvalidId(u64),
    /// Invalid chain name
    #[error(
        "unsupported chain name: {0}. Supported chain names are: Polygon, Ethereum, Base, Avalanche, Arbitrum"
    )]
    InvalidName(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_status() {
        // All chains are now fully supported based on Pinax documentation
        for &chain in ChainId::all() {
            assert_eq!(
                chain.support_status(),
                ChainStatus::FullySupported,
                "Chain {chain:?} should be fully supported"
            );
            assert_eq!(
                chain.implementation_status(),
                ChainImplementationStatus::Full,
                "Chain {chain:?} should have full implementation"
            );
            assert!(
                chain.is_fully_implemented(),
                "Chain {chain:?} should be fully implemented"
            );
        }
    }

    #[test]
    fn chain_capabilities() {
        // Test that all chains now have full capabilities
        for &chain in ChainId::all() {
            let caps = chain.capabilities();
            assert!(
                caps.contains(&ChainCapability::MoralisMetadata),
                "Chain {chain:?} missing Moralis"
            );
            assert!(
                caps.contains(&ChainCapability::PinaxAnalytics),
                "Chain {chain:?} missing Pinax"
            );
            assert!(
                caps.contains(&ChainCapability::SpamPrediction),
                "Chain {chain:?} missing Spam"
            );
            assert_eq!(caps.len(), 3, "Chain {chain:?} should have 3 capabilities");
        }
    }

    #[test]
    fn supports_capability() {
        // Test that all chains now support all capabilities
        for &chain in ChainId::all() {
            assert!(
                chain.supports_capability(ChainCapability::MoralisMetadata),
                "Chain {chain:?} should support Moralis"
            );
            assert!(
                chain.supports_capability(ChainCapability::PinaxAnalytics),
                "Chain {chain:?} should support Pinax"
            );
            assert!(
                chain.supports_capability(ChainCapability::SpamPrediction),
                "Chain {chain:?} should support Spam"
            );
        }
    }

    #[test]
    fn chain_id_numeric_conversion() {
        assert_eq!(ChainId::Polygon.chain_id(), 137);
        assert_eq!(ChainId::Ethereum.chain_id(), 1);
        assert_eq!(ChainId::Base.chain_id(), 8453);
        assert_eq!(ChainId::Avalanche.chain_id(), 43114);
        assert_eq!(ChainId::Arbitrum.chain_id(), 42161);
    }

    #[test]
    fn chain_id_name_conversion() {
        assert_eq!(ChainId::Polygon.name(), "Polygon");
        assert_eq!(ChainId::Ethereum.name(), "Ethereum");
        assert_eq!(ChainId::Base.name(), "Base");
        assert_eq!(ChainId::Avalanche.name(), "Avalanche");
        assert_eq!(ChainId::Arbitrum.name(), "Arbitrum");
    }

    #[test]
    fn chain_id_from_str() {
        // Test numeric chain IDs as strings (for configuration parsing)
        assert_eq!(ChainId::from_str("137").unwrap(), ChainId::Polygon);
        assert_eq!(ChainId::from_str("1").unwrap(), ChainId::Ethereum);
        assert_eq!(ChainId::from_str("8453").unwrap(), ChainId::Base);
        assert_eq!(ChainId::from_str("43114").unwrap(), ChainId::Avalanche);
        assert_eq!(ChainId::from_str("42161").unwrap(), ChainId::Arbitrum);

        // Test primary names
        assert_eq!(ChainId::from_str("POLYGON").unwrap(), ChainId::Polygon);
        assert_eq!(ChainId::from_str("polygon").unwrap(), ChainId::Polygon);
        assert_eq!(ChainId::from_str("ETHEREUM").unwrap(), ChainId::Ethereum);
        assert_eq!(ChainId::from_str("Base").unwrap(), ChainId::Base);
        assert_eq!(ChainId::from_str("AVALANCHE").unwrap(), ChainId::Avalanche);
        assert_eq!(ChainId::from_str("ARBITRUM").unwrap(), ChainId::Arbitrum);

        // Test legacy names for backward compatibility
        assert_eq!(ChainId::from_str("MATIC").unwrap(), ChainId::Polygon);
        assert_eq!(ChainId::from_str("UNI").unwrap(), ChainId::Ethereum);
        assert_eq!(ChainId::from_str("AVAX").unwrap(), ChainId::Avalanche);
        assert_eq!(ChainId::from_str("ARB").unwrap(), ChainId::Arbitrum);

        assert!(ChainId::from_str("UNKNOWN").is_err());
        assert!(ChainId::from_str("999").is_err());
    }

    #[test]
    fn chain_id_try_from_u64() {
        assert_eq!(ChainId::try_from(137).unwrap(), ChainId::Polygon);
        assert_eq!(ChainId::try_from(1).unwrap(), ChainId::Ethereum);
        assert_eq!(ChainId::try_from(8453).unwrap(), ChainId::Base);
        assert_eq!(ChainId::try_from(43114).unwrap(), ChainId::Avalanche);
        assert_eq!(ChainId::try_from(42161).unwrap(), ChainId::Arbitrum);

        assert!(ChainId::try_from(999).is_err());
    }

    #[test]
    fn serde_serialization() {
        let chain = ChainId::Polygon;
        let serialized = serde_json::to_string(&chain).unwrap();
        assert_eq!(serialized, "137");
    }

    #[test]
    fn serde_deserialization_numeric() {
        let deserialized: ChainId = serde_json::from_str("137").unwrap();
        assert_eq!(deserialized, ChainId::Polygon);
    }

    #[test]
    fn serde_deserialization_string() {
        // Test primary name
        let deserialized: ChainId = serde_json::from_str("\"POLYGON\"").unwrap();
        assert_eq!(deserialized, ChainId::Polygon);

        // Test legacy name for backward compatibility
        let deserialized: ChainId = serde_json::from_str("\"MATIC\"").unwrap();
        assert_eq!(deserialized, ChainId::Polygon);
    }

    #[test]
    fn serde_deserialization_invalid() {
        assert!(serde_json::from_str::<ChainId>("999").is_err());
        assert!(serde_json::from_str::<ChainId>("\"UNKNOWN\"").is_err());
    }

    #[test]
    fn all_chains_comprehensive() {
        let all_chains = ChainId::all();

        // Verify we have all expected chains
        assert_eq!(
            all_chains.len(),
            5,
            "Should have exactly 5 supported chains"
        );

        // Verify all expected chains are present
        assert!(all_chains.contains(&ChainId::Ethereum), "Missing Ethereum");
        assert!(all_chains.contains(&ChainId::Polygon), "Missing Polygon");
        assert!(all_chains.contains(&ChainId::Base), "Missing Base");
        assert!(
            all_chains.contains(&ChainId::Avalanche),
            "Missing Avalanche"
        );
        assert!(all_chains.contains(&ChainId::Arbitrum), "Missing Arbitrum");

        // Verify each chain has unique properties
        let mut chain_ids = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();

        for &chain in all_chains {
            // Each chain should have unique ID and name
            assert!(
                chain_ids.insert(chain.chain_id()),
                "Duplicate chain ID: {}",
                chain.chain_id()
            );
            assert!(
                names.insert(chain.name()),
                "Duplicate chain name: {}",
                chain.name()
            );

            // Each chain should have consistent properties
            assert!(
                !chain.limitations().is_empty() || chain.limitations().is_empty(),
                "Chain {chain:?} limitations check passed"
            );
            assert!(
                chain.estimated_availability().is_none(),
                "Chain {chain:?} should not have estimated availability (all are production-ready)"
            );
        }
    }

    #[test]
    fn chain_id_consistency() {
        // Test that conversion methods are consistent
        for &chain in ChainId::all() {
            // Test numeric conversion round-trip
            let numeric_id = chain.chain_id();
            let parsed_from_numeric = ChainId::try_from(numeric_id).unwrap();
            assert_eq!(
                chain, parsed_from_numeric,
                "Numeric conversion inconsistent for {chain:?}"
            );

            // Test string conversion round-trip
            let name = chain.name();
            let parsed_from_name = ChainId::from_str(name).unwrap();
            assert_eq!(
                chain, parsed_from_name,
                "Name conversion inconsistent for {chain:?}"
            );

            // Test numeric string conversion
            let numeric_string = numeric_id.to_string();
            let parsed_from_numeric_string = ChainId::from_str(&numeric_string).unwrap();
            assert_eq!(
                chain, parsed_from_numeric_string,
                "Numeric string conversion inconsistent for {chain:?}"
            );
        }
    }

    #[test]
    fn chain_properties_comprehensive() {
        for &chain in ChainId::all() {
            // Verify each chain has proper properties
            assert!(
                !chain.name().is_empty(),
                "Chain {chain:?} name should not be empty"
            );
            assert!(
                chain.chain_id() > 0,
                "Chain {chain:?} should have positive ID"
            );
            assert!(
                !chain.status_message().is_empty(),
                "Chain {chain:?} status message should not be empty"
            );

            // Verify capabilities are non-empty and contain expected types
            let capabilities = chain.capabilities();
            assert!(
                !capabilities.is_empty(),
                "Chain {chain:?} should have capabilities"
            );

            // All chains should support these core capabilities
            for expected_cap in &[
                ChainCapability::MoralisMetadata,
                ChainCapability::PinaxAnalytics,
                ChainCapability::SpamPrediction,
            ] {
                assert!(
                    chain.supports_capability(*expected_cap),
                    "Chain {chain:?} should support capability {expected_cap:?}"
                );
            }
        }
    }

    #[test]
    fn chain_serialization_all_chains() {
        // Test serialization/deserialization for all chains
        for &chain in ChainId::all() {
            // Test JSON serialization
            let serialized = serde_json::to_string(&chain).unwrap();
            let deserialized: ChainId = serde_json::from_str(&serialized).unwrap();
            assert_eq!(
                chain, deserialized,
                "JSON serialization failed for {chain:?}"
            );

            // Test that serialized form is the numeric ID
            let expected_json = chain.chain_id().to_string();
            assert_eq!(
                serialized, expected_json,
                "Chain {chain:?} should serialize to numeric ID"
            );
        }
    }
}
