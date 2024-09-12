use futures::Stream;
use sqlx::Database;

use crate::Archetype;

mod sqlite;

pub trait Backend<DB, Entity>: Sized
where
    DB: Database,
    Entity: for<'q> sqlx::Encode<'q, DB> + for<'r> sqlx::Decode<'r, DB> + sqlx::Type<DB>,
{
    fn list<T>(&self) -> impl Stream<Item = Result<(Entity, Self), sqlx::Error>> + Send
    where
        T: Archetype<DB> + Unpin + Send;
}
