use rust_decimal::Decimal;
use serde::{Serialize, Serializer};

pub type ClientID = u16;
pub type Amount = Decimal;

#[derive(thiserror::Error, Debug)]
pub enum AccountError {
    #[error("Account is locked")]
    Locked,
    #[error("Insufficient funds")]
    InsufficientFunds,
}
/// Represents user account
#[derive(Serialize, Default, Debug)]
pub struct Account {
    #[serde(rename = "client")]
    pub(crate) id: ClientID,
    #[serde(serialize_with = "fixed_width_amount")]
    pub(crate) available: Amount,
    #[serde(serialize_with = "fixed_width_amount")]
    pub(crate) held: Amount,
    #[serde(serialize_with = "fixed_width_amount")]
    pub(crate) total: Amount,
    pub(crate) locked: bool,
}

pub fn fixed_width_amount<S>(amount: &Amount, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    const PRECISION: u32 = 4;
    serializer.serialize_str(
        &amount
            .round_dp_with_strategy(
                PRECISION,
                rust_decimal::RoundingStrategy::MidpointAwayFromZero,
            )
            .to_string(),
    )
}

impl Account {
    pub(crate) fn new(id: ClientID) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    /// Applies delta of user balance, changes are applied only when account is not locked
    pub fn apply(&mut self, change: AccountDelta) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }

        if let Some(available) = change.available {
            let can_create_debt = change.can_create_debt.unwrap_or_default();

            if !can_create_debt && self.available + available < Decimal::ZERO {
                return Err(AccountError::InsufficientFunds);
            }
            self.available += available;
        }

        if let Some(held) = change.held {
            self.held += held;
        }

        if let Some(locked) = change.locked {
            self.locked = locked;
        }

        self.update_total();

        Ok(())
    }

    fn update_total(&mut self) {
        self.total = self.available + self.held;
    }
}

/// Represents potential account changes which are outcome of incoming transaction
#[derive(Default)]
pub struct AccountDelta {
    pub available: Option<Amount>,
    pub held: Option<Amount>,
    pub locked: Option<bool>,

    // This is only possible when there is dispute on deposit and user already withdrawn those funds
    pub can_create_debt: Option<bool>,
}

// Helpers for different kind of transactions
impl AccountDelta {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn deposit(amount: Amount) -> Self {
        Self {
            available: Some(amount),
            ..Default::default()
        }
    }

    pub fn withdrawal(amount: Amount) -> Self {
        Self::deposit(-amount)
    }

    pub fn resolve(amount: Amount) -> Self {
        Self {
            available: Some(amount),
            held: Some(-amount),
            ..Default::default()
        }
    }

    pub fn dispute_deposit(amount: Amount) -> Self {
        Self {
            available: Some(-amount),
            held: Some(amount),
            can_create_debt: Some(true),
            ..Default::default()
        }
    }

    pub fn dispute_withdrawal(amount: Amount) -> Self {
        Self {
            held: Some(amount),
            ..Default::default()
        }
    }

