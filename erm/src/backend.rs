use async_trait::async_trait;
use sqlx::{query::Query, Database, Row};

use crate::{archetype::Archetype, component::Component, entity::GenerateUnique, OffsetRow};

pub trait Serialize<'q, DB: Database> {
    fn serialize(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;
}

pub trait Deserialize<'r, R: Row>: Sized + 'static {
    fn deserialize(row: &'r OffsetRow<R>) -> Result<Self, sqlx::Error>;
}

#[async_trait]
pub trait Backend<Entity>
where
    Entity: Send + GenerateUnique,
    Entity: for<'r> Deserialize<'r, <Self::DB as Database>::Row>,
    Entity: for<'q> Serialize<'q, Self::DB>,
{
    type DB: Database;

    async fn init<C>(&self)
    where
        C: Component + Send;

    async fn insert<C>(&self, entity: Entity, component: C)
    where
        C: Component + Send + for<'q> Serialize<'q, Self::DB>;

    async fn list<A>(&self) -> Vec<A>
    where
        A: Archetype + for<'r> Deserialize<'r, <Self::DB as Database>::Row> + Send;

    async fn get<A>(&self, entity: Entity) -> Option<A>
    where
        A: Archetype + for<'r> Deserialize<'r, <Self::DB as Database>::Row>;
}

#[cfg(feature = "sqlite")]
pub mod sqlite;
