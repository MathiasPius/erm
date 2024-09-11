use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DataStruct, Field};

fn idents(field: &Field) -> &Ident {
    field.ident.as_ref().unwrap()
}

fn names(field: &Field) -> String {
    idents(field).to_string()
}

fn type_info<'database, 'field>(
    database: &'database Ident,
) -> impl Fn(&'field Field) -> TokenStream + 'database {
    move |field: &'field Field| {
        let typename = &field.ty;
        quote! {
            <#typename as ::sqlx::Type<::sqlx::#database>>::type_info()
        }
    }
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
    let column_names = data.fields.iter().map(names).collect::<Vec<_>>();

    let placeholders = placeholders(character, column_names.iter());

    format!(
        "insert into {table}({column_names}) values({placeholders});",
        column_names = column_names.join(", ")
    )
}

pub fn create_query(database: &Ident, table: &str, data: &DataStruct) -> TokenStream {
    let columns = data.fields.iter().map(names);

    let type_info = data.fields.iter().map(type_info(database));

    let format_str = columns
        .map(|column| format!("{column} {{}} {{}}"))
        .collect::<Vec<_>>()
        .join(", ");

    let format_str = format!("create table {table}(entity {{}} primary key, {format_str}\n);");

    let arguments = quote! {
        <Entity as ::sqlx::Type<::sqlx::#database>>::type_info().name(),
        #(
            #type_info.name(),
            if #type_info.is_null() { "null" } else {"not null"},
        )*
    };

    quote! {
        format!(
            #format_str,
            #arguments
        )
    }
}
