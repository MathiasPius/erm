use proc_macro2::{Ident, TokenStream};
use quote::{quote, TokenStreamExt};
use syn::{Data, DeriveInput};

#[proc_macro_derive(Component)]
pub fn derive_component(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let derive: DeriveInput = syn::parse2(stream).unwrap();

    let Data::Struct(data) = derive.data else {
        panic!("only structs can be stored as components");
    };

    let component_name = derive.ident;
    let table = component_name.to_string().to_lowercase();

    let columns: Vec<_> = data
        .fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap().to_string())
        .collect();

    let unpack: Vec<_> = data
        .fields
        .iter()
        .map(|field| {
            let name = field.ident.as_ref().unwrap();
            let typename = &field.ty;

            quote! {
                let #name = row.try_get::<#typename>();
            }
        })
        .collect();

    let repack: Vec<_> = data
        .fields
        .iter()
        .map(|field| {
            let name = field.ident.as_ref().unwrap();

            quote! {
                #name: #name?
            }
        })
        .collect();

    let binds: Vec<_> = data
        .fields
        .iter()
        .map(|field| {
            let name = field.ident.as_ref().unwrap();

            quote! {
                .bind(self.#name)
            }
        })
        .collect();

    let implementation = |database: Ident| {
        quote! {
            impl ::erm::Component<::sqlx::#database> for #component_name {
                fn table() -> &'static str {
                    #table
                }

                fn columns() -> &'static [&'static str] {
                    &[#(#columns,)*]
                }

                fn deserialize_fields(row: &mut ::erm::OffsetRow<<::sqlx::#database as ::sqlx::Database>::Row>) -> Result<Self, ::sqlx::Error> {
                    #(#unpack;)*

                    Ok(#component_name {
                        #(#repack,)*
                    })
                }

                fn serialize_fields<'q>(
                    &'q self,
                    query: ::sqlx::query::Query<'q, ::sqlx::#database, <::sqlx::#database as ::sqlx::Database>::Arguments<'q>>,
                ) -> ::sqlx::query::Query<'q, ::sqlx::#database, <::sqlx::#database as ::sqlx::Database>::Arguments<'q>> {
                    query #(#binds)*
                }

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

/*
fn into_component_serialization<'a>(fields: impl Iterator<Item = &'a Field>) -> Vec<TokenStream> {
    fields
        .map(|field| {
            let name = field.ident.as_ref().unwrap();

            quote! {
                let query = self.#name.serialize(query, entity);
            }
        })
        .collect::<Vec<_>>()
}

fn into_component_deserialization<'a>(fields: impl Iterator<Item = &'a Field>) -> Vec<TokenStream> {
    fields
        .map(|field| {
            let name = field.ident.as_ref().unwrap();
            let typename = &field.ty;

            quote! {
                #name: {
                    let value = #typename::deserialize(&row.offset_by(accumulator))?;
                    accumulator += <#typename as ::erm::component::Component>::FIELDS.len();
                    value
                }
            }
        })
        .collect::<Vec<_>>()
}

#[proc_macro_derive(Archetype)]
pub fn derive_archetype(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let derive: DeriveInput = syn::parse2(stream).unwrap();

    let Data::Struct(data) = derive.data else {
        panic!("only structs can act as archetypes");
    };

    let archetype_name = derive.ident;

    let components = data.fields.iter().map(|field| {
        let typename = &field.ty;
        quote! {
            <#typename as ::erm::component::Component>::DESCRIPTION
        }
    });

    let deserialization_entries = into_component_deserialization(data.fields.iter());
    let serialization_entries = into_component_serialization(data.fields.iter());

    quote! {
        impl ::erm::archetype::Archetype for #archetype_name {
            const COMPONENTS: &'static [::erm::component::ComponentDesc] = &[#(#components,)*];
        }

        impl<'query, Entity: ::sqlx::Encode<'query, ::sqlx::Sqlite> + ::sqlx::Type<::sqlx::Sqlite> + 'query> ::erm::backend::Serialize<'query, ::sqlx::Sqlite, Entity> for #archetype_name
        {
            fn serialize(
                &'query self,
                query: ::sqlx::query::Query<'query, ::sqlx::Sqlite, <::sqlx::Sqlite as ::sqlx::Database>::Arguments<'query>>,
                entity: &'query Entity,
            ) -> ::sqlx::query::Query<'query, ::sqlx::Sqlite, <::sqlx::Sqlite as ::sqlx::Database>::Arguments<'query>> {
                #(#serialization_entries;)*
                query
            }
        }

        impl<'row> ::erm::backend::Deserialize<'row, ::sqlx::sqlite::SqliteRow> for #archetype_name
        {
            fn deserialize(row: &'row ::erm::OffsetRow<::sqlx::sqlite::SqliteRow>) -> Result<Self, ::sqlx::Error> {
                let mut accumulator = 0usize;
                Ok(#archetype_name {
                    #(#deserialization_entries),*
                })
            }
        }
    }.into()
}
 */
