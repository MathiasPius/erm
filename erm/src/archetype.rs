use std::future::Future;

use async_stream::stream;
use futures::Stream;
use sqlx::{query::Query, ColumnIndex, Database, Executor, IntoArguments, Pool};

use crate::{
    condition::Condition,
    cte::{CommonTableExpression, Filter, InnerJoin, Select},
    entity::EntityPrefixedQuery,
    row::Rowed,
    Component, OffsetRow,
};

pub trait Archetype<DB: Database>: Sized {
    fn insert_archetype<'query, Entity>(
        &'query self,
        query: &mut EntityPrefixedQuery<'query, DB, Entity>,
    ) where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query;

    fn update_archetype<'query, Entity>(
        &'query self,
        query: &mut EntityPrefixedQuery<'query, DB, Entity>,
    ) where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query;

    fn list_statement() -> impl CommonTableExpression;

    fn get_statement() -> impl CommonTableExpression {
        Filter {
            inner: Box::new(Self::list_statement()),
            clause: "entity".to_string(),
        }
    }

    fn list<Entity, Cond>(
        pool: &Pool<DB>,
        condition: Cond,
    ) -> impl Stream<Item = Result<(Entity, Self), sqlx::Error>>
    where
        Self: Unpin + Send,
        Cond: Condition<Entity>,
        for<'c> <DB as sqlx::Database>::Arguments<'c>: IntoArguments<'c, DB> + Send,
        for<'e> Entity: sqlx::Decode<'e, DB> + sqlx::Encode<'e, DB> + sqlx::Type<DB> + Unpin + Send,
        for<'c> &'c mut <DB as sqlx::Database>::Connection: Executor<'c, Database = DB>,
        usize: ColumnIndex<<DB as sqlx::Database>::Row>,
    {
        stream! {
            let sql = format!(
                "{} where {}",
                <Self as Archetype<DB>>::list_statement().serialize(),
                condition.serialize()
            );

            let query = condition.bind(sqlx::query_as::<DB, Rowed<Entity, Self>>(&sql));

            for await row in query.fetch(pool) {
                yield row.map(|rowed| (rowed.entity, rowed.inner))
            }
        }
    }

    fn get<Entity>(
        pool: &Pool<DB>,
        entity: &Entity,
    ) -> impl Future<Output = Result<Self, sqlx::Error>>
    where
        Self: Unpin + Send,
        for<'c> <DB as sqlx::Database>::Arguments<'c>: IntoArguments<'c, DB> + Send,
        for<'e> Entity: sqlx::Decode<'e, DB> + sqlx::Encode<'e, DB> + sqlx::Type<DB> + Unpin + Send,
        for<'c> &'c mut <DB as sqlx::Database>::Connection: Executor<'c, Database = DB>,
        usize: ColumnIndex<<DB as sqlx::Database>::Row>,
    {
        async move {
            let sql = <Self as Archetype<DB>>::get_statement().serialize();
            let result: Rowed<Entity, Self> =
                sqlx::query_as(&sql).bind(entity).fetch_one(pool).await?;

            Ok(result.inner)
        }
    }

    fn insert<'query, Entity>(
        &'query self,
        pool: &'query Pool<DB>,
        entity: Entity,
    ) -> impl Future<Output = ()> + Send + 'query
    where
        Self: Send,
        for<'connection> <DB as sqlx::Database>::Arguments<'connection>:
            IntoArguments<'connection, DB> + Send,
        for<'connection> &'connection mut <DB as sqlx::Database>::Connection:
            Executor<'connection, Database = DB>,
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + Send + 'query,
    {
        let mut inserts = EntityPrefixedQuery::<'_, DB, Entity>::new(entity);

        <Self as Archetype<DB>>::insert_archetype(&self, &mut inserts);

        async move {
            let mut tx = pool.begin().await.unwrap();
            for query in inserts.queries {
                query.execute(&mut *tx).await.unwrap();
            }

            tx.commit().await.unwrap();
        }
    }

    fn update<'query, Entity>(
        &'query self,
        pool: &'query Pool<DB>,
        entity: Entity,
    ) -> impl Future<Output = ()> + Send + 'query
    where
        Self: Send,
        for<'connection> <DB as sqlx::Database>::Arguments<'connection>:
            IntoArguments<'connection, DB> + Send,
        for<'connection> &'connection mut <DB as sqlx::Database>::Connection:
            Executor<'connection, Database = DB>,
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + Send + 'query,
    {
        let mut inserts = EntityPrefixedQuery::<'_, DB, Entity>::new(entity);

        <Self as Archetype<DB>>::update_archetype(&self, &mut inserts);

        async move {
            let mut tx = pool.begin().await.unwrap();
            for query in inserts.queries {
                query.execute(&mut *tx).await.unwrap();
            }

            tx.commit().await.unwrap();
        }
    }

    fn serialize_components<'q>(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;

    fn deserialize_components(
        row: &mut OffsetRow<<DB as Database>::Row>,
    ) -> Result<Self, sqlx::Error>;
}

impl<T, DB: Database> Archetype<DB> for T
where
    T: Component<DB>,
{
    fn insert_archetype<'query, Entity>(
        &'query self,
        query: &mut EntityPrefixedQuery<'query, DB, Entity>,
    ) where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query,
    {
        <Self as Component<DB>>::insert_component(&self, query);
    }

    fn update_archetype<'query, Entity>(
        &'query self,
        query: &mut EntityPrefixedQuery<'query, DB, Entity>,
    ) where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query,
    {
        <Self as Component<DB>>::update_component(&self, query);
    }

    fn list_statement() -> impl CommonTableExpression {
        Select {
            table: <T as Component<DB>>::table().to_string(),
            columns: <T as Component<DB>>::columns()
                .into_iter()
                .map(|column| column.name().to_string())
                .collect(),
        }
    }

    fn serialize_components<'q>(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>> {
        <Self as Component<DB>>::serialize_fields(self, query)
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
            fn insert_archetype<'query, Entity>(&'query self, query: &mut EntityPrefixedQuery<'query, $db, Entity>)
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

            fn update_archetype<'query, Entity>(&'query self, query: &mut EntityPrefixedQuery<'query, $db, Entity>)
            where
                Entity: sqlx::Encode<'query, $db> + sqlx::Type<$db> + Clone + 'query,
            {
                $(
                    {
                        #[allow(unused)]
                        const $list: () = ();
                        self.${index()}.update_archetype(query);
                    }
                )*
            }

            fn list_statement() -> impl CommonTableExpression {
                InnerJoin {
                    left: (
                        Box::new(<A as Archetype<$db>>::list_statement()),
                        "entity".to_string(),
                    ),
                    right: (
                        Box::new(<B as Archetype<$db>>::list_statement()),
                        "entity".to_string(),
                    ),
                }
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
