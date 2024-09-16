use std::{future::Future, marker::PhantomData};

use futures::Stream;
use sqlx::{MySql, Pool};

use crate::archetype::Archetype;
use crate::condition::Condition;

use super::Backend;

pub struct MySqlBackend<Entity> {
    pool: Pool<MySql>,
    _entity: PhantomData<Entity>,
}

impl<Entity> MySqlBackend<Entity> {
    pub fn new(pool: Pool<MySql>) -> Self {
        MySqlBackend {
            pool,
            _entity: PhantomData,
        }
    }
}

impl<Entity> Backend<MySql, Entity> for MySqlBackend<Entity>
where
    Entity: for<'q> sqlx::Encode<'q, MySql>
        + for<'r> sqlx::Decode<'r, MySql>
        + sqlx::Type<MySql>
        + Unpin
        + Send
        + 'static,
    for<'entity> &'entity Entity: Send,
{
    fn register<T>(&self) -> impl Future<Output = Result<(), sqlx::Error>>
    where
        T: Archetype<MySql>,
    {
        <T as Archetype<MySql>>::create_component_tables::<Entity>(&self.pool)
    }

    fn list<T, Cond>(&self, condition: Cond) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>>
    where
        T: Archetype<MySql> + Unpin + Send + 'static,
        Cond: for<'c> Condition<'c, MySql>,
    {
        <T as Archetype<MySql>>::list(&self.pool, condition)
    }

    fn get<T>(&self, entity: &Entity) -> impl Future<Output = Result<T, sqlx::Error>>
    where
        T: Archetype<MySql> + Unpin + Send + 'static,
    {
        <T as Archetype<MySql>>::get(&self.pool, entity)
    }

    fn insert<'a, 'b, 'c, T>(
        &'a self,
        entity: &'b Entity,
        components: &'c T,
    ) -> impl Future<Output = ()> + Send + 'c
    where
        'a: 'b,
        'b: 'c,
        T: Archetype<MySql> + Unpin + Send + 'static,
    {
        <T as Archetype<MySql>>::insert(&components, &self.pool, entity)
    }

    fn update<'a, T>(
        &'a self,
        entity: &'a Entity,
        components: &'a T,
    ) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<MySql> + Unpin + Send + 'static,
    {
        <T as Archetype<MySql>>::update(&components, &self.pool, entity)
    }

    fn remove<'a, T>(&'a self, entity: &'a Entity) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<MySql> + Unpin + Send + 'static,
    {
        <T as Archetype<MySql>>::remove(&self.pool, entity)
    }
}
