from datafusion import SessionContext
try:
    import readline # noqa: F401
except: # noqa: E722
    # will use default pytyhon input othwise
    pass


def main(input_map, output_dir):
    ctx = SessionContext()
    print(input_map)
    for name, data_path in input_map.items():
        ctx.register_parquet(name, data_path)

    df = ctx.sql("show tables")
    df.show()

    print(f"\nSchema for {name}")
    for name in input_map.keys():
        df = ctx.sql(f"show columns from {name};")
        df.show()

    df = None
    try:
        query = input("Enter the SQL query:\n ")
        df = ctx.sql(query)
        df.show()
    except Exception as e:
        print(type(e))
        if "datafusion" in str(e):
            print(f"Caught a DataFusion-specific error: {e}")
        else:
            print(f"An unexpected error occurred: {e}")

    df.write_csv(output_dir)

