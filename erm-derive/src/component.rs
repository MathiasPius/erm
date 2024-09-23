use proc_macro2::{Ident, Literal, Punct, TokenStream};
use quote::quote;
use syn::{parse::Parse, spanned::Spanned, Data, DeriveInput, Error, Token};

use crate::field::Field;

pub struct Component {
    pub typename: Ident,
    pub table_name: String,
    pub fields: Vec<Field>,
}

impl Component {
    pub fn implementation(
        &self,
        sqlx: &TokenStream,
        database: &TokenStream,
        placeholder_char: char,
    ) -> TokenStream {
        let component_name = &self.typename;

        let statements = self.statements(placeholder_char);
        let table = self.table();
        let columns = self.columns(sqlx, database);
        let table_creator = self.table_creator(sqlx, database);
        let serialize = self.field_serializer(sqlx, database);
        let deserialize = self.field_deserializer(sqlx, database);

        quote! {
            impl ::erm::component::Component<#database> for #component_name {
                #statements
                #table
                #columns
                #table_creator
                #serialize
                #deserialize
            }
        }
    }

    fn statements(&self, placeholder_char: char) -> TokenStream {
        let table = &self.table_name.trim_matches('"');

        let column_names: Vec<_> = self.fields.iter().map(Field::column_name).collect();
        let placeholders = placeholders(placeholder_char, column_names.len() + 1);

        let insert = format!(
            "insert into {table}(entity, {column_names}) values({placeholders});",
            placeholders = placeholders.join(", "),
            column_names = column_names.join(", ")
        );

        let update = {
            let field_updates = column_names
                .iter()
                .zip(placeholders.iter().skip(1))
                .map(|(column, placeholder)| format!("{column} = {placeholder}"))
                .collect::<Vec<_>>();

            format!(
                "update {table} set {field_updates} where entity = {placeholder_char}1",
                field_updates = field_updates.join(", ")
            )
        };

        let delete = format!("delete from {table} where entity = {placeholder_char}1");

        quote! {
            const INSERT: &'static str = #insert;
            const UPDATE: &'static str = #update;
            const DELETE: &'static str = #delete;
        }
    }

    fn table_creator(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let table = &self.table_name.trim_matches('"');

        let columns = self
            .fields
            .iter()
            .map(Field::column_name)
            .map(|column| format!("{column} {{}} {{}}"))
            .collect::<Vec<_>>()
            .join(", ");

        let format_str =
            format!("create table if not exists {table}(entity {{}} primary key, {columns}\n);");

        let definitions = self
            .fields
            .iter()
            .map(|field| field.sql_definition(sqlx, database));

        quote! {
            fn create_component_table<'pool, Entity>(
                pool: &'pool #sqlx::Pool<#database>,
            ) -> impl ::core::future::Future<Output = Result<<#database as #sqlx::Database>::QueryResult, #sqlx::Error>> + Send
            where
                Entity: #sqlx::Type<#database>,
            {
                async move {
                    use sqlx::TypeInfo as _;
                    use sqlx::Executor as _;

                    let sql = format!(
                        #format_str,
                        <Entity as #sqlx::Type<#database>>::type_info().name(),
                        #(#definitions,)*
                    );

                    pool.execute(sql.as_str()).await
                }
            }
        }
    }

    fn table(&self) -> TokenStream {
        let table_name = &self.table_name.trim_matches('"');
        quote! {
            fn table() -> &'static str {
                #table_name
            }
        }
    }

    fn columns(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let columns = self
            .fields
            .iter()
            .map(|field| field.column_definition(sqlx, database));

        quote! {
            fn columns() -> Vec<::erm::component::ColumnDefinition::<#database>> {
                vec![#(#columns,)*]
            }
        }
    }

    fn field_serializer(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let binds = self.fields.iter().map(Field::serialize);

        quote! {
            fn serialize_fields<'q>(
                &'q self,
                query: #sqlx::query::Query<'q, #database, <#database as #sqlx::Database>::Arguments<'q>>,
            ) -> #sqlx::query::Query<'q, #database, <#database as #sqlx::Database>::Arguments<'q>> {
                #(#binds)*

                query
            }
        }
    }

    fn field_deserializer(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let component_name = &self.typename;
        let deserialized_fields = self.fields.iter().map(Field::deserialize);

        let assignments = self.fields.iter().map(|field| {
            let ident = &field.ident;

            quote! {
                #ident: #ident?
            }
        });

        quote! {
            fn deserialize_fields(row: &mut ::erm::row::OffsetRow<<#database as #sqlx::Database>::Row>) -> Result<Self, #sqlx::Error> {
                #(#deserialized_fields;)*

                let component = #component_name {
                    #(#assignments,)*
                };

                Ok(component)
            }
        }
    }
}

impl Parse for Component {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let derive = DeriveInput::parse(input)?;

        let Data::Struct(data) = derive.data else {
            return Err(Error::new(
                derive.span(),
                "Component can only be derived for struct types",
            ));
        };

        let attributes: Vec<_> = Result::<Vec<Vec<_>>, syn::Error>::from_iter(
            derive
                .attrs
                .iter()
                .filter(|attr| attr.meta.path().is_ident("erm"))
                .map(|attr| {
                    let list = attr.meta.require_list()?;

                    Ok(syn::parse2::<ComponentAttributeList>(list.tokens.clone())?.0)
                }),
        )?
        .into_iter()
        .flatten()
        .collect();

        let table_name = attributes
            .iter()
            .find_map(ComponentAttribute::table)
            .unwrap_or(derive.ident.to_string());

        let type_name = derive.ident.clone();

        let fields = Result::<Vec<Field>, _>::from_iter(
            data.fields.into_iter().enumerate().map(Field::try_from),
        )?;

        Ok(Component {
            typename: type_name,
            table_name,
            fields,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ComponentAttributeList(pub Vec<ComponentAttribute>);

impl Parse for ComponentAttributeList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attributes = Vec::new();

        while !input.is_empty() {
            attributes.push(ComponentAttribute::parse(input)?);

            if input.peek(Token![,]) {
                input.parse::<Punct>()?;
            }
        }

        Ok(Self(attributes))
    }
}

#[derive(Debug, Clone)]
pub enum ComponentAttribute {
    /// Changes the name of the Component's sql table.
    Table { name: Literal },
}

impl ComponentAttribute {
    pub fn table(&self) -> Option<String> {
        #[allow(irrefutable_let_patterns)]
        if let ComponentAttribute::Table { name } = self {
            Some(name.to_string())
        } else {
            None
        }
    }
}

impl Parse for ComponentAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;

        Ok(match ident.to_string().as_str() {
            "table" => {
                input.parse::<Token![=]>()?;

                ComponentAttribute::Table {
                    name: input.parse()?,
                }
            }
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "unexpected Component attribute",
                ))
            }
        })
    }
}

/// Generates placeholder values corresponding to the number of columns.
pub fn placeholders(character: char, count: usize) -> Vec<String> {
    std::iter::repeat(character)
        .enumerate()
        .skip(1)
        .take(count)
        .map(|(i, character)| format!("{character}{i}"))
        .collect::<Vec<_>>()
}
