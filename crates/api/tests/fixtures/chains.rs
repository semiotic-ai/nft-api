// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

//! Chain-specific test fixtures
//!
//! Provides test data and fixtures for each supported blockchain chain.

use std::collections::HashMap;

use alloy_primitives::Address;
use api_client::{ContractMetadata, ContractType};
use serde_json::json;
use shared_types::{ChainCapability, ChainId};

/// Chain-specific test fixture containing test data for a blockchain chain
#[derive(Debug, Clone)]
pub struct ChainTestFixture {
    /// The blockchain chain identifier
    pub chain_id: ChainId,
    /// Valid contract addresses for testing
    pub valid_addresses: Vec<Address>,
    /// Invalid address strings for testing validation
    pub invalid_addresses: Vec<String>,
    /// Mock contract metadata for this chain
    pub mock_metadata: ContractMetadata,
    /// Expected capabilities for this chain
    pub expected_capabilities: Vec<ChainCapability>,
    /// Test contract addresses known to be spam
    pub known_spam_addresses: Vec<Address>,
    /// Test contract addresses known to be legitimate
    pub known_legitimate_addresses: Vec<Address>,
}

impl ChainTestFixture {
    /// Create a test fixture for the specified chain
    pub fn for_chain(chain_id: ChainId) -> Self {
        match chain_id {
            ChainId::Ethereum => Self::ethereum(),
            ChainId::Polygon => Self::polygon(),
            ChainId::Base => Self::base(),
            ChainId::Avalanche => Self::avalanche(),
            ChainId::Arbitrum => Self::arbitrum(),
        }
    }

    /// Ethereum mainnet test fixture
    fn ethereum() -> Self {
        Self {
            chain_id: ChainId::Ethereum,
            valid_addresses: vec![
                Address::from([0x1a; 20]), // Mock CryptoPunks
                Address::from([0x2b; 20]), // Mock Bored Apes
                Address::from([0x3c; 20]), // Mock Art Blocks
            ],
            invalid_addresses: vec![
                "0x123".to_string(),                                    // Too short
                "0xghijklmnopqrstuvwxyz123456789012345678".to_string(), // Invalid hex
                "not_an_address".to_string(),                           // Not hex
            ],
            mock_metadata: ContractMetadata {
                address: Address::from([0x1a; 20]),
                name: Some("CryptoPunks".to_string()),
                symbol: Some("PUNKS".to_string()),
                total_supply: Some("10000".to_string()),
                holder_count: Some(5000),
                transaction_count: Some(250_000),
                creation_block: Some(3_914_495),
                creation_timestamp: None,
                creator_address: Some(Address::from([0xc0; 20])),
                is_verified: Some(true),
                contract_type: Some(ContractType::Erc721),
                additional_data: HashMap::new(),
            },
            expected_capabilities: vec![
                ChainCapability::MoralisMetadata,
                ChainCapability::PinaxAnalytics,
                ChainCapability::SpamPrediction,
            ],
            known_spam_addresses: vec![
                Address::from([0x4d; 20]), // Mock spam contract
                Address::from([0x5e; 20]), // Mock phishing contract
            ],
            known_legitimate_addresses: vec![
                Address::from([0x1a; 20]), // CryptoPunks
                Address::from([0x2b; 20]), // Bored Apes
            ],
        }
    }

    /// Polygon test fixture
    fn polygon() -> Self {
        Self {
            chain_id: ChainId::Polygon,
            valid_addresses: vec![
                Address::from([0x6f; 20]), // Mock OpenSea Polygon
                Address::from([0x7a; 20]), // Mock Polygon NFT
                Address::from([0x8b; 20]), // Mock Gaming NFT
            ],
            invalid_addresses: vec![
                "0x456".to_string(),
                "0xinvalidhex123456789012345678901234567890".to_string(),
                "polygon_address".to_string(),
            ],
            mock_metadata: ContractMetadata {
                address: Address::from([0x6f; 20]),
                name: Some("PolygonNFT".to_string()),
                symbol: Some("PNFT".to_string()),
                total_supply: Some("50000".to_string()),
                holder_count: Some(12000),
                transaction_count: Some(180_000),
                creation_block: Some(25_000_000),
                creation_timestamp: None,
                creator_address: Some(Address::from([0xc1; 20])),
                is_verified: Some(true),
                contract_type: Some(ContractType::Erc1155),
                additional_data: HashMap::new(),
            },
            expected_capabilities: vec![
                ChainCapability::MoralisMetadata,
                ChainCapability::PinaxAnalytics,
                ChainCapability::SpamPrediction,
            ],
            known_spam_addresses: vec![
                Address::from([0x9c; 20]), // Mock spam on Polygon
            ],
            known_legitimate_addresses: vec![
                Address::from([0x6f; 20]), // OpenSea Polygon
                Address::from([0x7a; 20]), // Gaming NFT
            ],
        }
    }

    /// Base test fixture
    fn base() -> Self {
        Self {
            chain_id: ChainId::Base,
            valid_addresses: vec![
                Address::from([0xad; 20]), // Mock Base NFT
                Address::from([0xbe; 20]), // Mock Base Art
                Address::from([0xcf; 20]), // Mock Base Gaming
            ],
            invalid_addresses: vec![
                "0x789".to_string(),
                "0xzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_string(),
                "base_address".to_string(),
            ],
            mock_metadata: ContractMetadata {
                address: Address::from([0xad; 20]),
                name: Some("BaseArt".to_string()),
                symbol: Some("BART".to_string()),
                total_supply: Some("1000".to_string()),
                holder_count: Some(800),
                transaction_count: Some(5000),
                creation_block: Some(5_000_000),
                creation_timestamp: None,
                creator_address: Some(Address::from([0xc2; 20])),
                is_verified: Some(true),
                contract_type: Some(ContractType::Erc721),
                additional_data: HashMap::new(),
            },
            expected_capabilities: vec![
                ChainCapability::MoralisMetadata,
                ChainCapability::PinaxAnalytics,
                ChainCapability::SpamPrediction,
            ],
            known_spam_addresses: vec![
                Address::from([0xd0; 20]), // Mock spam on Base
            ],
            known_legitimate_addresses: vec![
                Address::from([0xad; 20]), // Base Art
                Address::from([0xbe; 20]), // Base Gaming
            ],
        }
    }

