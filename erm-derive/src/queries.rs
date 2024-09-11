use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DataStruct, Field, Fields};

fn idents(field: &Field) -> &Ident {
    field.ident.as_ref().unwrap()
}

fn names(field: &Field) -> String {
    idents(field).to_string()
}

fn type_info<'database, 'field>(
    database: &'database TokenStream,
) -> impl Fn(&'field Field) -> TokenStream + 'database {
    move |field: &'field Field| {
        let typename = &field.ty;
        quote! {
            <#typename as ::sqlx::Type<#database>>::type_info()
        }
    }
}

/// Generates placeholder values corresponding to the number of columns.
fn placeholders(character: char, count: usize) -> String {
    std::iter::repeat(character)
        .enumerate()
        .skip(1)
        .take(count)
        .map(|(i, character)| format!("{character}{i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn insert_component(table: &str, character: char, data: &DataStruct) -> String {
    let column_names = data.fields.iter().map(names).collect::<Vec<_>>();

    let placeholders = placeholders(character, column_names.len() + 1);

    format!(
        "insert into {table}(entity, {column_names}) values({placeholders});",
        column_names = column_names.join(", ")
    )
}

pub fn insert_archetype(database: &TokenStream, fields: &Fields) -> TokenStream {
    let sub_archetypes = fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        let typename = &field.ty;

        quote! {
            <#typename as ::erm::Archetype<#database>>::insert_archetype(&self.#name, query);
        }
    });

    quote! {
        fn insert_archetype<'query, Entity>(&'query self, query: &mut ::erm::InsertionQuery<'query, #database, Entity>)
        where
            Entity: sqlx::Encode<'query, #database> + sqlx::Type<#database> + Clone + 'query
        {
            #(#sub_archetypes)*
        }
    }
}

pub fn create_table(database: &TokenStream, table: &str, data: &DataStruct) -> TokenStream {
    let columns = data.fields.iter().map(names);

    let type_info = data.fields.iter().map(type_info(database));

    let format_str = columns
        .map(|column| format!("{column} {{}} {{}}"))
        .collect::<Vec<_>>()
        .join(", ");

    let format_str = format!("create table {table}(entity {{}} primary key, {format_str}\n);");

    let arguments = quote! {
        <Entity as ::sqlx::Type<#database>>::type_info().name(),
        #(
            #type_info.name(),
            if #type_info.is_null() { "null" } else {"not null"},
        )*
    };

    quote! {
        fn create_table<'pool, Entity>(
            pool: &'pool ::sqlx::Pool<#database>,
        ) -> impl ::core::future::Future<Output = Result<<#database as ::sqlx::Database>::QueryResult, ::sqlx::Error>> + Send
        where
            Entity: for<'q> sqlx::Encode<'q, #database> + sqlx::Type<#database> + Clone,
        {
            use ::sqlx::TypeInfo as _;
            use ::sqlx::Executor as _;

            static SQL: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
            let sql = SQL.get_or_init(||
                format!(
                    #format_str,
                    #arguments
                )
            ).as_str();

            pool.execute(sql)
        }
    }
}

pub fn select_query(database: &TokenStream, fields: &Fields) -> TokenStream {
    let mut fields = fields.iter();

    let first_item = &fields.next().unwrap().ty;
    let first = quote! {
        let join = <#first_item as Archetype<#database>>::select_statement();
    };

    let select_statements = fields.map(|field| {
        let field = &field.ty;

        quote! {
            let join = ::erm::cte::InnerJoin {
                left: (
                    Box::new(join),
                    "entity".to_string(),
                ),
                right: (
                    Box::new(<#field as Archetype<#database>>::select_statement()),
                    "entity".to_string(),
                ),
            }
        }
    });

    quote! {
        fn select_statement() -> impl ::erm::cte::CommonTableExpression {
            #first;
            #(#select_statements;)*

            join
        }
    }
}
