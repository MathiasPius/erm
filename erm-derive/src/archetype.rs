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

        // let insert = self.insert(sqlx, database);
        // let update = self.update(database);
        let remove = self.remove(sqlx, database);

        let select = self.select(database);

        // let serializer = self.component_serializer(sqlx, database);
        let deserializer = self.component_deserializer(sqlx, database);

        quote! {
            impl ::erm::archetype::Archetype<#database> for #archetype_name
            {
                #select
            }

            // impl ::erm::serialization::Serializable<#database> for #archetype_name {
            //     #serializer
            //     #insert
            //     #update
            // }

            impl ::erm::serialization::Deserializeable<#database> for #archetype_name {
                #deserializer
            }

            impl ::erm::tables::Removeable<#database> for #archetype_name {
                #remove
            }
        }
    }

    /*
    fn insert(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let sub_archetypes = self.fields.iter().map(|field| {
            let name = field.ident();
            let typename = field.typename();

            quote! {
                <#typename as ::erm::serialization::Serializable<#database>>::insert(&self.#name, query);
            }
        });

        quote! {
            fn insert<'query, Entity>(&'query self, query: &mut ::erm::entity::EntityPrefixedQuery<'query, #database, Entity>)
            where
                Entity: #sqlx::Encode<'query, #database> + #sqlx::Type<#database> + Clone + 'query
            {
                #(#sub_archetypes)*
            }
        }
    }

    fn update(&self, database: &TokenStream) -> TokenStream {
        let sub_archetypes = self.fields.iter().map(|field| {
            let name = field.ident();
            let typename = field.typename();

            quote! {
                <#typename as ::erm::serialization::Serializable<#database>>::update(&self.#name, query);
            }
        });

        quote! {
            fn update<'query, Entity>(&'query self, query: &mut ::erm::entity::EntityPrefixedQuery<'query, #database, Entity>)
            where
                Entity: sqlx::Encode<'query, #database> + sqlx::Type<#database> + Clone + 'query
            {
                #(#sub_archetypes)*
            }
        }
    }
     */

    fn remove(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let sub_archetypes = self.fields.iter().map(|field| {
            let typename = field.typename();

            quote! {
                <#typename as ::erm::tables::Removeable<#database>>::remove(query);
            }
        });

        quote! {
            fn remove<'query, Entity>(query: &mut ::erm::entity::EntityPrefixedQuery<'query, #database, Entity>)
            where
                Entity: #sqlx::Encode<'query, #database> + #sqlx::Type<#database> + Clone + 'query
            {
                #(#sub_archetypes)*
            }
        }
    }

    fn select(&self, database: &TokenStream) -> TokenStream {
        let mut fields = self.fields.iter();
        let first_item = fields.next().unwrap().typename();

        let first = quote! {
            let join = <#first_item as ::erm::archetype::Archetype<#database>>::list_statement();
        };

        let list_statements = fields.map(|field| {
            let field = field.typename();

            quote! {
                let join = ::erm::cte::Join {
                    left: Box::new(join),
                    right: Box::new(<#field as ::erm::archetype::Archetype<#database>>::list_statement()),
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

    /*
    fn component_serializer(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let binds = self.fields.iter().map(|field| {
            let field_name = field.ident();
            quote! {
                let query = self.#field_name.serialize(query);
            }
        });

        quote! {
            fn serialize<'q>(
                &'q self,
                query: #sqlx::query::Query<'q, #database, <#database as #sqlx::Database>::Arguments<'q>>,
            ) -> #sqlx::query::Query<'q, #database, <#database as #sqlx::Database>::Arguments<'q>> {
                #(#binds)*

                query
            }
        }
    }
     */

    fn component_deserializer(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let archetype_name = &self.typename;
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
