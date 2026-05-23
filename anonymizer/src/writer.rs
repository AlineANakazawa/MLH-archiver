//! Parquet file writer using Polars with ZSTD compression and row group control.

use crate::Result;
use crate::constants::BATCH_MAX_RECORDS;
use polars::prelude::*;
use polars_utils::compression::ZstdLevel;
use std::fs;
use std::path::Path;

/// Write a DataFrame to a Parquet file with ZSTD compression and row group control.
/// Each row group contains at most `BATCH_MAX_RECORDS` rows.
pub fn write_parquet(path: &Path, compression: usize, df: &mut DataFrame) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let zstd_level = if compression == 0 {
        None
    } else {
        Some(
            ZstdLevel::try_new(compression as i32)
                .map_err(|e| format!("invalid zstd level {compression}: {e}"))?,
        )
    };

    let file = fs::File::create(path)?;
    ParquetWriter::new(file)
        .with_compression(ParquetCompression::Zstd(zstd_level))
        .with_row_group_size(Some(BATCH_MAX_RECORDS))
        .finish(df)?;
    Ok(())
}
