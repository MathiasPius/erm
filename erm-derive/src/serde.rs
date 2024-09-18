use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident};

use crate::attributes::ComponentAttributes;

pub fn deserialize_fields(
    database: &TokenStream,
    component_name: &Ident,
    fields: &Fields,
) -> TokenStream {
    let unpack = fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        let typename = &field.ty;

        let attributes = ComponentAttributes::from_attributes(&field.attrs).unwrap();

        if let Some(intermediate) = attributes.deser_as {
            quote! {
                let #name: Result<#typename, _> = row.try_get::<#intermediate>().map(|field| <#typename as From<#intermediate>>::from(field));
            }
        } else {
            quote! {
                let #name = row.try_get::<#typename>();
            }
        }
    });

    let repack = fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();

        quote! {
            #name: #name?
        }
    });

    quote! {
        fn deserialize_fields(row: &mut ::erm::row::OffsetRow<<#database as ::sqlx::Database>::Row>) -> Result<Self, ::sqlx::Error> {
            #(#unpack;)*

            let component = #component_name {
                #(#repack,)*
            };

            Ok(component)
        }
    }
}

pub fn serialize_fields(database: &TokenStream, fields: &Fields) -> TokenStream {
    let binds = fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        let typename = &field.ty;

        let attributes = ComponentAttributes::from_attributes(&field.attrs).unwrap();

        if let Some(intermediate) = attributes.ser_as {
            quote! {
                let query = query.bind(<#typename as AsRef<#intermediate>>::as_ref(&self.#name));
            }
        } else {
            quote! {
                let query = query.bind(&self.#name);
            }
        }
    });

    quote! {
        fn serialize_fields<'q>(
            &'q self,
            query: ::sqlx::query::Query<'q, #database, <#database as ::sqlx::Database>::Arguments<'q>>,
        ) -> ::sqlx::query::Query<'q, #database, <#database as ::sqlx::Database>::Arguments<'q>> {
            #(#binds)*

            query
        }
    }
}

pub fn deserialize_components(
    archetype_name: &Ident,
    database: &TokenStream,
    fields: &Fields,
) -> TokenStream {
    let unpack: Vec<_> = fields
        .iter()
        .map(|field| {
            let name = field.ident.as_ref().unwrap();
            let typename = &field.ty;

            quote! {
                let #name = <#typename as ::erm::archetype::Archetype<#database>>::deserialize_components(row);
            }
        })
        .collect();

    let repack: Vec<_> = fields
        .iter()
        .map(|field| {
            let name = field.ident.as_ref().unwrap();

            quote! {
                #name: #name?
            }
        })
        .collect();

    quote! {
        fn deserialize_components(
            row: &mut ::erm::row::OffsetRow<<#database as ::sqlx::Database>::Row>,
        ) -> Result<Self, ::sqlx::Error> {
            #(#unpack;)*

            Ok(#archetype_name {
                #(#repack,)*
            })
        }
    }
}

pub fn serialize_components(database: &TokenStream, fields: &Fields) -> TokenStream {
    let field_names = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        quote! {
            let query = self.#field_name.serialize_components(query);
        }
    });

    quote! {
        fn serialize_components<'q>(
            &'q self,
            query: ::sqlx::query::Query<'q, #database, <#database as ::sqlx::Database>::Arguments<'q>>,
        ) -> ::sqlx::query::Query<'q, #database, <#database as ::sqlx::Database>::Arguments<'q>> {
            #(#field_names)*

            query
        }
    }
}
