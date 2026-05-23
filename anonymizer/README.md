# MLH Anonymizer

A Rust tool for pseudo-anonymizing personal identification data in mailing list datasets using Polars.

## Overview

The MLH Anonymizer processes Parquet datasets produced by the MLH Parser and replaces personally identifiable information (PII) with SHA1 digests. This enables analysis of mailing list data while protecting user privacy.

![Anonymizer Diagram](/docs/anonymizer.avif)

## Features

- **SHA1 Hashing**: Replaces email addresses and names with consistent hashes
- **Deterministic**: Same input always produces the same hash, enabling longitudinal analysis
- **Polars-Powered**: Fast columnar transformations using the Polars DataFrame library
- **Parallel Processing**: Multi-threaded using Rayon — one thread per mailing list, batched within each list
- **Hive-Partitioned Output**: Anonymized data written under `dataset/list=<name>/` and identity map under `id_map_from/list=<name>/`
- **Configurable**: YAML configuration for thread count, I/O paths, list selection, and batch size

## How It Works

The anonymizer applies SHA1 hashing to personal identification fields:

```
Original:  user@example.com
           ↓
Anonymized: 63a710569261a24b3766275b7000ce8d7b32e2f7

Original: Name <user@example.com>
           ↓
Anonymized: 709a23220f2c3d64d1e1d6d18c4d5280f8d82fca <63a710569261a24b3766275b7000ce8d7b32e2f7>
```

The same email address always produces the same hash, allowing you to:
- Track user activity across multiple emails
- Perform user-level analytics
- Maintain data utility while protecting privacy

## Prerequisites

### Required
- Rust/Cargo, or
- Podman with Podman Compose, or Docker with Docker Compose (for containerized builds)

### Native Development (Optional)
- Rust toolchain (rustup)
- [Devbox](https://www.jetify.com/devbox/) (recommended)

## Installation

### Using Devbox (Recommended)

```bash
devbox shell
```

This sets up Rust, Python, and all required dependencies automatically.

### Manual Setup

Install the Rust toolchain via [rustup](https://rustup.rs/):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Usage

### Running the Anonymizer

The anonymizer expects the parsed Parquet dataset from the MLH Parser.

```bash
# Using Make (builds and runs)
make anonymize

# Using Devbox
devbox run anonymize

# Debug mode (verbose logging)
RUST_LOG=debug cargo run
```

### Configuration

The anonymizer uses a YAML configuration file. Copy the example and edit:

```bash
cp example_anonymizer_config.yaml anonymizer_config.yaml
```

```yaml
nthreads: 2
input_dir_path: "./output/parser/dataset/"
output_dir_path: "./output/anonymizer/"
lists_to_anonymize: 
  # - linux-api    # Comment out to process all lists
  # - lkml
```

| Field | Description |
|-------|-------------|
| `nthreads` | Number of worker threads (one mailing list per thread) |
| `input_dir_path` | Directory containing parsed Parquet files |
| `output_dir_path` | Root output directory for anonymized files |
| `lists_to_anonymize` | Optional list of mailing list names to process (omit for all) |

## Output Format

```
output/anonymizer/
├── dataset/
│   ├── list=dev.rcpassos.me.lists.gfs2/
│   │   └── list_data.parquet
│   ├── list=dev.rcpassos.me.lists.iommu/
│   │   └── list_data.parquet
│   └── ...
└── id_map_from/
    ├── list=dev.rcpassos.me.lists.gfs2/
    │   └── list_data.parquet
    └── ...
```

Each `list_data.parquet` is a single file with multiple row groups (controlled by `BATCH_MAX_RECORDS = 50_000`).

### Anonymized Fields

| Original Field | Type | Anonymized Form |
|----------------|------|-----------------|
| `from` | String | SHA1 hash |
| `to` | List\<String\> | Each element: SHA1 hash |
| `cc` | List\<String\> | Each element: SHA1 hash |
| `raw_body` | String | Inline identities: SHA1 hash |
| `trailers.identification` | List\<Struct\> | `identification` field: SHA1 hash |

String columns are processed with `Series::apply_values`. List columns (`to`, `cc`) use `ListStringChunkedBuilder`. Struct-list columns (`trailers`) use `StructChunked::from_series` + `ListArray` reconstruction.

## Development

### Running Tests

```bash
# Using Make
make test-anonymizer

# Using Devbox
devbox run test-anonymizer

# Native with cargo
cargo test
```

### Debug Mode

```bash
RUST_LOG=debug cargo run
```

### Project Structure

```
anonymizer/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Orchestration (start, process_mailing_list)
│   ├── transform.rs     # Core anonymization logic
│   ├── reader.rs        # Parquet reading (Polars)
│   ├── writer.rs        # Parquet writing (Polars with row group control)
│   ├── anonymizer.rs    # SHA1 hashing engine
│   ├── config.rs        # YAML configuration loading
│   ├── constants.rs     # Column lists, batch sizes, schemas
│   └── errors.rs        # Error types
├── tests/               # Integration and unit tests
├── Cargo.toml           # Rust project configuration
└── Makefile             # Build automation
```

## Dependencies

### Runtime
- `polars` (0.53) — Columnar data processing and Parquet I/O
- `rayon` (1.12) — Parallel processing per mailing list
- `sha1` (0.10) — Deterministic hashing
- `regex` (1) — Email/identity extraction
- `clap` (4.6) — CLI argument parsing
- `config` (0.15) — YAML configuration loading

## Integration with Other Components

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  MLH Archiver   │ ──► │   MLH Parser    │ ──► │   Anonymizer    │
│  (raw emails)   │     │  (Parquet DS)   │     │ (anonymized DS) │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                        │
                                                        ▼
                                               ┌─────────────────┐
                                               │    Analysis     │
                                               └─────────────────┘
```

Full pipeline:
```bash
make run        # Archive emails
make parse      # Parse to Parquet
make anonymize  # Anonymize data
make analysis   # Run analysis
```

## Example Usage with Polars (Python)

```python
import polars as pl

# Read the anonymized dataset
df = pl.scan_parquet("../output/anonymizer/dataset/**/*.parquet")

# Count emails per anonymized user
result = (
    df
    .group_by("from")
    .agg(pl.len().alias("email_count"))
    .sort("email_count", descending=True)
    .limit(10)
    .collect()
)
```

## Cleaning Up

```bash
# Remove build artifacts
make clean

# This runs: cargo clean
```

## Troubleshooting

### "Input directory is missing or empty"
Run the parser first to generate the Parquet dataset:
```bash
make parse
```

### Memory Issues
For large datasets, consider:
- Reducing `nthreads` in the config to process fewer lists concurrently
- Processing a subset of lists using `lists_to_anonymize`
- Lowering `BATCH_MAX_RECORDS` in `constants.rs`

## License

See the root [LICENSE](../LICENSE) file.
