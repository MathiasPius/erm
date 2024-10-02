use std::future::Future;

use sqlx::{ColumnIndex, Database, Executor, IntoArguments, Pool};

use crate::{
    entity::EntityPrefixedQuery,
    serialization::{Deserializeable, Serializable},
    tables::Removable,
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
    fn insert<'query, EntityId>(
        &'query self,
        pool: &'query Pool<DB>,
        entity: EntityId,
    ) -> impl Future<Output = ()> + Send + 'query
    where
        Self: Serializable<DB> + Send,
        for<'connection> <DB as sqlx::Database>::Arguments<'connection>:
            IntoArguments<'connection, DB> + Send,
        for<'connection> &'connection mut <DB as sqlx::Database>::Connection:
            Executor<'connection, Database = DB>,
        EntityId: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + Send + 'query,
    {
        let mut inserts = EntityPrefixedQuery::<'_, DB, EntityId>::new(entity);

        <Self as Serializable<DB>>::insert(&self, &mut inserts);

        async move {
            let mut tx = pool.begin().await.unwrap();
            for query in inserts.queries {
                query.execute(&mut *tx).await.unwrap();
            }

            tx.commit().await.unwrap();
        }
    }

    fn update<'query, EntityId>(
        &'query self,
        pool: &'query Pool<DB>,
        entity: EntityId,
    ) -> impl Future<Output = ()> + Send + 'query
    where
        Self: Serializable<DB> + Send,
        for<'connection> <DB as sqlx::Database>::Arguments<'connection>:
            IntoArguments<'connection, DB> + Send,
        for<'connection> &'connection mut <DB as sqlx::Database>::Connection:
            Executor<'connection, Database = DB>,
        EntityId: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + Send + 'query,
    {
        let mut inserts = EntityPrefixedQuery::<'_, DB, EntityId>::new(entity);

        <Self as Serializable<DB>>::update(&self, &mut inserts);

        async move {
            let mut tx = pool.begin().await.unwrap();
            for query in inserts.queries {
                query.execute(&mut *tx).await.unwrap();
            }

            tx.commit().await.unwrap();
        }
    }

    fn remove<'query, EntityId>(
        pool: &'query Pool<DB>,
        entity: EntityId,
    ) -> impl Future<Output = ()> + Send + 'query
    where
        Self: Removable<DB> + Send,
        for<'connection> <DB as sqlx::Database>::Arguments<'connection>:
            IntoArguments<'connection, DB> + Send,
        for<'connection> &'connection mut <DB as sqlx::Database>::Connection:
            Executor<'connection, Database = DB>,
        EntityId: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + Send + 'query,
    {
        let mut removes = EntityPrefixedQuery::<'_, DB, EntityId>::new(entity);

        <Self as Removable<DB>>::remove(&mut removes);

        async move {
            let mut tx = pool.begin().await.unwrap();
            for query in removes.queries {
                query.execute(&mut *tx).await.unwrap();
            }

            tx.commit().await.unwrap();
        }
    }
}

impl<T, DB: Database> Archetype<DB> for Option<T>
where
    T: Archetype<DB>,
    usize: ColumnIndex<<DB as Database>::Row>,
{
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
macro_rules! impl_compound_for_db{
    ($db:ty, $($list:ident:$index:tt),*) => {
        impl<$($list),*> Archetype<$db> for ($($list,)*)
        where
            $($list: Archetype<$db>,)*
        {

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
