use super::{
    account_store, transaction::Transaction, transaction_processor::TransactionProcessor,
    transaction_store,
};

/// [`Engine`] in an entry point for transaction processing
///
/// Engine has a transaction processor which converts incoming transaction into [`AccountDelta`],
/// it describes how given transaction will affect user account and what changes needs to be applied (balances, locks).
///
/// Once delta is generated, it is applied to user account to reflect changes from transaction.
///
///    Transaction
///         │
/// ┌───────▼────────┐
/// │                 │
/// │      Engine     │
/// │                 │
/// └───────┬─────────┘
///         │ Transaction
///         │
/// ┌───────▼────────┐
/// │                 │
/// │   Transaction   │
/// │    Processor    │
/// │                 │
/// └───────┬─────────┘
///         │ AccountDelta
///         │
/// ┌───────▼────────┐
/// │                 │
/// │     Account     │
/// │                 │
/// └─────────────────┘
#[derive(Default)]
pub struct Engine<T, A>
where
    T: transaction_store::Store + Default,
    A: account_store::Store,
{
    transactions: TransactionProcessor<T>,
    accounts: A,
}

impl<T, A> Engine<T, A>
where
    T: transaction_store::Store + Default,
    A: account_store::Store,
{
    /// processes transaction and applies outcome of it to user account
    pub fn process_transaction(&mut self, transaction: Transaction) {
        let client_id = transaction.client_id();
        let tx_id = transaction.tx_id();

        let change = self.transactions.produce_delta(transaction);

        match self.accounts.get_mut_or_new(client_id).apply(change) {
            Ok(_) => {
                self.transactions.succeed(tx_id);
            }
            Err(err) => {
                self.transactions.failed(tx_id);
                log::error!("Transaction {:?} failed: {:?}", tx_id, err);
            }
        }
    }

    // returns all users accounts
    pub fn accounts(&self) -> &A::Storage {
        self.accounts.accounts()
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::core::{
        account::{Account, Amount},
        account_store::AccountStore,
        transaction::Transaction,
        transaction_store::TransactionStore,
    };

    use super::Engine;

    fn read_transactions(transactions: &str) -> Vec<Transaction> {
        csv::ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(transactions.as_bytes())
            .deserialize()
            .flatten()
            .collect()
    }

    #[test_case(
    r#"
    type,client,tx,amount
    deposit,1,1,3.0
    deposit,2,2,2.0
    deposit,1,3,5.0
    "#,  
    Account { id: 1, available: Amount::new(8,0), held: Amount::ZERO, total: Amount::new(8,0), locked: false }  ; "deposit_should_increase_available_funds")]
    #[test_case(
    r#"
    type,client,tx,amount
    deposit,1,1,3.0
    deposit,2,2,2.0
    withdrawal,1,3,5.0
    "#,  
    Account { id: 1, available: Amount::new(3,0), held: Amount::ZERO, total: Amount::new(3,0), locked: false }  ; "withdrawal_should_not_exceed_available_funds")]
    #[test_case(
    r#"
    type,client,tx,amount
    deposit,1,1,3.0
    deposit,2,2,2.0
    withdrawal,1,3,2.0
    "#,  
    Account { id: 1, available: Amount::new(1,0), held: Amount::ZERO, total: Amount::new(1,0), locked: false }  ; "withdrawal_should_decrease_available_funds")]
    #[test_case(
    r#"
    type,client,tx,amount
    deposit,1,1,3.0
    deposit,2,2,2.0
    dispute,1,1,
    "#,  
    Account { id: 1, available: Amount::ZERO, held: Amount::new(3,0), total: Amount::new(3,0), locked: false }  ; "dispute_should_decrease_available_funds_and_increase_held")]
    #[test_case(
    r#"
    type,client,tx,amount
    deposit,1,1,3.0
    deposit,2,2,2.0
    withdrawal,1,3,2.0
    dispute,1,3,
    "#,  
    Account { id: 1, available: Amount::new(1, 0), held: Amount::new(2,0), total: Amount::new(3,0), locked: false }  ; "dispute_on_withdrawal_should_increase_held_funds")]
    #[test_case(
    r#"
    type,client,tx,amount
    deposit,1,1,3.0
    deposit,2,2,2.0
    withdrawal,1,3,2.0
    dispute,1,1,
    "#,  
    Account { id: 1, available: Amount::new(-2, 0), held: Amount::new(3,0), total: Amount::new(1,0), locked: false }  ; "dispute_on_deposit_when_user_is_out_of_money_should_create_debt")]
    #[test_case(
    r#"
    type,client,tx,amount
    deposit,1,1,3.0
    deposit,2,2,2.0
    withdrawal,1,3,2.0
    dispute,1,3,
    resolve,1,3,
    "#,  
    Account { id: 1, available: Amount::new(3, 0), held: Amount::ZERO, total: Amount::new(3,0), locked: false }  ; "resolved_dispute_should_increase_available_funds_and_decrease_held_funds")]
    #[test_case(
    r#"
    type,client,tx,amount
    deposit,1,1,3.0
    deposit,2,2,2.0
    withdrawal,1,3,2.0
    dispute,1,3,
    chargeback,1,3,
    "#,  
    Account { id: 1, available: Amount::new(1, 0), held: Amount::ZERO, total: Amount::new(1,0), locked: true }  ; "charge_should_withdraw_held_funds_and_lock_acc")]

    fn engine(input_data: &str, expected_acc: Account) {
        let transactions = read_transactions(input_data);
        let mut engine = Engine::<TransactionStore, AccountStore>::default();

        transactions
            .into_iter()
            .for_each(|f| engine.process_transaction(f));

        let accounts = engine.accounts();

        assert_eq!(accounts.len(), 2);
        let acc_1 = accounts.get(&1).unwrap();

        assert_eq!(acc_1.available, expected_acc.available);
        assert_eq!(acc_1.total, expected_acc.total);
        assert_eq!(acc_1.held, expected_acc.held);
        assert_eq!(acc_1.locked, expected_acc.locked);

        let acc_2 = accounts.get(&2).unwrap();

        assert_eq!(acc_2.available, Amount::new(2, 0));
        assert_eq!(acc_2.total, Amount::new(2, 0));
        assert_eq!(acc_2.held, Amount::ZERO);
        assert_eq!(acc_2.locked, false);
    }
}
