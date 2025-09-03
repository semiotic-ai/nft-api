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
            Self::Polygon | Self::Ethereum => ChainImplementationStatus::Full,
            Self::Base => ChainImplementationStatus::Partial,
            Self::Avalanche | Self::Arbitrum => ChainImplementationStatus::Planned,
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

impl fmt::Display for ChainImplementationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "Full"),
            Self::Partial => write!(f, "Partial"),
            Self::Planned => write!(f, "Planned"),
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
    fn implementation_status() {
        assert_eq!(
            ChainId::Polygon.implementation_status(),
            ChainImplementationStatus::Full
        );
        assert_eq!(
            ChainId::Ethereum.implementation_status(),
            ChainImplementationStatus::Full
        );
        assert_eq!(
            ChainId::Base.implementation_status(),
            ChainImplementationStatus::Partial
        );
        assert_eq!(
            ChainId::Avalanche.implementation_status(),
            ChainImplementationStatus::Planned
        );
        assert_eq!(
            ChainId::Arbitrum.implementation_status(),
            ChainImplementationStatus::Planned
        );
    }

    #[test]
    fn is_fully_implemented() {
        assert!(ChainId::Polygon.is_fully_implemented());
        assert!(ChainId::Ethereum.is_fully_implemented());
        assert!(!ChainId::Base.is_fully_implemented());
        assert!(!ChainId::Avalanche.is_fully_implemented());
        assert!(!ChainId::Arbitrum.is_fully_implemented());
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
}
