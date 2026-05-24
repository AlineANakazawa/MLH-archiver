//! Error types for the anonymizer.

use polars::prelude::PolarsError;
use thiserror::Error;

/// Errors that can occur during anonymizer configuration.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

/// Errors that can occur during anonymization.
#[derive(Error, Debug)]
pub enum AnonymizerError {
    #[error("Unsupported type for anonymization")]
    UnsupportedType,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Polars error: {0}")]
    Polars(#[from] PolarsError),
}