    pub fn chargeback(amount: Amount) -> Self {
        Self {
            held: Some(-amount),
            locked: Some(true),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::account::Amount;

    use super::{Account, AccountDelta, AccountError};

    #[test]
    fn deposit_should_increase_available_funds_and_total() -> Result<(), AccountError> {
        let mut account = Account::new(1);

        let deposit = AccountDelta::deposit(Amount::ONE);
        account.apply(deposit)?;

        assert_eq!(account.available, Amount::ONE);
        assert_eq!(account.total, Amount::ONE);

        Ok(())
    }

    #[test]
    fn withdrawal_should_decrease_available_funds_and_total() -> Result<(), AccountError> {
        let mut account = Account::new(1);

        let deposit = AccountDelta::deposit(Amount::TWO);
        account.apply(deposit)?;

        assert_eq!(account.available, Amount::TWO);
        assert_eq!(account.total, Amount::TWO);

        let withdrawal = AccountDelta::withdrawal(Amount::ONE);
        account.apply(withdrawal)?;

        assert_eq!(account.available, Amount::ONE);
        assert_eq!(account.total, Amount::ONE);

        Ok(())
    }

    #[test]
    fn withdrawal_should_fail_when_insufficent_funds() -> Result<(), AccountError> {
        let mut account = Account::new(1);

        let deposit = AccountDelta::deposit(Amount::TWO);
        account.apply(deposit)?;

        assert_eq!(account.available, Amount::TWO);
        assert_eq!(account.total, Amount::TWO);

        let withdrawal = AccountDelta::withdrawal(Amount::TEN);

        let mut insufficient_funds = false;

        if let Err(AccountError::InsufficientFunds) = account.apply(withdrawal) {
            insufficient_funds = true;
        }

        assert_eq!(account.available, Amount::TWO);
        assert_eq!(account.total, Amount::TWO);
        assert_eq!(insufficient_funds, true);

        Ok(())
    }

    #[test]
    fn dispute_on_deposit_should_decrease_available_funds_and_increase_held(
    ) -> Result<(), AccountError> {
        let mut account = Account::new(1);

        let deposit = AccountDelta::deposit(Amount::TWO);
        account.apply(deposit)?;

        assert_eq!(account.available, Amount::TWO);
        assert_eq!(account.total, Amount::TWO);

        let dispute = AccountDelta::dispute_deposit(Amount::ONE);
        account.apply(dispute)?;

        assert_eq!(account.held, Amount::ONE);
        assert_eq!(account.available, Amount::ONE);
        assert_eq!(account.total, Amount::TWO);

        Ok(())
    }

    #[test]
    fn dispute_on_withdrawal_should_increase_held() -> Result<(), AccountError> {
        let mut account = Account::new(1);

        let deposit = AccountDelta::deposit(Amount::TWO);
        account.apply(deposit)?;

        assert_eq!(account.available, Amount::TWO);
        assert_eq!(account.total, Amount::TWO);

        let withdrawal = AccountDelta::withdrawal(Amount::ONE);
        account.apply(withdrawal)?;

        assert_eq!(account.available, Amount::ONE);
        assert_eq!(account.total, Amount::ONE);

        let dispute = AccountDelta::dispute_withdrawal(Amount::ONE);

        account.apply(dispute)?;

        assert_eq!(account.held, Amount::ONE);
        assert_eq!(account.available, Amount::ONE);
        assert_eq!(account.total, Amount::TWO);

        Ok(())
    }

    #[test]
    fn resolve_should_decrease_held_funds_and_increase_available() -> Result<(), AccountError> {
        let mut account = Account::new(1);

        let deposit = AccountDelta::deposit(Amount::TWO);
        account.apply(deposit)?;

        assert_eq!(account.available, Amount::TWO);
        assert_eq!(account.total, Amount::TWO);

        let dispute = AccountDelta::dispute_deposit(Amount::ONE);
        account.apply(dispute)?;

        assert_eq!(account.held, Amount::ONE);
        assert_eq!(account.available, Amount::ONE);
        assert_eq!(account.total, Amount::TWO);

        let resolve = AccountDelta::resolve(Amount::ONE);
        account.apply(resolve)?;

        assert_eq!(account.held, Amount::ZERO);
        assert_eq!(account.available, Amount::TWO);
        assert_eq!(account.total, Amount::TWO);

        Ok(())
    }

    #[test]
    fn chargeback_should_decrease_held_funds_increase_available_and_lock_acc(
    ) -> Result<(), AccountError> {
        let mut account = Account::new(1);

        let deposit = AccountDelta::deposit(Amount::TWO);
        account.apply(deposit)?;

        assert_eq!(account.available, Amount::TWO);
        assert_eq!(account.total, Amount::TWO);

        let dispute = AccountDelta::dispute_deposit(Amount::ONE);
        account.apply(dispute)?;

        assert_eq!(account.held, Amount::ONE);
        assert_eq!(account.available, Amount::ONE);
        assert_eq!(account.total, Amount::TWO);

        let chargeback = AccountDelta::chargeback(Amount::ONE);
        account.apply(chargeback)?;

        assert_eq!(account.held, Amount::ZERO);
        assert_eq!(account.available, Amount::ONE);
        assert_eq!(account.total, Amount::ONE);
        assert_eq!(account.locked, true);

        Ok(())
    }

    #[test]
    fn locked_acc_should_not_apply_any_change() -> Result<(), AccountError> {
        let mut account = Account::new(1);

        let deposit = AccountDelta::deposit(Amount::TWO);
        account.apply(deposit)?;

        assert_eq!(account.available, Amount::TWO);
        assert_eq!(account.total, Amount::TWO);

        let dispute = AccountDelta::dispute_deposit(Amount::TWO);
        account.apply(dispute)?;

        assert_eq!(account.held, Amount::TWO);
        assert_eq!(account.available, Amount::ZERO);
        assert_eq!(account.total, Amount::TWO);

        let chargeback = AccountDelta::chargeback(Amount::TWO);
        account.apply(chargeback)?;

        assert_eq!(account.held, Amount::ZERO);
        assert_eq!(account.available, Amount::ZERO);
        assert_eq!(account.total, Amount::ZERO);
        assert_eq!(account.locked, true);

        let deposit = AccountDelta::deposit(Amount::TWO);

        let result = account.apply(deposit);

        assert!(result.is_err());
        assert_eq!(account.held, Amount::ZERO);
        assert_eq!(account.available, Amount::ZERO);
        assert_eq!(account.total, Amount::ZERO);
        assert_eq!(account.locked, true);

        Ok(())
    }

    #[test]
    fn withdrawal_should_have_negative_available_amount_in_delta() {
        let withdrawal = AccountDelta::withdrawal(Amount::ONE);

        assert_eq!(
            withdrawal.available.unwrap_or_default(),
            Amount::NEGATIVE_ONE
        );
        assert_eq!(withdrawal.locked, None);
        assert_eq!(withdrawal.held, None);
    }

    #[test]
    fn deposit_should_have_positive_available_amount_in_delta() {
        let withdrawal = AccountDelta::deposit(Amount::ONE);

        assert_eq!(withdrawal.available.unwrap_or_default(), Amount::ONE);
        assert_eq!(withdrawal.locked, None);
        assert_eq!(withdrawal.held, None);
    }

    #[test]
    fn resolve_should_increment_available_and_decrement_held_funds_in_delta() {
        let withdrawal = AccountDelta::resolve(Amount::ONE);

        assert_eq!(withdrawal.available.unwrap_or_default(), Amount::ONE);
        assert_eq!(withdrawal.held.unwrap_or_default(), Amount::NEGATIVE_ONE);
        assert_eq!(withdrawal.locked, None);
    }
}
