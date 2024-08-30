use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Field};

#[proc_macro_derive(Component)]
pub fn derive_component(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let derive: DeriveInput = syn::parse2(stream).unwrap();

    let Data::Struct(data) = derive.data else {
        panic!("only structs can be stored as components");
    };

    let component_name = derive.ident;
    let table_name = component_name.to_string().to_lowercase();

    let field_descriptors = data.fields.iter().map(into_field_descriptor);
    let deserialization_entries = into_deserialization_entries(data.fields.iter());
    let serialization_entries = into_serialization_entries(data.fields.iter());

    quote! {
        impl ::erm::component::Component for #component_name {
            const TABLE_NAME: &'static str = #table_name;

            const FIELDS: &'static [::erm::component::Field] = &[
                #(#field_descriptors)*
            ];
        }

        impl ::erm::archetype::Archetype for #component_name {
            const COMPONENTS: &'static [::erm::component::ComponentDesc] = &[<Self as ::erm::component::Component>::DESCRIPTION];
        }
   
        impl<'query> ::erm::backend::Serialize<'query, ::sqlx::Sqlite> for #component_name
        {
            fn serialize(
                &self,
                query: ::sqlx::query::Query<'query, ::sqlx::Sqlite, <::sqlx::Sqlite as ::sqlx::Database>::Arguments<'query>>,
            ) -> ::sqlx::query::Query<'query, ::sqlx::Sqlite, <::sqlx::Sqlite as ::sqlx::Database>::Arguments<'query>> {
                query #(#serialization_entries)*
            }
        }

        impl<'row> ::erm::backend::Deserialize<'row, ::sqlx::sqlite::SqliteRow> for #component_name
        {
            fn deserialize(row: &'row ::erm::OffsetRow<::sqlx::sqlite::SqliteRow>) -> Result<Self, ::sqlx::Error> {
                Ok(#component_name {
                    #(#deserialization_entries),*
                })
            }
        }
    }.into()
}

fn into_field_descriptor(field: &Field) -> TokenStream {
    let name = field.ident.as_ref().unwrap().to_string();
    let typename = &field.ty;

    quote! {
        ::erm::component::Field {
            name: #name,
            optional: false,
            type_info: <#typename as ::erm::types::ColumnType>::SQL_TYPE,
        },
    }
}

fn into_deserialization_entries<'a>(fields: impl Iterator<Item = &'a Field>) -> Vec<TokenStream> {
    fields
        .enumerate()
        .map(|(index, field)| {
            let name = field.ident.as_ref().unwrap();
            let typename = &field.ty;

            quote! {
                #name: row.try_get::<#typename>(#index)?
            }
        })
        .collect::<Vec<_>>()
}

fn into_serialization_entries<'a>(fields: impl Iterator<Item = &'a Field>) -> Vec<TokenStream> {
    fields
        .map(|field| {
            let name = field.ident.as_ref().unwrap();
            let typename = &field.ty;

            quote! {
                .bind(self.#name as #typename)
            }
        })
        .collect::<Vec<_>>()
}