    /// Avalanche test fixture
    fn avalanche() -> Self {
        Self {
            chain_id: ChainId::Avalanche,
            valid_addresses: vec![
                Address::from([0xe1; 20]), // Mock Avalanche NFT
                Address::from([0xf2; 20]), // Mock AVAX Art
                Address::from([0xa3; 20]), // Mock Snow NFT
            ],
            invalid_addresses: vec![
                "0xabc".to_string(),
                "0xwrongwrongwrongwrongwrongwrongwrongwrong".to_string(),
                "avalanche_address".to_string(),
            ],
            mock_metadata: ContractMetadata {
                address: Address::from([0xe1; 20]),
                name: Some("SnowNFT".to_string()),
                symbol: Some("SNOW".to_string()),
                total_supply: Some("5000".to_string()),
                holder_count: Some(2500),
                transaction_count: Some(30000),
                creation_block: Some(8_000_000),
                creation_timestamp: None,
                creator_address: Some(Address::from([0xc3; 20])),
                is_verified: Some(true),
                contract_type: Some(ContractType::Erc721),
                additional_data: HashMap::new(),
            },
            expected_capabilities: vec![
                ChainCapability::MoralisMetadata,
                ChainCapability::PinaxAnalytics,
                ChainCapability::SpamPrediction,
            ],
            known_spam_addresses: vec![
                Address::from([0xb4; 20]), // Mock spam on Avalanche
            ],
            known_legitimate_addresses: vec![
                Address::from([0xe1; 20]), // Snow NFT
                Address::from([0xf2; 20]), // AVAX Art
            ],
        }
    }

    /// Arbitrum test fixture
    fn arbitrum() -> Self {
        Self {
            chain_id: ChainId::Arbitrum,
            valid_addresses: vec![
                Address::from([0xc5; 20]), // Mock Arbitrum NFT
                Address::from([0xd6; 20]), // Mock Layer 2 Art
                Address::from([0xe7; 20]), // Mock Fast NFT
            ],
            invalid_addresses: vec![
                "0xdef".to_string(),
                "0xbadbadbadbadbadbadbadbadbadbadbadbadbad".to_string(),
                "arbitrum_address".to_string(),
            ],
            mock_metadata: ContractMetadata {
                address: Address::from([0xc5; 20]),
                name: Some("ArbitrumArt".to_string()),
                symbol: Some("ARBT".to_string()),
                total_supply: Some("20000".to_string()),
                holder_count: Some(8000),
                transaction_count: Some(120_000),
                creation_block: Some(15_000_000),
                creation_timestamp: None,
                creator_address: Some(Address::from([0xc4; 20])),
                is_verified: Some(true),
                contract_type: Some(ContractType::Erc1155),
                additional_data: HashMap::new(),
            },
            expected_capabilities: vec![
                ChainCapability::MoralisMetadata,
                ChainCapability::PinaxAnalytics,
                ChainCapability::SpamPrediction,
            ],
            known_spam_addresses: vec![
                Address::from([0xf8; 20]), // Mock spam on Arbitrum
            ],
            known_legitimate_addresses: vec![
                Address::from([0xc5; 20]), // Arbitrum Art
                Address::from([0xd6; 20]), // Layer 2 Art
            ],
        }
    }

    /// Get all chain test fixtures
    pub fn all_chains() -> Vec<Self> {
        ChainId::all()
            .iter()
            .map(|&chain_id| Self::for_chain(chain_id))
            .collect()
    }

    /// Get a valid test address for this chain
    pub fn get_valid_address(&self) -> Address {
        self.valid_addresses[0]
    }

    /// Get an invalid address string for this chain
    pub fn get_invalid_address(&self) -> &str {
        &self.invalid_addresses[0]
    }

    /// Get a known spam address for this chain
    pub fn get_spam_address(&self) -> Address {
        self.known_spam_addresses[0]
    }

    /// Get a known legitimate address for this chain
    pub fn get_legitimate_address(&self) -> Address {
        self.known_legitimate_addresses[0]
    }

    /// Create a mock successful API response for this chain
    pub fn mock_success_response(&self) -> serde_json::Value {
        json!({
            self.get_valid_address().to_string(): {
                "chain_id": self.chain_id.chain_id(),
                "contract_spam_status": false,
                "message": format!("contract metadata found on {}, AI analysis classified as legitimate", self.chain_id.name())
            }
        })
    }

    /// Create a mock spam detection response for this chain
    pub fn mock_spam_response(&self) -> serde_json::Value {
        json!({
            self.get_spam_address().to_string(): {
                "chain_id": self.chain_id.chain_id(),
                "contract_spam_status": true,
                "message": format!("contract metadata found on {}, AI analysis classified as spam", self.chain_id.name())
            }
        })
    }

    /// Create a mock no data response for this chain
    pub fn mock_no_data_response(&self) -> serde_json::Value {
        json!({
            self.get_valid_address().to_string(): {
                "chain_id": self.chain_id.chain_id(),
                "contract_spam_status": false,
                "message": format!("no data found for the contract on {}", self.chain_id.name())
            }
        })
    }
}
