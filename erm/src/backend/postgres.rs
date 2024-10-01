use std::{future::Future, marker::PhantomData};

use sqlx::postgres::PgQueryResult;
use sqlx::{Pool, Postgres};

use crate::archetype::Archetype;
use crate::condition::All;
use crate::prelude::{Component, Deserializeable, Serializable};
use crate::row::Rowed;
use crate::tables::Removable;

use super::{Backend, List};

pub struct PostgresBackend<Entity> {
    pool: Pool<Postgres>,
    _entity: PhantomData<Entity>,
}

impl<Entity> PostgresBackend<Entity> {
    pub fn new(pool: Pool<Postgres>) -> Self {
        PostgresBackend {
            pool,
            _entity: PhantomData,
        }
    }
}

impl<Entity> Backend<Postgres, Entity> for PostgresBackend<Entity>
where
    Entity: for<'q> sqlx::Encode<'q, Postgres>
        + for<'r> sqlx::Decode<'r, Postgres>
        + sqlx::Type<Postgres>
        + Unpin
        + Send
        + 'static,
    for<'entity> &'entity Entity: Send,
{
    fn register<T>(&self) -> impl Future<Output = Result<PgQueryResult, sqlx::Error>>
    where
        T: Component<Postgres>,
    {
        <T as Component<Postgres>>::create_component_table::<Entity>(&self.pool)
    }

    fn list<T>(&self) -> List<Postgres, Entity, T, (), All> {
        List {
            pool: self.pool.clone(),
            _data: PhantomData,
            condition: All,
        }
    }

    fn get<T>(&self, entity: &Entity) -> impl Future<Output = Result<T, sqlx::Error>>
    where
        T: Deserializeable<Postgres> + Unpin + Send + 'static,
    {
        async move {
            let sql =
                crate::cte::serialize(<T as Deserializeable<Postgres>>::cte().as_ref()).unwrap();

            let result: Rowed<Entity, T> = sqlx::query_as(&sql)
                .bind(entity)
                .fetch_one(&self.pool)
                .await?;

            Ok(result.inner)
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
        T: Archetype<Postgres> + Serializable<Postgres> + Unpin + Send + 'static,
    {
        <T as Archetype<Postgres>>::insert(&components, &self.pool, entity)
    }

    fn update<'a, T>(
        &'a self,
        entity: &'a Entity,
        components: &'a T,
    ) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<Postgres> + Serializable<Postgres> + Unpin + Send + 'static,
    {
        <T as Archetype<Postgres>>::update(&components, &self.pool, entity)
    }

    fn remove<'a, T>(&'a self, entity: &'a Entity) -> impl Future<Output = ()> + 'a
    where
        T: Archetype<Postgres> + Removable<Postgres> + Unpin + Send + 'static,
    {
        <T as Archetype<Postgres>>::remove(&self.pool, entity)
    }
}
