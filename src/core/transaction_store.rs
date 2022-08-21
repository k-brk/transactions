use std::collections::HashMap;

use super::transaction::{Transaction, TransactionID};

pub trait Store {
    // Inserts transaction to storage
    fn insert(&mut self, transaction: Transaction);
    // Returns reference to corresponding transaction
    fn get(&self, tx_id: &TransactionID) -> Option<&Transaction>;
    // Returns mutable reference to corresponding transaction
    fn get_mut(&mut self, tx_id: &TransactionID) -> Option<&mut Transaction>;
}

/// Represents collection of transactions
#[derive(Default)]
pub struct TransactionStore {
    transactions: HashMap<TransactionID, Transaction>,
}

impl Store for TransactionStore {
    fn insert(&mut self, transaction: Transaction) {
        self.transactions.insert(transaction.tx_id(), transaction);
    }

    fn get(&self, tx_id: &TransactionID) -> Option<&Transaction> {
        self.transactions.get(tx_id)
    }

    fn get_mut(&mut self, tx_id: &TransactionID) -> Option<&mut Transaction> {
        self.transactions.get_mut(tx_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{account::Amount, tests::transaction, transaction::TransactionKind};

    use super::{Store, TransactionStore};

    #[test]
    fn insert_and_get_transaction() {
        let mut store = TransactionStore::default();

        let tx_id = 1;

        store.insert(transaction(
            TransactionKind::Deposit {
                amount: Amount::ONE,
            },
            tx_id,
            1,
        ));

        let transaction = store.get(&tx_id);

        assert!(transaction.is_some());

        assert_eq!(transaction.unwrap().metadata.client_id, 1);
    }
}
