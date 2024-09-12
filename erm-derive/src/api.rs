use proc_macro2::TokenStream;
use quote::quote;

pub fn list_impl(database: &TokenStream) -> TokenStream {
    quote! {
        fn list<'pool, Entity>(
            executor: &'pool ::sqlx::Pool<#database>,
        ) -> impl ::futures::Stream<Item = Result<(Entity, Self), ::sqlx::Error>> + Send
        where
            Self: Unpin + Send + Sync + 'static,
            for<'connection> <#database as ::sqlx::Database>::Arguments<'connection>:
                ::sqlx::IntoArguments<'connection, #database> + Send,
            for<'connection> &'connection mut <#database as ::sqlx::Database>::Connection:
                ::sqlx::Executor<'connection, Database = #database>,
            Entity: for<'a> ::sqlx::Decode<'a, #database> + ::sqlx::Type<#database> + Unpin + Send + Sync + 'static,
            usize: ::sqlx::ColumnIndex<<#database as ::sqlx::Database>::Row>,
        {
            use ::erm::cte::CommonTableExpression as _;
            static SQL: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();

            let query = ::sqlx::query_as(
                &SQL.get_or_init(|| <Self as ::erm::Archetype<#database>>::list_statement().serialize()),
            );

            query
                .fetch(executor)
                .map(|row| row.map(|result: ::erm::row::Rowed<Entity, Self>| (result.entity, result.inner)))
        }
    }
}
