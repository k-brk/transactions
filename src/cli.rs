use std::{ffi::OsStr, path::PathBuf};

use clap::Parser;

use crate::AppError;

#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct Args {
    #[clap(
        forbid_empty_values = true,
        value_parser,
        help = "A path to CSV file with transactions"
    )]
    pub transactions_file: PathBuf,
}

const FILE_EXT: &str = "csv";

pub fn validate_ext(args: &Args) -> Result<(), AppError> {
    let ext = args
        .transactions_file
        .extension()
        .and_then(OsStr::to_str)
        .ok_or(AppError::InvalidFileExt(
            "Unable to validate extension".to_string(),
        ))?;

    if ext != FILE_EXT {
        return Err(AppError::InvalidFileExt(ext.to_string()));
    }

    Ok(())
}
