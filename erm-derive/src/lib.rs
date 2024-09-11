use proc_macro2::{Ident, TokenStream};
use quote::{quote, TokenStreamExt};
use syn::{Data, DeriveInput};

mod queries;

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
        let columns = data.fields.iter().map(|field| {
            let name = field.ident.as_ref().unwrap().to_string();
            let typename = &field.ty;

            quote! {
                ::erm::ColumnDefinition::<::sqlx::#database> {
                    name: #name,
                    type_info: <#typename as ::sqlx::Type<::sqlx::#database>>::type_info(),
                }
            }
        });

        let unpack = data.fields.iter().map(|field| {
            let name = field.ident.as_ref().unwrap();
            let typename = &field.ty;

            quote! {
                let #name = row.try_get::<#typename>();
            }
        });

        let repack = data.fields.iter().map(|field| {
            let name = field.ident.as_ref().unwrap();

            quote! {
                #name: #name?
            }
        });

        let binds = data.fields.iter().map(|field| {
            let name = field.ident.as_ref().unwrap();

            quote! {
                let query = query.bind(&self.#name);
            }
        });

        let insert = queries::insertion_query(&table, '?', &data);
        let create = queries::create_query(&database, &table, &data);

        quote! {
            impl ::erm::Component<::sqlx::#database> for #component_name {
                const INSERT: &'static str = #insert;

                fn table() -> &'static str {
                    #table
                }

                fn columns() -> Vec<::erm::ColumnDefinition::<::sqlx::#database>> {
                    vec![#(#columns,)*]
                }

                fn deserialize_fields(row: &mut ::erm::OffsetRow<<::sqlx::#database as ::sqlx::Database>::Row>) -> Result<Self, ::sqlx::Error> {
                    #(#unpack;)*

                    let component = #component_name {
                        #(#repack,)*
                    };

                    Ok(component)
                }

                fn serialize_fields<'q>(
                    &'q self,
                    query: ::sqlx::query::Query<'q, ::sqlx::#database, <::sqlx::#database as ::sqlx::Database>::Arguments<'q>>,
                ) -> ::sqlx::query::Query<'q, ::sqlx::#database, <::sqlx::#database as ::sqlx::Database>::Arguments<'q>> {
                    #(#binds)*

                    query
                }

                fn create<'e, E, Entity>(
                    executor: &'e E,
                ) -> impl ::core::future::Future<Output = Result<<::sqlx::#database as ::sqlx::Database>::QueryResult, ::sqlx::Error>> + Send
                where
                    Entity: for<'q> sqlx::Encode<'q, ::sqlx::#database> + sqlx::Type<::sqlx::#database> + std::fmt::Debug + Clone,
                    &'e E: ::sqlx::Executor<'e, Database = ::sqlx::#database>
                {
                    use ::sqlx::TypeInfo as _;

                    static SQL: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
                    let sql = SQL.get_or_init(||
                        #create
                    ).as_str();

                    executor.execute(sql)
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

#[proc_macro_derive(Archetype)]
pub fn derive_archetype(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let derive: DeriveInput = syn::parse2(stream).unwrap();

    let Data::Struct(data) = derive.data else {
        panic!("only structs can act as archetypes");
    };

    let archetype_name = derive.ident;

    let implementation = |database: Ident| {
        let field_names = data.fields.iter().map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            quote! {
                let query = self.#field_name.serialize_components(query);
            }
        });

        let mut field_iter = data.fields.iter();

        let first_item = &field_iter.next().unwrap().ty;
        let first = quote! {
            let join = <#first_item as Archetype<::sqlx::#database>>::select_statement();
        };

        let select_statements = field_iter.map(|field| {
            let field = &field.ty;

            quote! {
                let join = ::erm::cte::InnerJoin {
                    left: (
                        Box::new(join),
                        "entity".to_string(),
                    ),
                    right: (
                        Box::new(<#field as Archetype<::sqlx::#database>>::select_statement()),
                        "entity".to_string(),
                    ),
                }
            }
        });

        let unpack: Vec<_> = data
            .fields
            .iter()
            .map(|field| {
                let name = field.ident.as_ref().unwrap();
                let typename = &field.ty;

                quote! {
                    let #name = <#typename as ::erm::Archetype<::sqlx::#database>>::deserialize_components(row);
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

        let insertion_queries = data.fields.iter().map(|field| {
            let name = field.ident.as_ref().unwrap();
            let typename = &field.ty;

            quote! {
                <#typename as ::erm::Archetype<::sqlx::#database>>::insertion_query(&self.#name, query);
            }
        });

        quote! {
            impl ::erm::Archetype<::sqlx::#database> for #archetype_name
            {
                fn insertion_query<'q, Entity>(&'q self, query: &mut ::erm::InsertionQuery<'q, ::sqlx::#database, Entity>)
                where
                    Entity: sqlx::Encode<'q, ::sqlx::#database> + sqlx::Type<::sqlx::#database> + std::fmt::Debug + Clone + 'q
                {
                    #(#insertion_queries)*
                }

                fn serialize_components<'q>(
                    &'q self,
                    query: ::sqlx::query::Query<'q, ::sqlx::#database, <::sqlx::#database as ::sqlx::Database>::Arguments<'q>>,
                ) -> ::sqlx::query::Query<'q, ::sqlx::#database, <::sqlx::#database as ::sqlx::Database>::Arguments<'q>> {
                    #[cfg(feature = "tracing")]
                    ::tracing::trace!("serializing archetype {}", ::std::any::type_name::<Self>());

                    #(#field_names)*

                    query
                }

                fn select_statement() -> impl ::erm::cte::CommonTableExpression {
                    #first;
                    #(#select_statements;)*

                    join
                }

                fn deserialize_components(
                    row: &mut ::erm::OffsetRow<<::sqlx::#database as ::sqlx::Database>::Row>,
                ) -> Result<Self, ::sqlx::Error> {
                    #[cfg(feature = "tracing")]
                    ::tracing::trace!("deserializing archetype {}", ::std::any::type_name::<Self>());

                    #(#unpack;)*

                    Ok(#archetype_name {
                        #(#repack,)*
                    })
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
