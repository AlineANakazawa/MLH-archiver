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
    selected_lists = (os.environ.get("LISTS_OF_INTEREST") or default_lists).split(",")
    selected_lists = [li for li in selected_lists if li]

    if not selected_lists:
        raw_dirs = glob.glob(f"{working_dir}/list=*")
        selected_lists = sorted(
            [os.path.basename(d).removeprefix("list=") for d in raw_dirs]
        )
        print(f"Using all available lists: {selected_lists}")

    # Generate merged DataFrame os lists
    df_array = []
    for m_list in selected_lists:
        new_list_df = pl.read_parquet(f"{working_dir}/list={m_list}/*.parquet")
        new_list_df = new_list_df.with_columns(pl.lit(m_list).alias("list"))
        df_array.append(new_list_df)
    df = pl.concat(df_array)

    # Filter out non-patches
    df = df.filter(
        (pl.col("has_patch_tag") | pl.col("has_rfc_tag"))
        & (~pl.col("has_response_tag"))
        & (~pl.col("has_forward_tag"))
        & (
            pl.col("untagged_subject").is_not_null()
            & (pl.col("untagged_subject") != "")
        )
    )

    # Group by mailing list (plot unit) and untagged_subject (data point unit)
    df = (
        df.group_by(["list", "untagged_subject"])
        .agg(
            pl.col("date").min().alias("min_date"),
            pl.col("date").max().alias("max_date"),
            pl.len().alias("rev_count"),
        )
        .sort("list")
    )

    # Calculate time difference between last and first version
    df = df.with_columns(
        (pl.col("max_date") - pl.col("min_date"))
        .dt.total_days()
        .alias("time_diff_days")
    )

    # Plot

    ## Time difference between first and last versions
    df_time_diff = df.filter(pl.col("time_diff_days") > 0)
    df_time_diff = df_time_diff.to_pandas()

    plt.figure(figsize=(10, 4))
    sns.violinplot(
        data=df_time_diff,
        y="list",
        x="time_diff_days",
        log_scale=True,
        inner="quartile",
        hue="list",
    )
    plt.ylabel("Mailing List")
    plt.xlabel("Time Difference (Days)")
    plt.title("Time Difference Between First and Last Patch Versions")
    plt.tight_layout()

    if output_dir:
        os.makedirs(output_dir, exist_ok=True)
        plt.savefig(f"{output_dir}/revisions_latencies.svg")

    ## Max versions
    df_versions = df.to_pandas()

    plt.figure(figsize=(10, 4))
    sns.violinplot(
        data=df_versions, y="list", x="rev_count", inner="quartile", hue="list"
    )
    plt.ylabel("Mailing List")
    plt.xlabel("Maximum Patch Version")
    plt.xlim(1, 10)
    plt.title("Distribution of Maximum Patch Versions")
    plt.tight_layout()

    if output_dir:
        os.makedirs(output_dir, exist_ok=True)
        plt.savefig(f"{output_dir}/revisions_versions_violin.svg")

    plt.figure(figsize=(10, 4))
    sns.boxplot(data=df_versions, y="list", x="rev_count", hue="list")
    plt.ylabel("Mailing List")
    plt.xlabel("Maximum Patch Version")
    plt.xlim(1, 10)
    plt.title("Distribution of Maximum Patch Versions")
    plt.tight_layout()

    if output_dir:
        os.makedirs(output_dir, exist_ok=True)
        plt.savefig(f"{output_dir}/revisions_versions_box.svg")
