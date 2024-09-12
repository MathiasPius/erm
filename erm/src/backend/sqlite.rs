use std::{future::Future, marker::PhantomData};

use futures::Stream;
use sqlx::{Pool, Sqlite};

use crate::Archetype;

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
    fn list<T>(&self) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>> + Send
    where
        T: Archetype<Sqlite> + Unpin + Send + 'static,
    {
        <T as Archetype<Sqlite>>::list(&self.pool)
    }

    fn get<'pool, 'entity, T>(
        &'pool self,
        entity: &'entity Entity,
    ) -> impl Future<Output = Result<T, sqlx::Error>> + Send + 'entity
    where
        T: Archetype<Sqlite> + Unpin + Send + 'static,
        'pool: 'entity,
    {
        <T as Archetype<Sqlite>>::get(&self.pool, entity)
    }
}
