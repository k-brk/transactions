use crate::core::engine::Engine;
use crate::core::{account_store::AccountStore, transaction_store::TransactionStore};
use std::fs::File;
use std::io::{self, Stdout};

use clap::Parser;
use cli::validate_ext;
use csv::{Reader, Writer};

mod cli;
mod core;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    OpenFileError(#[from] std::io::Error),
    #[error("{0}")]
    InvalidFileExt(String),
}

fn main() -> Result<(), AppError> {
    let args = cli::Args::parse();
    validate_ext(&args)?;

    let file = File::open(args.transactions_file)?;

    let input = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(file);

    let output = csv::WriterBuilder::new()
        .flexible(true)
        .from_writer(io::stdout());

    worker(input, output);

    Ok(())
}

pub fn worker(mut input: Reader<File>, mut output: Writer<Stdout>) {
    let mut engine = Engine::<TransactionStore, AccountStore>::default();

    input
        .deserialize()
        .into_iter()
        .flatten()
        .for_each(|t| engine.process_transaction(t));

    engine.accounts().values().for_each(|acc| {
        output
            .serialize(acc)
            .unwrap_or_else(|err| log::error!("{}", err))
    });
}
