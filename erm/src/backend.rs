use std::future::Future;

use futures::Stream;
use sqlx::Database;
use uuid::Uuid;

use crate::{
    condition::{All, Condition},
    Archetype,
};

mod sqlite;

pub use sqlite::SqliteBackend;

pub trait GenerateNew {
    fn generate_new() -> Self;
}

impl GenerateNew for Uuid {
    fn generate_new() -> Self {
        Uuid::new_v4()
    }
}

pub trait Backend<DB, Entity>: Sized
where
    DB: Database,
    Entity: for<'q> sqlx::Encode<'q, DB>
        + for<'r> sqlx::Decode<'r, DB>
        + sqlx::Type<DB>
        + Send
        + 'static,
{
    fn init<T>(&self) -> impl Future<Output = Result<(), sqlx::Error>>
    where
        T: Archetype<DB>;

    fn spawn<'a, T>(&'a self, components: &'a T) -> impl Future<Output = Entity> + 'a
    where
        Entity: GenerateNew,
        T: Archetype<DB> + Unpin + Send + 'static,
    {
        async move {
            let entity = Entity::generate_new();
            self.insert(&entity, components).await;
            entity
        }
    }

    fn insert<'a, 'b, 'c, T>(
        &'a self,
        entity: &'b Entity,
        components: &'c T,
    ) -> impl Future<Output = ()> + Send + 'c
    where
        'a: 'b,
        'b: 'c,
        T: Archetype<DB> + Unpin + Send + 'static;

    fn update<'a, T>(
        &'a self,
        entity: &'a Entity,
        components: &'a T,
    ) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<DB> + Unpin + Send + 'static;

    fn list<T, Cond>(&self, cond: Cond) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>>
    where
        T: Archetype<DB> + Unpin + Send + 'static,
        Cond: for<'c> Condition<'c, DB>;

    fn list_all<T>(&self) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>>
    where
        T: Archetype<DB> + Unpin + Send + 'static,
    {
        self.list(All)
    }

    fn get<T>(&self, entity: &Entity) -> impl Future<Output = Result<T, sqlx::Error>>
    where
        T: Archetype<DB> + Unpin + Send + 'static;
}
