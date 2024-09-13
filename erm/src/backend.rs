use std::future::Future;

use futures::Stream;
use sqlx::Database;

use crate::{
    condition::{All, Condition},
    Archetype,
};

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
    fn list<T, Cond>(
        &self,
        condition: Cond,
    ) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>> + Send
    where
        T: Archetype<DB> + Unpin + Send + 'static,
        Cond: Condition<Entity>;

    fn list_all<T>(&self) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>> + Send
    where
        T: Archetype<DB> + Unpin + Send + 'static,
    {
        self.list(All)
    }

    fn get<T>(&self, entity: &Entity) -> impl Future<Output = Result<T, sqlx::Error>>
    where
        T: Archetype<DB> + Unpin + Send + 'static;
}
