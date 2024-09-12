use std::marker::PhantomData;

use futures::Stream;
use sqlx::{Pool, Sqlite};

use crate::Archetype;

use super::Backend;

pub struct SqliteBackend<Entity> {
    pool: Pool<Sqlite>,
    _entity: PhantomData<Entity>,
}
/*
impl<Entity> Backend<Sqlite, Entity> for SqliteBackend<Entity>
where
    Entity:
        for<'q> sqlx::Encode<'q, Sqlite> + for<'r> sqlx::Decode<'r, Sqlite> + sqlx::Type<Sqlite>,
{
    fn list<T>(&self) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>> + Send
    where
        T: Archetype<Sqlite> + Unpin + Send + Sync,
    {
        <T as Archetype<Sqlite>>::list(&self.pool)
    }
}
 */
