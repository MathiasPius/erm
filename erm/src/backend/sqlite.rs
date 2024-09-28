use std::{future::Future, marker::PhantomData};

use futures::Stream;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteQueryResult};
use sqlx::{Pool, Sqlite};

use crate::archetype::Archetype;
use crate::condition::Condition;
use crate::prelude::{Component, Serializable};
use crate::tables::Removeable;

use super::Backend;

pub struct SqliteBackend<Entity> {
    pool: Pool<Sqlite>,
    _entity: PhantomData<Entity>,
}

impl<Entity> SqliteBackend<Entity> {
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

impl<Entity> Backend<Sqlite, Entity> for SqliteBackend<Entity>
where
    Entity: for<'q> sqlx::Encode<'q, Sqlite>
        + for<'r> sqlx::Decode<'r, Sqlite>
        + sqlx::Type<Sqlite>
        + Unpin
        + Send
        + 'static,
    for<'entity> &'entity Entity: Send,
{
    fn register<T>(&self) -> impl Future<Output = Result<SqliteQueryResult, sqlx::Error>>
    where
        T: Component<Sqlite>,
    {
        <T as Component<Sqlite>>::create_component_table::<Entity>(&self.pool)
    }

    fn list<T, Cond>(&self, condition: Cond) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>>
    where
        T: Archetype<Sqlite> + Unpin + Send + 'static,
        Cond: for<'c> Condition<'c, Sqlite>,
    {
        <T as Archetype<Sqlite>>::list(&self.pool, condition)
    }

    fn get<T>(&self, entity: &Entity) -> impl Future<Output = Result<T, sqlx::Error>>
    where
        T: Archetype<Sqlite> + Unpin + Send + 'static,
    {
        <T as Archetype<Sqlite>>::get(&self.pool, entity)
    }

    fn insert<'a, 'b, 'c, T>(
        &'a self,
        entity: &'b Entity,
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
        entity: &'a Entity,
        components: &'a T,
    ) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<Sqlite> + Serializable<Sqlite> + Unpin + Send + 'static,
    {
        <T as Archetype<Sqlite>>::update(&components, &self.pool, entity)
    }

    fn remove<'a, T>(&'a self, entity: &'a Entity) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<Sqlite> + Removeable<Sqlite> + Unpin + Send + 'static,
    {
        <T as Archetype<Sqlite>>::remove(&self.pool, entity)
    }
}
