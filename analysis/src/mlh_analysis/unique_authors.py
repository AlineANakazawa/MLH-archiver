import polars as pl


def main(id_map_dir, output_dir):
    df = pl.scan_parquet(id_map_dir)

    df = df.group_by(["__original_from", "from"]).agg(pl.col("list")).collect()

    df.write_parquet(f"{output_dir}/unique_linux_authors.parquet")
