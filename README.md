
## About

Simple application for managing transactions.

## Usage

```
cargo run -- xyz.csv
```

Output of command will be returned to stdout.

## Implementation

Delta based approach has been choosen, each transaction is converted to structure with changes(increased balance, account locked, etc.) which is later on applied to user account. By doing this way account is decoupled from transactions, rollback can be easily obtained and deltas can be used to recreate user balance upto any given point.


- `core/engine.rs`
    
    Engine is an entrypoint for each transaction.

    It has a transaction processor which converts incoming transaction into `AccountDelta` which
    describes how given transaction will affect user account and what changes needs to be applied (balances, locks).
     
    Once delta is generated, it is applied to user account to reflect changes from transaction.
    
    ```     
           Transaction
                │
        ┌───────▼────────┐
        │                 │
        │      Engine     │
        │                 │
        └───────┬─────────┘
                │ Transaction
                │
        ┌───────▼────────┐
        │                 │
        │   Transaction   │
        │    Processor    │
        │                 │
        └───────┬─────────┘
                │ AccountDelta
                │
        ┌───────▼────────┐
        │                 │
        │     Account     │
        │                 │
        └─────────────────┘
    ```
- `core/transaction_processor.rs`

    Transaction processor based on transaction kind creates structure with changes for user that needs to be applied in order to reflect incoming transaction.

    ```rust
    pub struct AccountDelta {
        pub available: Option<Amount>,
        pub held: Option<Amount>,
        pub locked: Option<bool>,
    }
    ```
    Any or all of fileds can be set to be applied later on on user account.

    For convenience, `AccountDelta` has several methods that are tailored for transactions to avoid mistakes. 
    
- `core/account.rs`

    Has a definition of `AccountDelta`, its helpers and user account `Account`. 
    
    `Account` model represents user account, has ability to apply changes from `AccountDelta` and it is used for serialization into output csv. 


- `core/transaction.rs`

    Has a definition of `Transaction`, and its kinds. File content is deserialized into this structure.    

- `core/*_store.rs`

    Simple memory storages for accounts and transactions

