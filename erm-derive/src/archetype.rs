use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{parse::Parse, Data, DeriveInput, Error};

use crate::field::Field;

pub struct Archetype {
    pub typename: Ident,
    pub fields: Vec<Field>,
}

impl Archetype {
    pub fn implementation(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let archetype_name = &self.typename;

        let remove = self.remove(sqlx, database);

        let deserializer = self.component_deserializer(sqlx, database);

        quote! {
            impl ::erm::archetype::Archetype<#database> for #archetype_name
            {
            }

            impl ::erm::serialization::Deserializeable<#database> for #archetype_name {
                #deserializer
            }

            impl ::erm::tables::Removable<#database> for #archetype_name {
                #remove
            }
        }
    }

    fn remove(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let sub_archetypes = self.fields.iter().map(|field| {
            let typename = field.typename();

            quote! {
                <#typename as ::erm::tables::Removable<#database>>::remove(query);
            }
        });

        quote! {
            fn remove<'query, EntityId>(query: &mut ::erm::entity::EntityPrefixedQuery<'query, #database, EntityId>)
            where
                EntityId: #sqlx::Encode<'query, #database> + #sqlx::Type<#database> + Clone + 'query
            {
                #(#sub_archetypes)*
            }
        }
    }

    fn component_deserializer(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let archetype_name = &self.typename;

        let sub_expressions = self.fields.iter().map(|field| {
            let typename = field.typename();

            quote! {
                <#typename as ::erm::serialization::Deserializeable<#database>>::cte()
            }
        });

        let components = self.fields.iter().map(|field| {
            let name = field.ident();
            let typename = field.typename();

            quote! {
                let #name = <#typename as ::erm::serialization::Deserializeable<#database>>::deserialize(row);
            }
        });

        let assignments = self.fields.iter().map(|field| {
            let ident = field.ident();

            quote! {
                #ident: #ident?
            }
        });

        quote! {
            fn cte() -> Box<dyn ::erm::cte::CommonTableExpression> {
                Box::new(::erm::cte::Merge {
                    tables: vec![
                        #(#sub_expressions,)*
                    ]
                })
            }

            fn deserialize(row: &mut ::erm::row::OffsetRow<<#database as #sqlx::Database>::Row>) -> Result<Self, #sqlx::Error> {
                #(#components)*

                let archetype = #archetype_name {
                    #(#assignments,)*
                };

                Ok(archetype)
            }
        }
    }
}

impl Parse for Archetype {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let derive = DeriveInput::parse(input)?;

        let Data::Struct(data) = derive.data else {
            return Err(Error::new(
                derive.ident.span(),
                "Archetype can only be derived for struct types",
            ));
        };

        let type_name = derive.ident.clone();

        let fields = Result::<Vec<Field>, _>::from_iter(
            data.fields.into_iter().enumerate().map(Field::try_from),
        )?;

        Ok(Archetype {
            typename: type_name,
            fields,
        })
    }
}
