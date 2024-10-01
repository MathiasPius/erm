use std::{future::Future, marker::PhantomData};

use async_stream::stream;
use futures::Stream;
use sqlx::{ColumnIndex, Database, Executor, IntoArguments, Pool};

#[cfg(feature = "uuid")]
use uuid::Uuid;

use crate::{
    archetype::Archetype,
    condition::{All, And, Condition, Or},
    cte::{Filter, With, Without},
    prelude::{Component, Deserializeable, Serializable},
    row::Rowed,
    tables::Removable,
};

#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteBackend;

#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;

#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "mysql")]
pub use mysql::MySqlBackend;

pub trait GenerateNew {
    fn generate_new() -> Self;
}

#[cfg(feature = "uuid")]
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
    fn register<T>(
        &self,
    ) -> impl Future<Output = Result<<DB as Database>::QueryResult, sqlx::Error>>
    where
        T: Component<DB>;

    fn spawn<'a, T>(&'a self, components: &'a T) -> impl Future<Output = Entity> + 'a
    where
        Entity: GenerateNew,
        T: Archetype<DB> + Serializable<DB> + Unpin + Send + 'static,
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
        T: Archetype<DB> + Serializable<DB> + Unpin + Send + 'static;

    fn update<'a, T>(
        &'a self,
        entity: &'a Entity,
        components: &'a T,
    ) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<DB> + Serializable<DB> + Unpin + Send + 'static;

    fn remove<'a, T>(&'a self, entity: &'a Entity) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<DB> + Removable<DB> + Unpin + Send + 'static;

    fn list<T>(&self) -> List<DB, Entity, T, (), All>;

    fn get<T>(&self, entity: &Entity) -> impl Future<Output = Result<T, sqlx::Error>>
    where
        T: Deserializeable<DB> + Unpin + Send + 'static;
}

pub struct List<DB, Entity, T, F = (), C = All>
where
    DB: Database,
{
    pool: Pool<DB>,
    _data: PhantomData<(Entity, T, F)>,
    condition: C,
}

impl<DB, Entity, T, F, C> List<DB, Entity, T, F, C>
where
    DB: Database,
{
    pub fn with<U: Deserializeable<DB>>(self) -> List<DB, Entity, T, (With<U>, F), C> {
        List {
            pool: self.pool,
            _data: PhantomData,
            condition: self.condition,
        }
    }

    pub fn without<U: Deserializeable<DB>>(self) -> List<DB, Entity, T, (Without<U>, F), C> {
        List {
            pool: self.pool,
            _data: PhantomData,
            condition: self.condition,
        }
    }

    pub fn and<'q, Cond: Condition<'q, DB>>(
        self,
        condition: Cond,
    ) -> List<DB, Entity, T, F, And<C, Cond>> {
        List {
            pool: self.pool,
            _data: PhantomData,
            condition: And::new(self.condition, condition),
        }
    }

    pub fn or<'q, Cond: Condition<'q, DB>>(
        self,
        condition: Cond,
    ) -> List<DB, Entity, T, F, Or<C, Cond>> {
        List {
            pool: self.pool,
            _data: PhantomData,
            condition: Or::new(self.condition, condition),
        }
    }
}

impl<DB, Entity, T, F, Cond> List<DB, Entity, T, F, Cond>
where
    DB: Database,
    T: Deserializeable<DB> + Unpin + Send,
    F: Filter<DB>,
    Cond: for<'c> Condition<'c, DB>,
    for<'c> <DB as sqlx::Database>::Arguments<'c>: IntoArguments<'c, DB> + Send,
    for<'c> &'c mut <DB as sqlx::Database>::Connection: Executor<'c, Database = DB>,
    for<'e> Entity: sqlx::Decode<'e, DB> + sqlx::Encode<'e, DB> + sqlx::Type<DB> + Unpin + Send,
    usize: ColumnIndex<<DB as sqlx::Database>::Row>,
{
    pub fn fetch(self) -> impl Stream<Item = Result<(Entity, T), sqlx::Error>> {
        stream! {
            let mut sql = crate::cte::serialize(<F as Filter<DB>>::cte(<T as Deserializeable<DB>>::cte()).as_ref()).unwrap();
            sql.push_str(" where ");
            self.condition.serialize(&mut sql).unwrap();

            let query = self.condition.bind(sqlx::query_as::<DB, Rowed<Entity, T>>(&sql));

            for await row in query.fetch(&self.pool) {
                yield row.map(|rowed| (rowed.entity, rowed.inner))
            }
        }
    }
}
