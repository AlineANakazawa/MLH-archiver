from mlh_analysis import list_comparison
from mlh_analysis import list_sizes
from mlh_analysis import unique_authors
from mlh_analysis import date_analysis
from mlh_analysis.inputs import resolve_inputs

import os


def main():
    input_dirs = os.environ.get("INPUT_DIR", "").split(",")
    output_dir = os.environ.get("OUTPUT_DIR", "results")

    inputs = resolve_inputs(input_dirs)

    print("Starting list_comparison...")
    list_comparison.main(inputs["dataset_dir"], output_dir)

    print("Starting list_sizes...")
    list_sizes.main(inputs["dataset_dir"], output_dir)

    print("Starting unique_authors...")
    unique_authors.main(inputs["id_map_dir"], output_dir)

    print("Starting date_analysis...")
    date_analysis.main(inputs["dataset_dir"], output_dir)


if __name__ == "__main__":
    main()

