use super::{
    account::AccountDelta,
    transaction::{Transaction, TransactionID, TransactionKind, TransactionState},
    transaction_store::Store,
};

/// Processes transactions and produces delta of user balance as a outcome of transaction
#[derive(Default)]
pub struct TransactionProcessor<S>
where
    S: Store + Default,
{
    transactions: S,
}

impl<S> TransactionProcessor<S>
where
    S: Store + Default,
{
    /// Returns delta of balance based on transaction thats should be applied to user account
    pub fn produce_delta(&mut self, transaction: Transaction) -> AccountDelta {
        match transaction.kind {
            TransactionKind::Deposit { amount } => {
                self.transactions.insert(transaction);
                AccountDelta::deposit(amount)
            }
            TransactionKind::Withdrawal { amount } => {
                self.transactions.insert(transaction);
                AccountDelta::withdrawal(amount)
            }

            TransactionKind::Dispute => self.dispute(&transaction),
            TransactionKind::Resolve => self.resolve(&transaction),
            TransactionKind::Chargeback => self.chargeback(&transaction),
        }
    }

    pub fn succeed(&mut self, tx_id: TransactionID) {
        self.set_state(tx_id, TransactionState::Succeeded)
    }

    pub fn failed(&mut self, tx_id: TransactionID) {
        self.set_state(tx_id, TransactionState::Failed)
    }

    fn set_state(&mut self, tx_id: TransactionID, state: TransactionState) {
        if let Some(tx) = self.transactions.get_mut(&tx_id) {
            // New transaction can either succes or fail, can't be anything else as it is not fully processed yet.
            if tx.state == TransactionState::New {
                tx.state = state;
            }
        }
    }

    /// Returns delta for disputed transaction
    /// Only deposit and withdrawal transaction can be disputed, for others delta is empty
    ///
    /// In case of dispute of deposit then following operation should be invoked:
    /// - Decrease available funds by disputed amount
    /// - Increase held funds by disputed amount
    ///
    /// In case of dispute of withdrawal:
    /// - Increase held funds by disputed amount
    ///
    /// [`TransactionState`] is set to [`TransactionState::Disputed`].
    fn dispute(&mut self, disputed_transaction: &Transaction) -> AccountDelta {
        if let Some(transaction) = self.transactions.get_mut(&disputed_transaction.tx_id()) {
            if disputed_transaction.client_id() != transaction.client_id() {
                return AccountDelta::none()
            }

            if transaction.state == TransactionState::Resolved
                || transaction.state == TransactionState::Chargeback
                || transaction.state == TransactionState::Disputed
            {
                return AccountDelta::none();
            }

            match transaction.kind {
                TransactionKind::Deposit { amount } => {
                    transaction.state = TransactionState::Disputed;
                    AccountDelta::dispute_deposit(amount)
                }
                TransactionKind::Withdrawal { amount } => {
                    transaction.state = TransactionState::Disputed;
                    AccountDelta::dispute_withdrawal(amount)
                }
                _ => AccountDelta::none(),
            }
        } else {
            AccountDelta::none()
        }
    }

    /// Returns delta for resolved transaction
    /// Only deposit and withdrawal transaction can be resolved and their [`TransactionState`] needs to be set to [`TransactionState::Disputed`]
    ///
    /// In case of dispute of deposit then following operation are invoked:
    /// - Increase available funds by disputed amount
    /// - Decrease held funds by disputed amount
    ///
    /// In case of dispute of withdrawal:
    /// - Increase available funds by disputed amount
    /// - Decrease held funds by disputed amount
    ///
    fn resolve(&mut self, resolve_transaction: &Transaction) -> AccountDelta {
        if let Some(transaction) = self.transactions.get_mut(&resolve_transaction.tx_id()) {
            if resolve_transaction.client_id() != transaction.client_id() {
                return AccountDelta::none()
            }

            if transaction.state != TransactionState::Disputed {
                return AccountDelta::none();
            }

            match transaction.kind {
                TransactionKind::Deposit { amount } | TransactionKind::Withdrawal { amount } => {
                    transaction.state = TransactionState::Resolved;

                    return AccountDelta::resolve(amount);
                }

                _ => return AccountDelta::none(),
            }
        }
        AccountDelta::none()
    }

    /// Returns delta for chargeback transaction.
    /// Only deposit and withdrawal transaction can be chargedback and their [`TransactionState`] needs to be set to [`TransactionState::Disputed`]
    /// Held funds are being withdrawn and user account is immediately locked after this operation
    ///
    fn chargeback(&mut self, chargeback_transaction: &Transaction) -> AccountDelta {
        if let Some(transaction) = self.transactions.get_mut(&chargeback_transaction.tx_id()) {
            if chargeback_transaction.client_id() != transaction.client_id() {
                return AccountDelta::none()
            }

            if transaction.state == TransactionState::Disputed {
                let change = match transaction.kind {
                    TransactionKind::Deposit { amount }
                    | TransactionKind::Withdrawal { amount } => {
                        transaction.state = TransactionState::Chargeback;

                        AccountDelta::chargeback(amount)
                    }

                    _ => AccountDelta::none(),
                };
                return change;
            }
        }
        AccountDelta::none()
    }
}

