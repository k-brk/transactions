use rust_decimal::Decimal;
use serde::Deserialize;

use super::account::ClientID;

pub type TransactionID = u32;

/// Represents model of incoming transaction
#[derive(Deserialize, Debug)]
pub struct Transaction {
    #[serde(flatten)]
    pub kind: TransactionKind,
    #[serde(flatten)]
    pub metadata: TransactionMetadata,
    #[serde(skip)]
    pub state: TransactionState,
}

impl Transaction {
    pub fn tx_id(&self) -> TransactionID {
        self.metadata.tx_id
    }
    pub fn client_id(&self) -> ClientID {
        self.metadata.client_id
    }
}

/// Determinates type of transaction
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum TransactionKind {
    Deposit { amount: Decimal },
    Withdrawal { amount: Decimal },
    Dispute,
    Resolve,
    Chargeback,
}

/// Metadata keeps client and transaction ids
#[derive(Deserialize, Debug)]
pub struct TransactionMetadata {
    #[serde(rename = "client")]
    pub client_id: ClientID,
    #[serde(rename = "tx")]
    pub tx_id: TransactionID,
}

/// States of transaction
/// New
/// Success - transaction is successfully processed
/// Failed - transaction processing failed
/// Disputed - transaction is being disputed
/// Resolved - dispute has been resolved
/// Chargeback - transaction has been chargedback
#[derive(Debug, PartialEq, Eq)]
pub enum TransactionState {
    New,
    Succeeded,
    Failed,
    Disputed,
    Resolved,
    Chargeback,
}

impl Default for TransactionState {
    fn default() -> Self {
        TransactionState::New
    }
}
