use std::{future::Future, marker::PhantomData};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteQueryResult};
use sqlx::{Pool, Sqlite};

use crate::archetype::Archetype;
use crate::condition::All;
use crate::prelude::{Component, Deserializeable, Serializable};
use crate::row::Entity;
use crate::tables::Removable;

use super::{Backend, List};

pub struct SqliteBackend<EntityId> {
    pool: Pool<Sqlite>,
    _entity: PhantomData<EntityId>,
}

impl<EntityId> SqliteBackend<EntityId> {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        SqliteBackend {
            pool,
            _entity: PhantomData,
        }
    }

    pub async fn in_memory() -> Self {
        let options = SqliteConnectOptions::new().in_memory(true);

        let pool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await
            .unwrap();

        Self::new(pool)
    }
}

impl<EntityId> Backend<Sqlite, EntityId> for SqliteBackend<EntityId>
where
    EntityId: for<'q> sqlx::Encode<'q, Sqlite>
        + for<'r> sqlx::Decode<'r, Sqlite>
        + sqlx::Type<Sqlite>
        + Unpin
        + Send
        + 'static,
    for<'entity> &'entity EntityId: Send,
{
    fn register<T>(&self) -> impl Future<Output = Result<SqliteQueryResult, sqlx::Error>>
    where
        T: Component<Sqlite>,
    {
        <T as Component<Sqlite>>::create_component_table::<EntityId>(&self.pool)
    }

    fn list<T>(&self) -> List<Sqlite, EntityId, T, (), All> {
        fn identity<EntityId, T>(entity: Entity<EntityId, T>) -> Entity<EntityId, T> {
            entity
        }
        List {
            pool: self.pool.clone(),
            _data: PhantomData,
            condition: All,
            map: identity::<EntityId, T>,
        }
    }

    fn get<T>(&self, entity: &EntityId) -> impl Future<Output = Result<T, sqlx::Error>>
    where
        T: Deserializeable<Sqlite> + Unpin + Send + 'static,
    {
        async move {
            let sql =
                crate::cte::serialize(<T as Deserializeable<Sqlite>>::cte().as_ref()).unwrap();

            let result: Entity<EntityId, T> = sqlx::query_as(&sql)
                .bind(entity)
                .fetch_one(&self.pool)
                .await?;

            Ok(result.into_components())
        }
    }

    fn insert<'a, 'b, 'c, T>(
        &'a self,
        entity: &'b EntityId,
        components: &'c T,
    ) -> impl Future<Output = ()> + Send + 'c
    where
        'a: 'b,
        'b: 'c,
        T: Archetype<Sqlite> + Serializable<Sqlite> + Unpin + Send + 'static,
    {
        <T as Archetype<Sqlite>>::insert(&components, &self.pool, entity)
    }

    fn update<'a, T>(
        &'a self,
        entity: &'a EntityId,
        components: &'a T,
    ) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<Sqlite> + Serializable<Sqlite> + Unpin + Send + 'static,
    {
        <T as Archetype<Sqlite>>::update(&components, &self.pool, entity)
    }

    fn remove<'a, T>(&'a self, entity: &'a EntityId) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<Sqlite> + Removable<Sqlite> + Unpin + Send + 'static,
    {
        <T as Archetype<Sqlite>>::remove(&self.pool, entity)
    }
}