#[cfg(test)]
mod tests {
    use super::TransactionProcessor;
    use crate::core::{
        account::Amount, tests::transaction, transaction, transaction_store::TransactionStore,
    };

    #[test]
    fn deposit_should_create_deposit_change() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let transaction = transaction(
            transaction::TransactionKind::Deposit {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );

        let change = processor.produce_delta(transaction);

        assert_eq!(change.available.unwrap_or_default(), Amount::new(3, 1));
    }

    #[test]
    fn withdraw_should_create_widthdraw_change() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let transaction = transaction(
            transaction::TransactionKind::Withdrawal {
                amount: Amount::new(5, 1),
            },
            1,
            1,
        );

        let change = processor.produce_delta(transaction);

        assert_eq!(change.available.unwrap_or_default(), Amount::new(-5, 1));
    }

    #[test]
    fn dispute_on_not_existing_transaction_should_do_nothing() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let transaction = transaction(transaction::TransactionKind::Dispute, 1, 1);

        let change = processor.produce_delta(transaction);

        assert!(change.available.is_none());
        assert!(change.held.is_none());
        assert!(change.locked.is_none());
    }

    #[test]
    fn dispute_on_deposit_transaction_should_incr_held_funds_and_decr_available() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let deposit = transaction(
            transaction::TransactionKind::Deposit {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );

        let deposit_change = processor.produce_delta(deposit);
        assert_eq!(
            deposit_change.available.unwrap_or_default(),
            Amount::new(3, 1)
        );
        assert!(deposit_change.held.is_none());
        assert!(deposit_change.locked.is_none());

        let dispute = transaction(transaction::TransactionKind::Dispute, 1, 1);
        let dispute_change = processor.produce_delta(dispute);

        assert_eq!(
            dispute_change.available.unwrap_or_default(),
            Amount::new(-3, 1)
        );
        assert_eq!(dispute_change.held.unwrap_or_default(), Amount::new(3, 1));
        assert!(dispute_change.locked.is_none());
    }

    #[test]
    fn dispute_on_incorrect_client_transaction_should_do_nothing() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let deposit = transaction(
            transaction::TransactionKind::Deposit {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );

        let deposit_change = processor.produce_delta(deposit);
        assert_eq!(
            deposit_change.available.unwrap_or_default(),
            Amount::new(3, 1)
        );
        assert!(deposit_change.held.is_none());
        assert!(deposit_change.locked.is_none());

        let dispute = transaction(transaction::TransactionKind::Dispute, 1, 2);
        let dispute_change = processor.produce_delta(dispute);

        assert!(
            dispute_change.available.is_none(),
        );
        assert!(dispute_change.held.is_none());
        assert!(dispute_change.locked.is_none());
    }


    #[test]
    fn dispute_on_withdrawal_transaction_should_increase_held_funds() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let withdrawal = transaction(
            transaction::TransactionKind::Withdrawal {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );

        let withdrawal_change = processor.produce_delta(withdrawal);
        assert_eq!(
            withdrawal_change.available.unwrap_or_default(),
            Amount::new(-3, 1)
        );
        assert!(withdrawal_change.held.is_none());
        assert!(withdrawal_change.locked.is_none());

        let dispute = transaction(transaction::TransactionKind::Dispute, 1, 1);
        let dispute_change = processor.produce_delta(dispute);

        assert!(dispute_change.available.is_none());
        assert_eq!(dispute_change.held.unwrap_or_default(), Amount::new(3, 1));
        assert!(dispute_change.locked.is_none());
    }

    #[test]
    fn resolve_on_not_existing_transaction_should_do_nothing() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let transaction = transaction(transaction::TransactionKind::Resolve, 1, 1);

        let change = processor.produce_delta(transaction);

        assert!(change.available.is_none());
        assert!(change.held.is_none());
        assert!(change.locked.is_none());
    }

    #[test]
    fn resolve_on_not_disputed_transaction_should_do_nothing() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let withdrawal = transaction(
            transaction::TransactionKind::Withdrawal {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );

        let withdrawal_change = processor.produce_delta(withdrawal);
        assert_eq!(
            withdrawal_change.available.unwrap_or_default(),
            Amount::new(-3, 1)
        );
        assert!(withdrawal_change.held.is_none());
        assert!(withdrawal_change.locked.is_none());

        let resolve = transaction(transaction::TransactionKind::Resolve, 1, 1);
        let resolve_change = processor.produce_delta(resolve);

        assert!(resolve_change.available.is_none());
        assert!(resolve_change.held.is_none());
        assert!(resolve_change.locked.is_none());
    }

    #[test]
    fn resolve_on_incorrect_client_transaction_should_do_nothing() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let deposit = transaction(
            transaction::TransactionKind::Deposit {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );

        let deposit_change = processor.produce_delta(deposit);
        assert_eq!(
            deposit_change.available.unwrap_or_default(),
            Amount::new(3, 1)
        );
        assert!(deposit_change.held.is_none());
        assert!(deposit_change.locked.is_none());

        let resolve = transaction(transaction::TransactionKind::Resolve, 1, 2);
        let resolve_change = processor.produce_delta(resolve);

        assert!(
            resolve_change.available.is_none(),
        );
        assert!(resolve_change.held.is_none());
        assert!(resolve_change.locked.is_none());
    }


    #[test]
    fn resolve_on_dispute_of_deposit_transaction_should_increase_available_funds_and_decr_held() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let deposit = transaction(
            transaction::TransactionKind::Deposit {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );
        let deposit_change = processor.produce_delta(deposit);

        assert_eq!(
            deposit_change.available.unwrap_or_default(),
            Amount::new(3, 1)
        );
        assert!(deposit_change.held.is_none());
        assert!(deposit_change.locked.is_none());

        let dispute = transaction(transaction::TransactionKind::Dispute, 1, 1);
        let dispute_change = processor.produce_delta(dispute);

        assert_eq!(
            dispute_change.available.unwrap_or_default(),
            Amount::new(-3, 1)
        );
        assert_eq!(dispute_change.held.unwrap_or_default(), Amount::new(3, 1));
        assert!(dispute_change.locked.is_none());

        let resolve = transaction(transaction::TransactionKind::Resolve, 1, 1);
        let resolve_change = processor.produce_delta(resolve);

        assert_eq!(
            resolve_change.available.unwrap_or_default(),
            Amount::new(3, 1)
        );
        assert_eq!(resolve_change.held.unwrap_or_default(), Amount::new(-3, 1));
        assert!(dispute_change.locked.is_none());
    }

    #[test]
    fn resolve_on_dispute_of_withdrawal_transaction_should_increase_available_funds_and_decr_held()
    {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let withdrawal = transaction(
            transaction::TransactionKind::Withdrawal {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );
        let withdrawal_change = processor.produce_delta(withdrawal);

        assert_eq!(
            withdrawal_change.available.unwrap_or_default(),
            Amount::new(-3, 1)
        );
        assert!(withdrawal_change.held.is_none());
        assert!(withdrawal_change.locked.is_none());

        let dispute = transaction(transaction::TransactionKind::Dispute, 1, 1);
        let dispute_change = processor.produce_delta(dispute);

        assert!(dispute_change.available.is_none());
        assert_eq!(dispute_change.held.unwrap_or_default(), Amount::new(3, 1));
        assert!(dispute_change.locked.is_none());

        let resolve = transaction(transaction::TransactionKind::Resolve, 1, 1);
        let resolve_change = processor.produce_delta(resolve);

        assert_eq!(
            resolve_change.available.unwrap_or_default(),
            Amount::new(3, 1)
        );
        assert_eq!(resolve_change.held.unwrap_or_default(), Amount::new(-3, 1));
        assert!(dispute_change.locked.is_none());
    }

    #[test]
    fn resolve_on_resolved_dispute_of_deposit_transaction_should_do_nothing() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let deposit = transaction(
            transaction::TransactionKind::Deposit {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );
        let deposit_change = processor.produce_delta(deposit);

        assert_eq!(
            deposit_change.available.unwrap_or_default(),
            Amount::new(3, 1)
        );
        assert!(deposit_change.held.is_none());
        assert!(deposit_change.locked.is_none());

        let dispute = transaction(transaction::TransactionKind::Dispute, 1, 1);
        let dispute_change = processor.produce_delta(dispute);

        assert_eq!(
            dispute_change.available.unwrap_or_default(),
            Amount::new(-3, 1)
        );
        assert_eq!(dispute_change.held.unwrap_or_default(), Amount::new(3, 1));
        assert!(dispute_change.locked.is_none());

        let resolve = transaction(transaction::TransactionKind::Resolve, 1, 1);
        let resolve_change = processor.produce_delta(resolve);

        assert_eq!(
            resolve_change.available.unwrap_or_default(),
            Amount::new(3, 1)
        );
        assert_eq!(resolve_change.held.unwrap_or_default(), Amount::new(-3, 1));
        assert!(dispute_change.locked.is_none());

        let dispute2 = transaction(transaction::TransactionKind::Dispute, 1, 1);
        let dispute2_change = processor.produce_delta(dispute2);

        assert!(dispute2_change.available.is_none());
        assert!(dispute2_change.held.is_none());
        assert!(dispute2_change.locked.is_none());
    }

    #[test]
    fn chargeback_on_not_existing_transaction_should_do_nothing() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let chargeback = transaction(transaction::TransactionKind::Chargeback, 1, 1);
        let chargeback_change = processor.produce_delta(chargeback);

        assert!(chargeback_change.available.is_none());
        assert!(chargeback_change.held.is_none());
        assert!(chargeback_change.locked.is_none());
    }

    #[test]
    fn chargeback_on_not_disputed_transaction_should_do_nothing() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let withdrawal = transaction(
            transaction::TransactionKind::Withdrawal {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );

        let withdrawal_change = processor.produce_delta(withdrawal);
        assert_eq!(
            withdrawal_change.available.unwrap_or_default(),
            Amount::new(-3, 1)
        );
        assert!(withdrawal_change.held.is_none());
        assert!(withdrawal_change.locked.is_none());

        let chargeback = transaction(transaction::TransactionKind::Chargeback, 1, 1);
        let chargeback_change = processor.produce_delta(chargeback);

        assert!(chargeback_change.available.is_none());
        assert!(chargeback_change.held.is_none());
        assert!(chargeback_change.locked.is_none());
    }

    #[test]
    fn chargeback_on_incorrect_client_transaction_should_do_nothing() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let deposit = transaction(
            transaction::TransactionKind::Deposit {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );

        let deposit_change = processor.produce_delta(deposit);
        assert_eq!(
            deposit_change.available.unwrap_or_default(),
            Amount::new(3, 1)
        );
        assert!(deposit_change.held.is_none());
        assert!(deposit_change.locked.is_none());

        let chargeback = transaction(transaction::TransactionKind::Chargeback, 1, 2);
        let chargeback_change = processor.produce_delta(chargeback);

        assert!(
            chargeback_change.available.is_none(),
        );
        assert!(chargeback_change.held.is_none());
        assert!(chargeback_change.locked.is_none());
    }

    #[test]
    fn chargeback_on_dispute_should_withdraw_held_funds_and_lock_acc() {
        let mut processor = TransactionProcessor::<TransactionStore>::default();

        let withdrawal = transaction(
            transaction::TransactionKind::Withdrawal {
                amount: Amount::new(3, 1),
            },
            1,
            1,
        );

        let withdrawal_change = processor.produce_delta(withdrawal);
        assert_eq!(
            withdrawal_change.available.unwrap_or_default(),
            Amount::new(-3, 1)
        );
        assert!(withdrawal_change.held.is_none());
        assert!(withdrawal_change.locked.is_none());

        let dispute = transaction(transaction::TransactionKind::Dispute, 1, 1);
        let dispute_change = processor.produce_delta(dispute);

        assert!(dispute_change.available.is_none());
        assert_eq!(dispute_change.held.unwrap_or_default(), Amount::new(3, 1));
        assert!(dispute_change.locked.is_none());

        let chargeback = transaction(transaction::TransactionKind::Chargeback, 1, 1);
        let chargeback_change = processor.produce_delta(chargeback);

        assert!(chargeback_change.available.is_none());
        assert_eq!(
            chargeback_change.held.unwrap_or_default(),
            Amount::new(-3, 1)
        );

        let locked = if let Some(val) = chargeback_change.locked {
            val
        } else {
            false
        };
        assert!(locked);
    }
}
