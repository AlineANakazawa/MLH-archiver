//! Parquet file reader using Polars.

use crate::Result;
use polars::prelude::*;
use std::fs;
use std::path::Path;

/// Discover all `.parquet` files in the given directory (non-recursive).
pub fn discover_parquet_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "parquet") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Read a single Parquet file into a Polars DataFrame.
pub fn read_parquet_file(path: &Path) -> Result<DataFrame> {
    let file = fs::File::open(path)?;
    let df = ParquetReader::new(file).finish()?;
    Ok(df)
}

/// Read all Parquet files in a directory into a single DataFrame by concatenation.
pub fn read_parquet_dir(dir: &Path) -> Result<DataFrame> {
    let files = discover_parquet_files(dir)?;
    let dfs: Result<Vec<DataFrame>> = files.iter().map(|p| read_parquet_file(p)).collect();
    let dfs = dfs?;
    if dfs.is_empty() {
        return Err("No parquet files found".into());
    }
    let mut combined = dfs[0].clone();
    for df in &dfs[1..] {
        combined.vstack_mut(df)?;
    }
    Ok(combined)
}
