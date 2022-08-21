use crate::core::engine::Engine;
use crate::core::{account_store::AccountStore, transaction_store::TransactionStore};
use std::fs::File;
use std::io;

use clap::Parser;
use cli::validate_ext;

use crate::core::transaction::Transaction;

mod cli;
mod core;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    OpenFileError(#[from] std::io::Error),
    #[error("{0}")]
    InvalidFileExt(String),
    #[error("{0}")]
    DeserializeTransactionError(#[from] csv::Error),
}

fn main() -> Result<(), AppError> {
    let args = cli::Args::parse();
    validate_ext(&args)?;

    let input = File::open(args.transactions_file)?;

    let mut engine = Engine::<TransactionStore, AccountStore>::default();
    //FIXME: Trailing comma
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(input);

    for result in rdr.deserialize() {
        let result: Transaction = match result {
            Ok(t) => t,
            Err(err) => {
                println!("{}", err);
                continue;
            }
        };

        engine.process_transaction(result);
    }

    let mut wtr = csv::Writer::from_writer(io::stdout());
    engine
        .accounts()
        .values()
        .for_each(|acc| wtr.serialize(acc).unwrap());

    Ok(())
}

//TODO:
// Test engine (transaction state)
// Multithreading
// Main
// Trailing comma issue in csv
// Readme
// comments
// Error handling
// Precision 