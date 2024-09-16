use std::{future::Future, marker::PhantomData};

use futures::Stream;
use sqlx::{Pool, Postgres};

use crate::archetype::Archetype;
use crate::condition::Condition;

use super::Backend;

pub struct PostgresBackend<Entity> {
    pool: Pool<Postgres>,
    _entity: PhantomData<Entity>,
}

impl<Entity> PostgresBackend<Entity> {
    pub fn new(pool: Pool<Postgres>) -> Self {
        PostgresBackend {
            pool,
            _entity: PhantomData,
        }
    }
}

impl<Entity> Backend<Postgres, Entity> for PostgresBackend<Entity>
where
    Entity: for<'q> sqlx::Encode<'q, Postgres>
        + for<'r> sqlx::Decode<'r, Postgres>
        + sqlx::Type<Postgres>
        + Unpin
        + Send
        + 'static,
    for<'entity> &'entity Entity: Send,
{
    fn init<T>(&self) -> impl Future<Output = Result<(), sqlx::Error>>
    where
        T: Archetype<Postgres>,
    {
        <T as Archetype<Postgres>>::create_component_tables::<Entity>(&self.pool)
    }

    fn list<T, Cond>(&self, condition: Cond) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>>
    where
        T: Archetype<Postgres> + Unpin + Send + 'static,
        Cond: for<'c> Condition<'c, Postgres>,
    {
        <T as Archetype<Postgres>>::list(&self.pool, condition)
    }

    fn get<T>(&self, entity: &Entity) -> impl Future<Output = Result<T, sqlx::Error>>
    where
        T: Archetype<Postgres> + Unpin + Send + 'static,
    {
        <T as Archetype<Postgres>>::get(&self.pool, entity)
    }

    fn insert<'a, 'b, 'c, T>(
        &'a self,
        entity: &'b Entity,
        components: &'c T,
    ) -> impl Future<Output = ()> + Send + 'c
    where
        'a: 'b,
        'b: 'c,
        T: Archetype<Postgres> + Unpin + Send + 'static,
    {
        <T as Archetype<Postgres>>::insert(&components, &self.pool, entity)
    }

    fn update<'a, T>(
        &'a self,
        entity: &'a Entity,
        components: &'a T,
    ) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<Postgres> + Unpin + Send + 'static,
    {
        <T as Archetype<Postgres>>::update(&components, &self.pool, entity)
    }

    fn delete<'a, T>(&'a self, entity: &'a Entity) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<Postgres> + Unpin + Send + 'static,
    {
        <T as Archetype<Postgres>>::delete(&self.pool, entity)
    }
}
