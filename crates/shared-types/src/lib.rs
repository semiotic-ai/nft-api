// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Shared types for the NFT API service
//!
//! This crate provides common types that are shared across multiple crates
//! in the NFT API workspace, avoiding circular dependencies.

pub mod chains;
pub mod spam_status;

pub use chains::{ChainCapability, ChainId, ChainImplementationStatus, ChainStatus};
pub use spam_status::ContractSpamStatus;
