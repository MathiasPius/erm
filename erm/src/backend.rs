use std::{future::Future, marker::PhantomData};

use async_stream::stream;
use futures::Stream;
use sqlx::{ColumnIndex, Database, Executor, IntoArguments, Pool};

#[cfg(feature = "uuid")]
use uuid::Uuid;

use crate::{
    archetype::Archetype,
    condition::{All, Condition},
    cte::{Filter, With, Without},
    prelude::{Component, Deserializeable, Serializable},
    row::Entity,
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

pub trait Backend<DB, EntityId>: Sized
where
    DB: Database,
    EntityId: for<'q> sqlx::Encode<'q, DB>
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

    fn spawn<'a, T>(&'a self, components: &'a T) -> impl Future<Output = EntityId> + 'a
    where
        EntityId: GenerateNew,
        T: Archetype<DB> + Serializable<DB> + Unpin + Send + 'static,
    {
        async move {
            let entity = EntityId::generate_new();
            self.insert(&entity, components).await;
            entity
        }
    }

    fn insert<'a, 'b, 'c, T>(
        &'a self,
        entity: &'b EntityId,
        components: &'c T,
    ) -> impl Future<Output = ()> + Send + 'c
    where
        'a: 'b,
        'b: 'c,
        T: Archetype<DB> + Serializable<DB> + Unpin + Send + 'static;

    fn update<'a, T>(
        &'a self,
        entity: &'a EntityId,
        components: &'a T,
    ) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<DB> + Serializable<DB> + Unpin + Send + 'static;

    fn remove<'a, T>(&'a self, entity: &'a EntityId) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<DB> + Removable<DB> + Unpin + Send + 'static;

    /// Query the backend for entities.
    ///
    /// See [`List`] for options allowng further filtering and processing.
    fn list<T>(&self) -> List<DB, EntityId, T, (), All>;

    fn get<T>(&self, entity: &EntityId) -> impl Future<Output = Result<T, sqlx::Error>>
    where
        T: Deserializeable<DB> + Unpin + Send + 'static;
}

/// Configurable query for entities.
pub struct List<
    DB,
    EntityId,
    T,
    F = (),
    C = All,
    Out = Entity<EntityId, T>,
    Map = fn(Entity<EntityId, T>) -> Out,
> where
    DB: Database,
{
    pool: Pool<DB>,
    _data: PhantomData<(EntityId, T, F, Out)>,
    map: Map,
    condition: C,
}

impl<DB, EntityId, T, F, C, Out, Map> List<DB, EntityId, T, F, C, Out, Map>
where
    DB: Database,
{
    /// Include only entities that contain the components `Include`, but do not return
    /// return these components as part of the query.
    ///
    /// This is especially useful for Marker components, that don't contain
    /// any information by themselves, but whose presence has implications
    /// for the state of the entity.
    pub fn with<Include: Deserializeable<DB>>(
        self,
    ) -> List<DB, EntityId, T, (With<Include>, F), C, Out, Map> {
        List {
            pool: self.pool,
            _data: PhantomData,
            condition: self.condition,
            map: self.map,
        }
    }

    /// Exclude entities that contain the components `Exclude`.
    ///
    /// This is especially useful for Marker components, that don't contain
    /// any information by themselves, but whose absence has implications
    /// for the state of the entity.
    pub fn without<Exclude: Deserializeable<DB>>(
        self,
    ) -> List<DB, EntityId, T, (Without<Exclude>, F), C, Out, Map> {
        List {
            pool: self.pool,
            _data: PhantomData,
            condition: self.condition,
            map: self.map,
        }
    }

    /// Apply the given condition to the query, filtering the results before
    /// returning them from the database.
    ///
    /// See [`crate::condition`] for constructing conditionals.
    pub fn filter<'q, Cond: Condition<'q, DB>>(
        self,
        condition: Cond,
    ) -> List<DB, EntityId, T, F, Cond, Out, Map> {
        List {
            pool: self.pool,
            _data: PhantomData,
            condition,
            map: self.map,
        }
    }

    /// Map each [`Entity<EntityId, T>`] using a custom mapping function before yielding the result.
    pub fn map<M>(
        self,
        map: fn(Entity<EntityId, T>) -> M,
    ) -> List<DB, EntityId, T, F, C, M, fn(Entity<EntityId, T>) -> M> {
        List {
            pool: self.pool,
            _data: PhantomData,
            condition: self.condition,
            map,
        }
    }

    /// Return only the Entity IDs of the returned entities, discarding the components.
    pub fn ids(self) -> List<DB, EntityId, T, F, C, EntityId, fn(Entity<EntityId, T>) -> EntityId> {
        fn ids<EntityId, T>(entity: Entity<EntityId, T>) -> EntityId {
            entity.into_id()
        }

        self.map(ids::<EntityId, T>)
    }

    /// Return only the components themselves, discarding the Entity IDs associated with each entity.
    pub fn components(self) -> List<DB, EntityId, T, F, C, T, fn(Entity<EntityId, T>) -> T> {
        fn components<EntityId, T>(entity: Entity<EntityId, T>) -> T {
            entity.into_components()
        }

        self.map(components::<EntityId, T>)
    }
}

impl<DB, EntityId, T, F, Cond, Out, Map: Fn(Entity<EntityId, T>) -> Out>
    List<DB, EntityId, T, F, Cond, Out, Map>
where
    DB: Database,
    T: Deserializeable<DB> + Unpin + Send,
    F: Filter<DB>,
    Cond: for<'c> Condition<'c, DB>,
    for<'c> <DB as sqlx::Database>::Arguments<'c>: IntoArguments<'c, DB> + Send,
    for<'c> &'c mut <DB as sqlx::Database>::Connection: Executor<'c, Database = DB>,
    for<'e> EntityId: sqlx::Decode<'e, DB> + sqlx::Encode<'e, DB> + sqlx::Type<DB> + Unpin + Send,
    usize: ColumnIndex<<DB as sqlx::Database>::Row>,
{
    /// Execute the query, returning a stream of results.
    pub fn fetch(self) -> impl Stream<Item = Result<Out, sqlx::Error>> {
        stream! {
            let mut sql = crate::cte::serialize(<F as Filter<DB>>::cte(<T as Deserializeable<DB>>::cte()).as_ref()).unwrap();
            sql.push_str(" where ");
            self.condition.serialize(&mut sql).unwrap();

            let query = self.condition.bind(sqlx::query_as::<DB, Entity<EntityId, T>>(&sql));

            for await result in query.fetch(&self.pool) {
                yield match result {
                    Ok(result) => Ok((self.map)(result)),
                    Err(err) => Err(err)
                }
            }
        }
    }
}
