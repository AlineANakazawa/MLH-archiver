//! DataFrame transformation — anonymizes PII columns using Polars.

use std::borrow::Cow;

use crate::Result;
use crate::anonymizer::maybe_anonymize;
use crate::constants::{ANONYMIZE_MAP_COLUMNS, ANONYMIZE_STR_COLUMNS, SPLIT_DATASET_COLUMN};
use crate::errors::AnonymizerError;
use polars::prelude::*;

/// Transform a DataFrame by anonymizing PII columns.
pub fn anonymize_dataframe(mut df: DataFrame) -> Result<DataFrame> {
    for col_name in ANONYMIZE_STR_COLUMNS {
        if let Some(idx) = df.get_column_index(col_name) {
            let s = df.column(col_name)?.as_materialized_series().clone();
            let anon = anonymize_series(&s)?;
            df.replace_column(idx, Column::from(anon))?;
        }
    }

    for (parent_col, child_key) in ANONYMIZE_MAP_COLUMNS {
        if let Some(idx) = df.get_column_index(parent_col) {
            let s = df.column(parent_col)?.as_materialized_series().clone();
            let anon = anonymize_trailers(&s, child_key)?;
            df.replace_column(idx, Column::from(anon))?;
        }
    }

    Ok(df)
}

/// Build an ID-map DataFrame from the original `from` column.
pub fn build_id_map(df: &DataFrame) -> Result<DataFrame> {
    let from_col = df.column(SPLIT_DATASET_COLUMN).map_err(|_| {
        AnonymizerError::Config(format!("Column '{}' not found", SPLIT_DATASET_COLUMN))
    })?;

    let from_ca = from_col.as_materialized_series().str()?;
    let hashed: StringChunked = from_ca.apply_values(|v| maybe_anonymize(v));

    let out = df![
        format!("__original_{}", SPLIT_DATASET_COLUMN) => from_col.as_materialized_series().clone(),
        SPLIT_DATASET_COLUMN => hashed.into_series(),
    ]?;

    Ok(out)
}

fn anonymize_series(s: &Series) -> Result<Series> {
    match s.dtype() {
        DataType::String => {
            let ca = s.str()?;
            let anon: StringChunked = ca.apply_values(|v| maybe_anonymize(v));
            Ok(anon.into_series())
        }
        DataType::List(inner) if matches!(inner.as_ref(), DataType::String) => {
            anonymize_list_str(s)
        }
        _ => Ok(s.clone()),
    }
}

fn anonymize_list_str(s: &Series) -> Result<Series> {
    let list_ca = s.list()?;
    let total_values: usize = (0..list_ca.len())
        .map(|i| list_ca.get_as_series(i).map(|s| s.len()).unwrap_or(0))
        .sum();

    let mut builder = ListStringChunkedBuilder::new(s.name().clone(), list_ca.len(), total_values);

    for i in 0..list_ca.len() {
        match list_ca.get_as_series(i) {
            Some(sub_s) => {
                let sub_str = sub_s.str().map_err(|e| {
                    AnonymizerError::Config(format!("Expected string in list: {}", e))
                })?;
                let vals: Vec<Cow<str>> = sub_str
                    .into_iter()
                    .map(|opt| match opt {
                        Some(v) => maybe_anonymize(v),
                        None => Cow::Borrowed(""),
                    })
                    .collect();
                builder.append_values_iter(vals.iter().map(|c| c.as_ref()));
            }
            None => {
                builder.append_null();
            }
        }
    }

    Ok(builder.finish().into_series())
}

fn anonymize_trailers(s: &Series, child_key: &str) -> Result<Series> {
    let list_ca = s.list()?;
    let inner = list_ca.get_inner();
    let inner_struct = inner.struct_().map_err(|e| {
        AnonymizerError::Config(format!("{} inner is not a struct: {}", s.name(), e))
    })?;

    let ident = inner_struct.field_by_name(child_key).map_err(|_| {
        AnonymizerError::Config(format!("{} missing '{}' field", s.name(), child_key))
    })?;
    let ident_str = ident.str()?;
    let anon_ident: StringChunked = ident_str.apply_values(|v| maybe_anonymize(v));

    let mut new_fields: Vec<Series> = Vec::new();
    for field in inner_struct.fields_as_series() {
        if field.name() == child_key {
            new_fields.push(anon_ident.clone().into_series().with_name(child_key.into()));
        } else {
            new_fields.push(field);
        }
    }

    let new_struct = StructChunked::from_series(
        PlSmallStr::from_str(inner_struct.name()),
        inner_struct.len(),
        new_fields.iter(),
    )
    .map_err(|e| AnonymizerError::Config(format!("Failed to build struct: {}", e)))?;

    let new_inner = new_struct.into_series().rechunk();
    let offsets = list_ca.offsets()?.clone();
    let inner_arr = new_inner.array_ref(0).clone();
    let inner_dtype = inner_arr.dtype().clone();

    use polars_arrow::array::ListArray;
    use polars_arrow::datatypes::{ArrowDataType, Field};

    let item_field = Field::new(PlSmallStr::from_static("item"), inner_dtype, true);

    let arr = ListArray::<i64>::try_new(
        ArrowDataType::LargeList(Box::new(item_field)),
        offsets,
        inner_arr,
        None,
    )
    .map_err(|e| AnonymizerError::Config(format!("Failed to build list array: {}", e)))?;

    let new_list = ListChunked::with_chunk(PlSmallStr::from_str(s.name()), arr);
    Ok(new_list.into_series())
}
