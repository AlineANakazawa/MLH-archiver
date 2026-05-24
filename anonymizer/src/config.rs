//! Configuration loading from YAML/JSON/TOML files via glob matching.

use crate::errors::ConfigError;
use clap::{Parser, ValueHint};
use config::Config;
use glob::glob;

/// CLI arguments for the anonymizer binary.
#[derive(Debug, Parser)]
pub struct Opts {
    /// Glob pattern for config file(s). Defaults to `anonymizer_config*`.
    #[arg(short, long, default_value = "anonymizer_config*", value_hint = ValueHint::FilePath)]
    pub config_file: String,
}

/// Anonymizer configuration deserialized from a YAML/JSON/TOML file.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub struct AppConfig {
    /// Number of worker threads.
    pub nthreads: u8,
    /// Root directory containing mailing list subdirectories from the parser.
    pub input_dir_path: String,
    /// Root directory for anonymized output.
    pub output_dir_path: String,
    /// Specific mailing list directories to anonymize. `None` anonymizes all subdirectories.
    pub lists_to_anonymize: Option<Vec<String>>,
    /// dataset_compression_level: ZSTD levels between 1 and 22
    /// defaults to 12
    /// TODO: add validation. values above 22 should be refused
    pub compression_level: Option<usize>,
}

/// Reads configuration from files matching the glob pattern in [`Opts::config_file`].
pub fn read_config() -> Result<AppConfig, ConfigError> {
    let opts = Opts::parse();

    let config_files: Vec<_> = glob(&opts.config_file)
        .map_err(|e| {
            ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid config file glob pattern '{}': {}",
                    opts.config_file, e
                ),
            ))
        })?
        .filter_map(|path_result| match path_result {
            Ok(path) => {
                log::debug!("Found config file: {}", path.display());
                Some(config::File::from(path))
            }
            Err(e) => {
                log::warn!("Error reading config file path: {}", e);
                None
            }
        })
        .collect();

    if config_files.is_empty() {
        log::warn!(
            "No config files found matching pattern: {}",
            opts.config_file
        );
    }

    let mut config_builder = Config::builder();

    for config_file in config_files {
        config_builder = config_builder.add_source(config_file);
    }

    let config = config_builder.build().map_err(|e| {
        ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to build config: {}", e),
        ))
    })?;

    let app_config: AppConfig = config.try_deserialize().map_err(|e| {
        ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to deserialize config: {}", e),
        ))
    })?;

    Ok(app_config)
}
