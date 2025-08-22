// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Common data types for API client contracts and metadata

use std::collections::HashMap;

use alloy_primitives::Address;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata for a smart contract
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractMetadata {
    /// Contract address
    pub address: Address,
    /// Contract name (if available)
    pub name: Option<String>,
    /// Contract symbol (if available)  
    pub symbol: Option<String>,
    /// Total supply of tokens (if applicable)
    pub total_supply: Option<String>,
    /// Number of unique token holders
    pub holder_count: Option<u64>,
    /// Number of transactions involving this contract
    pub transaction_count: Option<u64>,
    /// Block number when contract was created
    pub creation_block: Option<u64>,
    /// Timestamp when contract was created
    pub creation_timestamp: Option<DateTime<Utc>>,
    /// Address that deployed this contract
    pub creator_address: Option<Address>,
    /// Whether contract source code is verified
    pub is_verified: Option<bool>,
    /// Contract type (ERC-20, ERC-721, ERC-1155, etc.)
    pub contract_type: Option<ContractType>,
    /// Additional metadata fields specific to different APIs
    pub additional_data: HashMap<String, serde_json::Value>,
}

/// Type of smart contract
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum ContractType {
    /// ERC-20 fungible token
    Erc20,
    /// ERC-721 non-fungible token
    Erc721,
    /// ERC-1155 multi-token
    Erc1155,
    /// Generic contract (not a standard token)
    Contract,
    /// Unknown/unidentified contract type
    #[default]
    Unknown,
}

/// Spam analysis result for a contract
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpamAnalysis {
    /// Whether the contract is considered spam
    pub is_spam: bool,
    /// Reasons for spam classification
    pub reasons: Vec<String>,
    /// Source of the spam analysis
    pub source: String,
    /// When the analysis was performed
    pub analyzed_at: DateTime<Utc>,
}

/// Combined contract information from multiple sources
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInfo {
    /// Basic contract metadata
    pub metadata: ContractMetadata,
    /// Spam analysis result (if available)
    pub spam_analysis: Option<SpamAnalysis>,
    /// Which API sources provided data
    pub data_sources: Vec<String>,
    /// When this information was last updated
    pub last_updated: DateTime<Utc>,
}

impl ContractMetadata {
    /// Create minimal contract metadata with just an address
    pub fn minimal(address: Address) -> Self {
        Self {
            address,
            name: None,
            symbol: None,
            total_supply: None,
            holder_count: None,
            transaction_count: None,
            creation_block: None,
            creation_timestamp: None,
            creator_address: None,
            is_verified: None,
            contract_type: None,
            additional_data: HashMap::new(),
        }
    }

    /// Check if this contract appears to be an NFT (ERC-721 or ERC-1155)
    pub fn is_nft(&self) -> bool {
        matches!(
            self.contract_type,
            Some(ContractType::Erc721 | ContractType::Erc1155)
        )
    }

    /// Check if this contract appears to be a fungible token (ERC-20)
    pub fn is_fungible_token(&self) -> bool {
        matches!(self.contract_type, Some(ContractType::Erc20))
    }

    /// Get a display name for the contract (name, symbol, or shortened address)
    pub fn display_name(&self) -> String {
        if let Some(ref name) = self.name {
            if let Some(ref symbol) = self.symbol {
                format!("{name} ({symbol})")
            } else {
                name.clone()
            }
        } else if let Some(ref symbol) = self.symbol {
            symbol.clone()
        } else {
            let addr_str = format!("{:?}", self.address);
            if addr_str.len() >= 12 {
                format!("{}...{}", &addr_str[..6], &addr_str[addr_str.len() - 4..])
            } else {
                addr_str
            }
        }
    }
}

impl ContractInfo {
    /// Create new contract info with minimal metadata
    pub fn new(address: Address, source: String) -> Self {
        Self {
            metadata: ContractMetadata::minimal(address),
            spam_analysis: None,
            data_sources: vec![source],
            last_updated: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn contract_metadata_minimal() {
        let address = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        let metadata = ContractMetadata::minimal(address);

        assert_eq!(metadata.address, address);
        assert!(metadata.name.is_none());
        assert!(metadata.symbol.is_none());
        assert!(metadata.additional_data.is_empty());
    }

    #[test]
    fn contract_type_identification() {
        let mut metadata = ContractMetadata::minimal(Address::ZERO);

        metadata.contract_type = Some(ContractType::Erc721);
        assert!(metadata.is_nft());
        assert!(!metadata.is_fungible_token());

        metadata.contract_type = Some(ContractType::Erc1155);
        assert!(metadata.is_nft());
        assert!(!metadata.is_fungible_token());

        metadata.contract_type = Some(ContractType::Erc20);
        assert!(!metadata.is_nft());
        assert!(metadata.is_fungible_token());
    }

    #[test]
    fn contract_display_name() {
        let mut metadata = ContractMetadata::minimal(Address::ZERO);

        metadata.name = Some("Test Token".to_string());
        metadata.symbol = Some("TEST".to_string());
        assert_eq!(metadata.display_name(), "Test Token (TEST)");

        metadata.symbol = None;
        assert_eq!(metadata.display_name(), "Test Token");

        metadata.name = None;
        metadata.symbol = Some("TEST".to_string());
        assert_eq!(metadata.display_name(), "TEST");

        metadata.symbol = None;
        let display = metadata.display_name();
        assert!(display.contains("0x0000"));
        assert!(display.contains("..."));
    }

    #[test]
    fn contract_type_default() {
        assert_eq!(ContractType::default(), ContractType::Unknown);
    }
}
