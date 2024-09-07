use std::sync::OnceLock;

use futures::{Stream, StreamExt as _};
use sqlx::{query::Query, Database, Executor, IntoArguments};

use crate::{
    cte::{CommonTableExpression, InnerJoin, Select},
    row::Rowed,
    Component, OffsetRow,
};

pub trait Archetype<DB: Database>: Sized {
    fn insert_statement() -> String;

    fn serialize_components<'q>(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;

    fn select_statement() -> impl CommonTableExpression;

    fn deserialize_components(
        row: &mut OffsetRow<<DB as Database>::Row>,
    ) -> Result<Self, sqlx::Error>;

    fn get<'a, E, Entity>(
        executor: E,
        entity: &'a Entity,
    ) -> impl std::future::Future<Output = Result<Self, sqlx::Error>> + Send
    where
        &'a Entity: sqlx::Encode<'a, DB> + sqlx::Type<DB> + Send + Sync + 'static,
        for<'q> <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        E: for<'e> Executor<'e, Database = DB> + Send + Sync,
        Self: Unpin + Send + Sync,
    {
        async move {
            static SQL: OnceLock<String> = OnceLock::new();
            let sql = SQL.get_or_init(|| {
                let cte = <Self as Archetype<DB>>::select_statement();

                format!(
                    "{}\nselect {} from {} where entity = ?",
                    cte.finalize(),
                    cte.columns()
                        .iter()
                        .map(|(_, column)| format!("{}.{column}", cte.name()))
                        .collect::<Vec<_>>()
                        .join(", "),
                    cte.name()
                )
            });

            println!("{sql}");

            let result: Rowed<Self> = sqlx::query_as(&sql)
                .bind(entity)
                .fetch_one(executor)
                .await?;

            Ok(result.0)
        }
    }

    fn list<'e, E>(executor: &'e E) -> impl Stream<Item = Result<Self, sqlx::Error>> + Send
    where
        for<'q> <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        &'e E: Executor<'e, Database = DB>,
        Self: Unpin + Send + Sync + 'static,
    {
        static SQL: OnceLock<String> = OnceLock::new();
        let sql = SQL.get_or_init(|| {
            let cte = <Self as Archetype<DB>>::select_statement();

            format!(
                "{}\nselect {} from {}",
                cte.finalize(),
                cte.columns()
                    .iter()
                    .map(|(_, column)| format!("{}.{column}", cte.name()))
                    .collect::<Vec<_>>()
                    .join(", "),
                cte.name()
            )
        });

        println!("{sql}");

        let query = sqlx::query_as(&sql);

        Box::pin(
            query
                .fetch(executor)
                .map(|row| row.map(|result: Rowed<Self>| result.0)),
        )
    }

    fn insert<'q, 'e, E, Entity>(
        &'q self,
        executor: &'e E,
        entity: &'q Entity,
    ) -> impl std::future::Future<Output = Result<<DB as Database>::QueryResult, sqlx::Error>>
    where
        <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        &'e E: Executor<'e, Database = DB>,
        &'q Entity: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
        'q: 'e,
    {
        static SQL: OnceLock<String> = OnceLock::new();

        let sql = SQL.get_or_init(|| <Self as Archetype<DB>>::insert_statement());

        let query = sqlx::query(sql).bind(entity);
        let query = self.serialize_components(query);

        executor.execute(query)
    }
}

impl<T, DB: Database> Archetype<DB> for T
where
    T: Component<DB>,
{
    fn insert_statement() -> String {
        let table = <T as Component<DB>>::table();
        let columns: Vec<_> = <T as Component<DB>>::columns()
            .into_iter()
            .map(|def| def.name())
            .collect();

        format!(
            "insert into {}(entity, {}) values(?1, {})",
            table,
            columns.join(", "),
            std::iter::repeat("?")
                .take(columns.len())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn serialize_components<'q>(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>> {
        <Self as Component<DB>>::serialize_fields(self, query)
    }

    fn select_statement() -> impl CommonTableExpression {
        Select {
            table: <T as Component<DB>>::table().to_string(),
            columns: <T as Component<DB>>::columns()
                .into_iter()
                .map(|column| column.name().to_string())
                .collect(),
        }
    }

    fn deserialize_components(
        row: &mut OffsetRow<<DB as Database>::Row>,
    ) -> Result<Self, sqlx::Error> {
        <Self as Component<DB>>::deserialize_fields(row)
    }
}

macro_rules! impl_compound_for_db{
    ($db:ty, $($list:ident),*) => {
        impl<$($list),*> Archetype<$db> for ($($list,)*)
        where
            $($list: Archetype<$db>,)*
        {
            fn insert_statement() -> String {
                vec![
                    $(<$list as Archetype<$db>>::insert_statement()),*
                ]
                .join(";\n")
            }

            fn serialize_components<'q>(
                &'q self,
                query: Query<'q, $db, <$db as Database>::Arguments<'q>>,
            ) -> Query<'q, $db, <$db as Database>::Arguments<'q>> {
                $(
                    #[allow(unused)]
                    const $list: () = ();
                    let query = self.${index()}.serialize_components(query);
                )*

                query
            }

            fn select_statement() -> impl CommonTableExpression {
                InnerJoin {
                    left: (
                        Box::new(<A as Archetype<$db>>::select_statement()),
                        "entity".to_string(),
                    ),
                    right: (
                        Box::new(<B as Archetype<$db>>::select_statement()),
                        "entity".to_string(),
                    ),
                }
            }

            fn deserialize_components(
                row: &mut OffsetRow<<$db as Database>::Row>,
            ) -> Result<Self, sqlx::Error> {
                Ok((
                    $(
                        <$list as Archetype<$db>>::deserialize_components(row)?,
                    )*
                ))
            }
        }
    };
}

macro_rules! impl_compound {
    ($($list:ident),*) => {
        #[cfg(feature = "sqlite")]
        impl_compound_for_db!(sqlx::Sqlite, $($list),*);
        #[cfg(feature = "postgres")]
        impl_compound_for_db!(sqlx::Postgres, $($list),*);
        #[cfg(feature = "mysql")]
        impl_compound_for_db!(sqlx::MySql, $($list),*);
    };
}

impl_compound!(A, B);
impl_compound!(A, B, C);
impl_compound!(A, B, C, D);
impl_compound!(A, B, C, D, E);
impl_compound!(A, B, C, D, E, F);
impl_compound!(A, B, C, D, E, F, G);
impl_compound!(A, B, C, D, E, F, G, H);
impl_compound!(A, B, C, D, E, F, G, H, I);
impl_compound!(A, B, C, D, E, F, G, H, I, J);
