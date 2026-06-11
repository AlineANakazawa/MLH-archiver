import polars as pl
import os
import glob


def main(working_dir, output_dir):
    if not working_dir:
        print("Error: no input directory available.")
        return
 
    default_lists = "netdev,bpf,rust-for-linux"
    LISTS_OF_INTEREST = (os.environ.get("LISTS_OF_INTEREST") or default_lists).split(
        ","
    )
    LISTS_OF_INTEREST = [li for li in LISTS_OF_INTEREST if li]
 
    if not LISTS_OF_INTEREST:
        raw_dirs = glob.glob(f"{working_dir}/list=*")
        LISTS_OF_INTEREST = sorted(
            [os.path.basename(d).removeprefix("list=") for d in raw_dirs]
        )
        print(f"Using all available lists: {LISTS_OF_INTEREST}")
 
    for m_list in LISTS_OF_INTEREST:
        print(f"\nProcessing list: {m_list}")
 
        df = pl.read_parquet(f"{working_dir}/list={m_list}/*.parquet")
        df = df.with_columns(pl.lit(m_list).alias("list"))

        duplicates = (
            df.group_by(["message_id", "body_sha1"])
            .agg([
                pl.min("date").alias("date"),
                pl.count().alias("number_of_replicas"),
                pl.first("list").alias("list"),
            ])
            .filter(pl.col("number_of_replicas") > 1)
        )
 
        print(f"Found {len(duplicates)} duplicated messages in {m_list}")
        if len(duplicates) > 0:
            print(duplicates)
 
        output_path = os.path.join(output_dir, f"duplications_{m_list}.csv")
        duplicates.write_csv(output_path)
        print(f"Saved to {output_path}")
