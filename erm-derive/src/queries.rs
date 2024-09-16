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
fn placeholders(character: char, count: usize) -> Vec<String> {
    std::iter::repeat(character)
        .enumerate()
        .skip(1)
        .take(count)
        .map(|(i, character)| format!("{character}{i}"))
        .collect::<Vec<_>>()
}

pub fn insert_component(table: &str, character: char, data: &DataStruct) -> String {
    let column_names = data.fields.iter().map(names).collect::<Vec<_>>();

    let placeholders = placeholders(character, column_names.len() + 1).join(", ");

    format!(
        "insert into {table}(entity, {column_names}) values({placeholders});",
        column_names = column_names.join(", ")
    )
}

pub fn update_component(table: &str, character: char, data: &DataStruct) -> String {
    let column_names = data.fields.iter().map(names);

    let placeholders = placeholders(character, column_names.len() + 1)
        .into_iter()
        .skip(1);

    let field_updates = column_names
        .zip(placeholders)
        .map(|(column, placeholder)| format!("{column} = {placeholder}"))
        .collect::<Vec<_>>()
        .join(", ");

    format!("update {table} set {field_updates} where entity = {character}1")
}

pub fn delete_component(table: &str, character: char) -> String {
    format!("delete from {table} where entity = {character}1")
}

pub fn create_archetype_component_tables(database: &TokenStream, fields: &Fields) -> TokenStream {
    let sub_archetypes = fields.iter().map(|field| {
        let typename = &field.ty;

        quote! {
            <#typename as ::erm::archetype::Archetype<#database>>::create_component_tables::<Entity>(pool).await?;
        }
    });

    quote! {
        fn create_component_tables<'a, Entity>(
            pool: &'a ::sqlx::Pool<#database>,
        ) -> impl ::std::future::Future<Output = Result<(), ::sqlx::Error>> + Send + 'a where Entity: ::sqlx::Type<#database> {

            async move {
                #(#sub_archetypes)*

                Ok(())
            }
        }
    }
}

pub fn insert_archetype(database: &TokenStream, fields: &Fields) -> TokenStream {
    let sub_archetypes = fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        let typename = &field.ty;

        quote! {
            <#typename as ::erm::archetype::Archetype<#database>>::insert_archetype(&self.#name, query);
        }
    });

    quote! {
        fn insert_archetype<'query, Entity>(&'query self, query: &mut ::erm::entity::EntityPrefixedQuery<'query, #database, Entity>)
        where
            Entity: sqlx::Encode<'query, #database> + sqlx::Type<#database> + Clone + 'query
        {
            #(#sub_archetypes)*
        }
    }
}

pub fn update_archetype(database: &TokenStream, fields: &Fields) -> TokenStream {
    let sub_archetypes = fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        let typename = &field.ty;

        quote! {
            <#typename as ::erm::archetype::Archetype<#database>>::update_archetype(&self.#name, query);
        }
    });

    quote! {
        fn update_archetype<'query, Entity>(&'query self, query: &mut ::erm::entity::EntityPrefixedQuery<'query, #database, Entity>)
        where
            Entity: sqlx::Encode<'query, #database> + sqlx::Type<#database> + Clone + 'query
        {
            #(#sub_archetypes)*
        }
    }
}

pub fn delete_archetype(database: &TokenStream, fields: &Fields) -> TokenStream {
    let sub_archetypes = fields.iter().map(|field| {
        let typename = &field.ty;

        quote! {
            <#typename as ::erm::archetype::Archetype<#database>>::delete_archetype(query);
        }
    });

    quote! {
        fn delete_archetype<'query, Entity>(query: &mut ::erm::entity::EntityPrefixedQuery<'query, #database, Entity>)
        where
            Entity: sqlx::Encode<'query, #database> + sqlx::Type<#database> + Clone + 'query
        {
            #(#sub_archetypes)*
        }
    }
}

pub fn create_component_table(
    database: &TokenStream,
    table: &str,
    data: &DataStruct,
) -> TokenStream {
    let columns = data.fields.iter().map(names);

    let type_info = data.fields.iter().map(type_info(database));

    let format_str = columns
        .map(|column| format!("{column} {{}} {{}}"))
        .collect::<Vec<_>>()
        .join(", ");

    let format_str =
        format!("create table if not exists {table}(entity {{}} primary key, {format_str}\n);");

    let arguments = quote! {
        <Entity as ::sqlx::Type<#database>>::type_info().name(),
        #(
            #type_info.name(),
            if #type_info.is_null() { "null" } else {"not null"},
        )*
    };

    quote! {
        fn create_component_table<'pool, Entity>(
            pool: &'pool ::sqlx::Pool<#database>,
        ) -> impl ::core::future::Future<Output = Result<<#database as ::sqlx::Database>::QueryResult, ::sqlx::Error>> + Send
        where
            Entity: sqlx::Type<#database>,
        {
            use ::sqlx::TypeInfo as _;
            use ::sqlx::Executor as _;

            async move {
                let sql = format!(
                    #format_str,
                    #arguments
                );

                pool.execute(sql.as_str()).await
            }
        }
    }
}

pub fn select_query(database: &TokenStream, fields: &Fields) -> TokenStream {
    let mut fields = fields.iter();

    let first_item = &fields.next().unwrap().ty;
    let first = quote! {
        let join = <#first_item as ::erm::archetype::Archetype<#database>>::list_statement();
    };

    let list_statements = fields.map(|field| {
        let field = &field.ty;

        quote! {
            let join = ::erm::cte::InnerJoin {
                left: (
                    Box::new(join),
                    "entity".to_string(),
                ),
                right: (
                    Box::new(<#field as ::erm::archetype::Archetype<#database>>::list_statement()),
                    "entity".to_string(),
                ),
            }
        }
    });

    quote! {
        fn list_statement() -> impl ::erm::cte::CommonTableExpression {
            #first;
            #(#list_statements;)*

            join
        }
    }
}
