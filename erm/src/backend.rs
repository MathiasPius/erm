use std::future::Future;

use futures::Stream;
use sqlx::Database;

use crate::Archetype;

mod sqlite;

pub use sqlite::SqliteBackend;

pub trait Backend<DB, Entity>: Sized
where
    DB: Database,
    Entity: for<'q> sqlx::Encode<'q, DB>
        + for<'r> sqlx::Decode<'r, DB>
        + sqlx::Type<DB>
        + Send
        + 'static,
{
    fn list<T>(&self) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>> + Send
    where
        T: Archetype<DB> + Unpin + Send + 'static;

    fn get<'pool, 'entity, T>(
        &'pool self,
        entity: &'entity Entity,
    ) -> impl Future<Output = Result<T, sqlx::Error>> + Send + 'entity
    where
        'pool: 'entity,
        T: Archetype<DB> + Unpin + Send + 'static;
}
