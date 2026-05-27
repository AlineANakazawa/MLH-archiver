import polars as pl
import seaborn as sns
import matplotlib.pyplot as plt
import os
import glob

sns.set_style("whitegrid")

def main(working_dir, output_dir):
    if not working_dir:
        print("Error: no input directory available.")
        return

    default_lists = "netdev,bpf,rust-for-linux"
    LISTS_OF_INTEREST = (os.environ.get("LISTS_OF_INTEREST") or default_lists).split(",")
    LISTS_OF_INTEREST = [li for li in LISTS_OF_INTEREST if li]

    if not LISTS_OF_INTEREST:
        raw_dirs = glob.glob(f"{working_dir}/list=*")
        LISTS_OF_INTEREST = sorted(
            [os.path.basename(d).removeprefix("list=") for d in raw_dirs]
        )
        print(f"Using all available lists: {LISTS_OF_INTEREST}")

    # Generate merged DataFrame os lists
    df_array = []
    for m_list in LISTS_OF_INTEREST:
        new_list_df = pl.read_parquet(f"{working_dir}/list={m_list}/*.parquet")
        new_list_df = new_list_df.with_columns(pl.lit(m_list).alias("list"))
        df_array.append(new_list_df)
    df = pl.concat(df_array)

    # Filter out non-patches
    df = df.filter(
        (pl.col("has_patch_tag") | pl.col("has_rfc_tag")) &
        (~pl.col("has_response_tag")) & (~pl.col("has_forward_tag")) &
        (pl.col("untagged_subject").is_not_null() & (pl.col("untagged_subject") != ""))
    )

    # Group by mailing list (plot unit) and untagged_subject (data point unit)
    df = df.group_by(["list", "untagged_subject"]).agg(
        pl.col("date").min().alias("min_date"),
        pl.col("date").max().alias("max_date"),
        pl.len().alias("rev_count")
    )

    # Calculate time difference between last and first version
    df = df.with_columns(
        (pl.col("max_date") - pl.col("min_date")).dt.total_days().alias("time_diff_days")
    )

    # Plot

    ## Time difference between first and last versions
    df_time_diff = df.filter(pl.col("time_diff_days") > 0)
    df_time_diff = df_time_diff.to_pandas()

    plt.figure(figsize=(10, 6))
    sns.violinplot(data=df_time_diff, x="list", y="time_diff_days", log_scale=True, inner="quartile")
    plt.xlabel("Mailing List")
    plt.ylabel("Time Difference (Days)")
    plt.title("Time Difference Between First and Last Patch Versions")
    plt.tight_layout()

    if output_dir:
        os.makedirs(output_dir, exist_ok=True)
        plt.savefig(f"{output_dir}/revisions_latencies.svg")

    ## Max versions
    df_versions = df.to_pandas()

    plt.figure(figsize=(10, 6))
    sns.violinplot(data=df_versions, x="list", y="rev_count", inner="quartile")
    plt.xlabel("Mailing List")
    plt.ylabel("Maximum Patch Version")
    plt.title("Distribution of Maximum Patch Versions")
    plt.tight_layout()

    if output_dir:
        os.makedirs(output_dir, exist_ok=True)
        plt.savefig(f"{output_dir}/revisions_versions.svg")
