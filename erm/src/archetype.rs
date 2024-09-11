use std::{future::Future, sync::OnceLock};

use futures::{Stream, StreamExt as _};
use sqlx::{query::Query, ColumnIndex, Database, Executor, IntoArguments, Pool};
use tracing::{instrument, span, Instrument, Level};

use crate::{
    cte::{CommonTableExpression, InnerJoin, Select},
    insert::InsertionQuery,
    row::Rowed,
    Component, OffsetRow,
};

pub trait Archetype<DB: Database>: Sized {
    fn insert_archetype<'query, Entity>(
        &'query self,
        query: &mut InsertionQuery<'query, DB, Entity>,
    ) where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query;

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
    ) -> impl Future<Output = Result<Self, sqlx::Error>> + Send
    where
        &'a Entity: sqlx::Encode<'a, DB> + sqlx::Type<DB> + Send + Sync + 'static,
        Entity: for<'r> sqlx::Decode<'r, DB> + sqlx::Type<DB> + Unpin + Send + Sync + 'static,
        for<'q> <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        E: for<'e> Executor<'e, Database = DB> + Send + Sync,
        Self: Unpin + Send + Sync,
        usize: ColumnIndex<<DB as sqlx::Database>::Row>,
    {
        let span = span!(Level::TRACE, "get");

        async move {
            static SQL: OnceLock<String> = OnceLock::new();
            let sql = SQL.get_or_init(|| {
                let cte = <Self as Archetype<DB>>::select_statement();

                let sql = cte.serialize();
                sql
            });

            let result: Rowed<Entity, Self> = sqlx::query_as(&sql)
                .bind(entity)
                .fetch_one(executor)
                .await?;

            Ok(result.inner)
        }
        .instrument(span)
    }

    #[instrument(name = "list")]
    fn list<'e, Entity, E>(
        executor: &'e E,
    ) -> impl Stream<Item = Result<(Entity, Self), sqlx::Error>> + Send
    where
        for<'q> <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        &'e E: Executor<'e, Database = DB>,
        Entity: for<'a> sqlx::Decode<'a, DB> + sqlx::Type<DB> + Unpin + Send + Sync + 'static,
        Self: Unpin + Send + Sync + 'static,
        usize: ColumnIndex<<DB as sqlx::Database>::Row>,
    {
        static SQL: OnceLock<String> = OnceLock::new();
        let sql = SQL.get_or_init(|| {
            let cte = <Self as Archetype<DB>>::select_statement();

            cte.serialize()
        });

        println!("{sql}");

        let query = sqlx::query_as(&sql);

        query
            .fetch(executor)
            .map(|row| row.map(|result: Rowed<Entity, Self>| (result.entity, result.inner)))
    }

    fn insert<'query, Entity>(
        &'query self,
        pool: &'query Pool<DB>,
        entity: Entity,
    ) -> impl Future<Output = ()> + Send + 'query
    where
        Self: Send + Sync,
        for<'connection> <DB as sqlx::Database>::Arguments<'connection>:
            IntoArguments<'connection, DB> + Send,
        for<'connection> &'connection mut <DB as sqlx::Database>::Connection:
            Executor<'connection, Database = DB>,
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + Send + 'query,
    {
        let mut inserts = InsertionQuery::<'_, DB, Entity>::new(entity);

        <Self as Archetype<DB>>::insert_archetype(&self, &mut inserts);

        async move {
            let mut tx = pool.begin().await.unwrap();
            for query in inserts.queries {
                query.execute(&mut *tx).await.unwrap();
            }

            tx.commit().await.unwrap();
        }
    }
}

impl<T, DB: Database> Archetype<DB> for T
where
    T: Component<DB>,
{
    fn insert_archetype<'query, Entity>(
        &'query self,
        query: &mut InsertionQuery<'query, DB, Entity>,
    ) where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query,
    {
        <Self as Component<DB>>::insert_component(&self, query);
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
            fn insert_archetype<'query, Entity>(&'query self, query: &mut InsertionQuery<'query, $db, Entity>)
            where
                Entity: sqlx::Encode<'query, $db> + sqlx::Type<$db> + Clone + 'query,
            {
                $(
                    {
                        #[allow(unused)]
                        const $list: () = ();
                        self.${index()}.insert_archetype(query);
                    }
                )*
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
// impl_compound!(A, B, C);
// impl_compound!(A, B, C, D);
// impl_compound!(A, B, C, D, E);
// impl_compound!(A, B, C, D, E, F);
// impl_compound!(A, B, C, D, E, F, G);
// impl_compound!(A, B, C, D, E, F, G, H);
// impl_compound!(A, B, C, D, E, F, G, H, I);
// impl_compound!(A, B, C, D, E, F, G, H, I, J);
