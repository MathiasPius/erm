use std::future::Future;

use async_stream::stream;
use futures::Stream;
use sqlx::{ColumnIndex, Database, Executor, IntoArguments, Pool};

use crate::{
    component::Component,
    condition::Condition,
    cte::{CommonTableExpression, Filter, Select},
    entity::EntityPrefixedQuery,
    row::Rowed,
    serialization::{Deserializeable, Serializable},
    tables::Removeable,
};

pub trait DatabasePlaceholder {
    const PLACEHOLDER: char = '?';
}

#[cfg(feature = "sqlite")]
impl DatabasePlaceholder for sqlx::Sqlite {}

#[cfg(feature = "mysql")]
impl DatabasePlaceholder for sqlx::MySql {}

#[cfg(feature = "postgres")]
impl DatabasePlaceholder for sqlx::Postgres {
    const PLACEHOLDER: char = '$';
}

pub trait Archetype<DB: Database>: Deserializeable<DB> + Sized {
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
        Cond: for<'c> Condition<'c, DB>,
        for<'c> <DB as sqlx::Database>::Arguments<'c>: IntoArguments<'c, DB> + Send,
        for<'e> Entity: sqlx::Decode<'e, DB> + sqlx::Encode<'e, DB> + sqlx::Type<DB> + Unpin + Send,
        for<'c> &'c mut <DB as sqlx::Database>::Connection: Executor<'c, Database = DB>,
        DB: DatabasePlaceholder,
        usize: ColumnIndex<<DB as sqlx::Database>::Row>,
    {
        stream! {
            let sql = format!(
                "{} where {}",
                <Self as Archetype<DB>>::list_statement().serialize(<DB as DatabasePlaceholder>::PLACEHOLDER),
                condition.serialize()
            );

            println!("{sql}");

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
        DB: DatabasePlaceholder,
        usize: ColumnIndex<<DB as sqlx::Database>::Row>,
    {
        async move {
            let sql = <Self as Archetype<DB>>::get_statement()
                .serialize(<DB as DatabasePlaceholder>::PLACEHOLDER);
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
        Self: Serializable<DB> + Send,
        for<'connection> <DB as sqlx::Database>::Arguments<'connection>:
            IntoArguments<'connection, DB> + Send,
        for<'connection> &'connection mut <DB as sqlx::Database>::Connection:
            Executor<'connection, Database = DB>,
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + Send + 'query,
    {
        let mut inserts = EntityPrefixedQuery::<'_, DB, Entity>::new(entity);

        <Self as Serializable<DB>>::insert(&self, &mut inserts);

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
        Self: Serializable<DB> + Send,
        for<'connection> <DB as sqlx::Database>::Arguments<'connection>:
            IntoArguments<'connection, DB> + Send,
        for<'connection> &'connection mut <DB as sqlx::Database>::Connection:
            Executor<'connection, Database = DB>,
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + Send + 'query,
    {
        let mut inserts = EntityPrefixedQuery::<'_, DB, Entity>::new(entity);

        <Self as Serializable<DB>>::update(&self, &mut inserts);

        async move {
            let mut tx = pool.begin().await.unwrap();
            for query in inserts.queries {
                query.execute(&mut *tx).await.unwrap();
            }

            tx.commit().await.unwrap();
        }
    }

    fn remove<'query, Entity>(
        pool: &'query Pool<DB>,
        entity: Entity,
    ) -> impl Future<Output = ()> + Send + 'query
    where
        Self: Removeable<DB> + Send,
        for<'connection> <DB as sqlx::Database>::Arguments<'connection>:
            IntoArguments<'connection, DB> + Send,
        for<'connection> &'connection mut <DB as sqlx::Database>::Connection:
            Executor<'connection, Database = DB>,
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + Send + 'query,
    {
        let mut removes = EntityPrefixedQuery::<'_, DB, Entity>::new(entity);

        <Self as Removeable<DB>>::remove(&mut removes);

        async move {
            let mut tx = pool.begin().await.unwrap();
            for query in removes.queries {
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
    fn list_statement() -> impl CommonTableExpression {
        Select {
            optional: false,
            table: <T as Component<DB>>::table().to_string(),
            columns: <T as Component<DB>>::columns()
                .into_iter()
                .map(|column| column.name().to_string())
                .collect(),
        }
    }
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
macro_rules! expand_inner_join {
    ($db:ty, $first:ident, $second:ident) => {
        crate::cte::Join {
            direction: "inner",
            left:
                Box::new(<$first as Archetype<$db>>::list_statement()),
            right:
                Box::new(<$second as Archetype<$db>>::list_statement()),
        }
    };

    ($db:ty, $first:ident, $($list:ident),*) => {
        crate::cte::Join {
            direction: "inner",
            left: Box::new(<$first as Archetype<$db>>::list_statement()),
            right: Box::new(<($($list),*) as Archetype<$db>>::list_statement()),
        }
    };
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
macro_rules! impl_compound_for_db{
    ($db:ty, $($list:ident:$index:tt),*) => {
        impl<$($list),*> Archetype<$db> for ($($list,)*)
        where
            $($list: Archetype<$db>,)*
        {
            fn list_statement() -> impl CommonTableExpression {
                expand_inner_join!($db, $($list),*)
            }
        }
    };
}

macro_rules! impl_compound {
    ($($list:ident:$index:tt),*) => {
        #[cfg(feature = "sqlite")]
        impl_compound_for_db!(sqlx::Sqlite, $($list:$index),*);
        #[cfg(feature = "postgres")]
        impl_compound_for_db!(sqlx::Postgres, $($list:$index),*);
        #[cfg(feature = "mysql")]
        impl_compound_for_db!(sqlx::MySql, $($list:$index),*);
    };
}

impl_compound!(A:0, B:1);
impl_compound!(A:0, B:1, C:2);
impl_compound!(A:0, B:1, C:2, D:3);
impl_compound!(A:0, B:1, C:2, D:3, E:4);
impl_compound!(A:0, B:1, C:2, D:3, E:4, F:5);
impl_compound!(A:0, B:1, C:2, D:3, E:4, F:5, G:6);
impl_compound!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7);
impl_compound!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8);
