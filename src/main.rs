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

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(input);



    Ok(())
}
