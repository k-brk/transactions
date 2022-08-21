pub mod account;
pub mod account_store;


pub mod transaction;
pub mod transaction_processor;
pub mod transaction_store;

#[cfg(test)]
mod tests {
    use super::{
        account::ClientID,
        transaction::{Transaction, TransactionID, TransactionKind, TransactionMetadata},
    };

    pub fn transaction(
        kind: TransactionKind,
        tx_id: TransactionID,
        client_id: ClientID,
    ) -> Transaction {
        Transaction {
            kind,
            metadata: TransactionMetadata { client_id, tx_id },
            state: Default::default(),
        }
    }
}
