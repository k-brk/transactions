use std::collections::HashMap;

use super::account::{Account, ClientID};

pub trait Store {
    type Storage;

    /// Returns existing account or creates a new if does not exist
    fn get_mut_or_new(&mut self, client_id: ClientID) -> &mut Account;

    /// Returns all existing accounts
    fn accounts(&self) -> &Self::Storage;
}

#[derive(Default)]
pub struct AccountStore {
    accounts: HashMap<ClientID, Account>,
}

impl Store for AccountStore {
    type Storage = HashMap<ClientID, Account>;

    fn get_mut_or_new(&mut self, client_id: ClientID) -> &mut Account {
        self.accounts
            .entry(client_id)
            .or_insert(Account::new(client_id))
    }

    fn accounts(&self) -> &Self::Storage {
        return &self.accounts;
    }
}

#[cfg(test)]
mod tests {
    use super::AccountStore;
    use super::Store;

    #[test]
    fn returns_new_account_if_not_exists() {
        let client_id = 1;
        let mut store = AccountStore::default();

        assert!(!store.accounts().contains_key(&client_id));

        let _acc = store.get_mut_or_new(client_id);

        assert!(store.accounts().contains_key(&client_id));
    }
}
