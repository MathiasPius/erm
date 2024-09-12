use proc_macro2::{Ident, TokenStream};
use queries::{insert_archetype, select_query};
use quote::{quote, TokenStreamExt};
use serde::{deserialize_components, deserialize_fields, serialize_components, serialize_fields};
use syn::{Data, DeriveInput};

mod queries;
mod serde;

#[proc_macro_derive(Component)]
pub fn derive_component(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let derive: DeriveInput = syn::parse2(stream).unwrap();

    let Data::Struct(data) = derive.data else {
        panic!("only structs can be stored as components");
    };

    let component_name = derive.ident;
    let table = component_name.to_string().to_lowercase();

    let implementation = |database: Ident| {
        let database = quote! {::sqlx::#database};

        let columns = data.fields.iter().map(|field| {
            let name = field.ident.as_ref().unwrap().to_string();
            let typename = &field.ty;

            quote! {
                ::erm::ColumnDefinition::<#database> {
                    name: #name,
                    type_info: <#typename as ::sqlx::Type<#database>>::type_info(),
                }
            }
        });

        let deserialize_fields = deserialize_fields(&database, &component_name, &data.fields);
        let serialize_fields = serialize_fields(&database, &data.fields);

        let insert = queries::insert_component(&table, '?', &data);
        let update = queries::update_component(&table, '?', &data);
        let create_component_table = queries::create_component_table(&database, &table, &data);

        quote! {
            impl ::erm::Component<#database> for #component_name {
                const INSERT: &'static str = #insert;
                const UPDATE: &'static str = #update;

                fn table() -> &'static str {
                    #table
                }

                fn columns() -> Vec<::erm::ColumnDefinition::<#database>> {
                    vec![#(#columns,)*]
                }

                #create_component_table
                #deserialize_fields
                #serialize_fields
            }
        }
    };

    let mut implementations = TokenStream::new();
    #[cfg(feature = "sqlite")]
    implementations.append_all(implementation(Ident::new("Sqlite", data.struct_token.span)));

    #[cfg(feature = "postgres")]
    implementations.append_all(implementation(Ident::new(
        "Postgres",
        data.struct_token.span,
    )));

    #[cfg(feature = "mysql")]
    implementations.append_all(implementation(Ident::new("MySql", data.struct_token.span)));

    implementations.into()
}

#[proc_macro_derive(Archetype)]
pub fn derive_archetype(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let derive: DeriveInput = syn::parse2(stream).unwrap();

    let Data::Struct(data) = derive.data else {
        panic!("only structs can act as archetypes");
    };

    let archetype_name = derive.ident;

    let implementation = |database: Ident| {
        let database = quote! {::sqlx::#database};

        let select_query = select_query(&database, &data.fields);

        let serialize_components = serialize_components(&database, &data.fields);

        let deserialize_components =
            deserialize_components(&archetype_name, &database, &data.fields);

        let insert_archetype = insert_archetype(&database, &data.fields);

        quote! {
            impl ::erm::Archetype<#database> for #archetype_name
            {
                #insert_archetype
                #select_query

                #deserialize_components
                #serialize_components
            }
        }
    };

    let mut implementations = TokenStream::new();
    #[cfg(feature = "sqlite")]
    implementations.append_all(implementation(Ident::new("Sqlite", data.struct_token.span)));

    #[cfg(feature = "postgres")]
    implementations.append_all(implementation(Ident::new(
        "Postgres",
        data.struct_token.span,
    )));

    #[cfg(feature = "mysql")]
    implementations.append_all(implementation(Ident::new("MySql", data.struct_token.span)));

    implementations.into()
}
