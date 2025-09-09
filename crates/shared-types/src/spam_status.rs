// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Contract spam status types

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Spam classification status for contracts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContractSpamStatus {
    /// Contract is classified as spam
    Spam,
    /// Contract is classified as legitimate
    Legitimate,
    /// Analysis was inconclusive
    Inconclusive,
    /// No data available for analysis
    NoData,
    /// Error occurred during analysis
    Error,
}

impl ContractSpamStatus {
    /// Check if the status represents spam
    pub fn is_spam(&self) -> bool {
        matches!(self, ContractSpamStatus::Spam)
    }

    /// Check if the status represents a legitimate contract
    pub fn is_legitimate(&self) -> bool {
        matches!(self, ContractSpamStatus::Legitimate)
    }

    /// Check if analysis was inconclusive
    pub fn is_inconclusive(&self) -> bool {
        matches!(self, ContractSpamStatus::Inconclusive)
    }

    /// Check if no data was available
    pub fn is_no_data(&self) -> bool {
        matches!(self, ContractSpamStatus::NoData)
    }

    /// Check if an error occurred
    pub fn is_error(&self) -> bool {
        matches!(self, ContractSpamStatus::Error)
    }

    /// Get a default message for this status
    pub fn default_message(&self) -> &'static str {
        match self {
            ContractSpamStatus::Spam => "AI analysis classified as spam",
            ContractSpamStatus::Legitimate => "AI analysis classified as legitimate",
            ContractSpamStatus::Inconclusive => {
                "AI analysis was inconclusive, defaulting to not spam"
            }
            ContractSpamStatus::NoData => "no data found for the contract",
            ContractSpamStatus::Error => "unable to retrieve contract data from external services",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spam_status_boolean_checks() {
        assert!(ContractSpamStatus::Spam.is_spam());
        assert!(!ContractSpamStatus::Spam.is_legitimate());
        assert!(!ContractSpamStatus::Spam.is_inconclusive());
        assert!(!ContractSpamStatus::Spam.is_no_data());
        assert!(!ContractSpamStatus::Spam.is_error());

        assert!(!ContractSpamStatus::Legitimate.is_spam());
        assert!(ContractSpamStatus::Legitimate.is_legitimate());
        assert!(!ContractSpamStatus::Legitimate.is_inconclusive());
        assert!(!ContractSpamStatus::Legitimate.is_no_data());
        assert!(!ContractSpamStatus::Legitimate.is_error());
    }

    #[test]
    fn default_messages() {
        assert_eq!(
            ContractSpamStatus::Spam.default_message(),
            "AI analysis classified as spam"
        );
        assert_eq!(
            ContractSpamStatus::Legitimate.default_message(),
            "AI analysis classified as legitimate"
        );
        assert_eq!(
            ContractSpamStatus::NoData.default_message(),
            "no data found for the contract"
        );
    }

    #[test]
    fn serde_serialization() {
        let spam = ContractSpamStatus::Spam;
        let serialized = serde_json::to_string(&spam).unwrap();
        assert_eq!(serialized, "\"spam\"");

        let legitimate = ContractSpamStatus::Legitimate;
        let serialized = serde_json::to_string(&legitimate).unwrap();
        assert_eq!(serialized, "\"legitimate\"");

        let no_data = ContractSpamStatus::NoData;
        let serialized = serde_json::to_string(&no_data).unwrap();
        assert_eq!(serialized, "\"no_data\"");
    }

    #[test]
    fn serde_deserialization() {
        let deserialized: ContractSpamStatus = serde_json::from_str("\"spam\"").unwrap();
        assert_eq!(deserialized, ContractSpamStatus::Spam);

        let deserialized: ContractSpamStatus = serde_json::from_str("\"legitimate\"").unwrap();
        assert_eq!(deserialized, ContractSpamStatus::Legitimate);

        let deserialized: ContractSpamStatus = serde_json::from_str("\"inconclusive\"").unwrap();
        assert_eq!(deserialized, ContractSpamStatus::Inconclusive);
    }
}
