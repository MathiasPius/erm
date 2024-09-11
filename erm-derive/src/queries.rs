use proc_macro2::Ident;
use syn::{spanned::Spanned, DataStruct};

fn column_names(data: &DataStruct) -> Vec<String> {
    let entity = Ident::new("entity", data.fields.span());

    [&entity]
        .into_iter()
        .chain(
            data.fields
                .iter()
                .map(|field| field.ident.as_ref().unwrap()),
        )
        .map(|ident| ident.to_string())
        .collect::<Vec<_>>()
}

/// Generates placeholder values corresponding to the number of columns.
fn placeholders(character: char, iter: impl Iterator) -> String {
    std::iter::repeat(character)
        .enumerate()
        .skip(1)
        .take(iter.count())
        .map(|(i, character)| format!("{character}{i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn insertion_query(table: &str, character: char, data: &DataStruct) -> String {
    let column_names = column_names(data);

    let placeholders = placeholders(character, column_names.iter());

    format!(
        "insert into {table}({column_names}) values({placeholders});",
        column_names = column_names.join(", ")
    )
}
