import os


def resolve_inputs(input_dirs):
    """Resolve dataset, lineage, and id_map directories from a list of input directories.

    Returns a dict with keys 'dataset', 'anon_dataset', 'lineage', and 'id_map'.
    """
    dataset_dir = None
    anon_dataset_dir = None
    lineage_dir = None
    id_map_dir = None

    for d in input_dirs:
        d = d.strip()
        if not d or not os.path.isdir(d):
            continue

        entries = os.listdir(d)

        if "id_map_from" in entries and id_map_dir is None:
            id_map_dir = os.path.join(d, "id_map_from")

        if lineage_dir is None:
            if os.path.isfile(os.path.join(d, "lineage.parquet")):
                lineage_dir = d

        has_list_dirs = any(e.startswith("list=") for e in entries)
        if dataset_dir is None and has_list_dirs:
            dataset_dir = d

        if "anonymizer" in d and anon_dataset_dir is None and "dataset" in entries:
            anon_dataset_dir = os.path.join(d, "dataset")

        # output/parser/dataset
        # if missing, use the anonimyzed in its place
        if "parser" in d and dataset_dir is None and not has_list_dirs:
            if "dataset" in entries:
                dataset_dir = os.path.join(d, "dataset")

    if dataset_dir is None and anon_dataset_dir is None:
        raise FileNotFoundError(
            f"No dataset directory found in: {input_dirs}. "
            "Expected 'list=*/' subdirectories, 'dataset/'"
        )

    return {
        "dataset": dataset_dir or anon_dataset_dir or "",
        "anon_dataset": anon_dataset_dir or "",
        "lineage": lineage_dir or "",
        "id_map": id_map_dir or "",
    }
