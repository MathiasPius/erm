use std::marker::PhantomData;

use super::*;
use crate::{
    archetype::Archetype,
    create::Create,
    entity::GenerateUnique,
    insert::Insert,
    select::{Column, Compound, ToSql as _},
};
use async_trait::async_trait;
use futures::StreamExt as _;
use sqlx::{sqlite::SqliteRow, Sqlite, SqlitePool};

pub struct SqliteBackend<Entity> {
    pool: SqlitePool,
    _entity: PhantomData<Entity>,
}

impl<Entity> SqliteBackend<Entity> {
    pub fn new(pool: SqlitePool) -> Self {
        SqliteBackend {
            pool,
            _entity: PhantomData,
        }
    }
}

#[async_trait]
impl<Entity> Backend<Entity> for SqliteBackend<Entity>
where
    Entity: Send + GenerateUnique + Sync,
    Entity: for<'r> Deserialize<'r, SqliteRow>,
    Entity: for<'q> Serialize<'q, Sqlite>,
{
    type DB = Sqlite;

    async fn init<C>(&self)
    where
        C: Component + Send,
    {
        let statement = Create::<Sqlite>::from(&C::DESCRIPTION).to_sql().unwrap();

        sqlx::query(&statement).execute(&self.pool).await.unwrap();
    }

    async fn insert<C>(&self, entity: Entity, component: C)
    where
        C: Component + Send + for<'r> Serialize<'r, Sqlite>,
    {
        let insert = Insert::<Sqlite>::from(&C::DESCRIPTION).to_sql().unwrap();

        let q = sqlx::query(&insert);
        let q = entity.serialize(q);
        let q = component.serialize(q);

        q.execute(&self.pool).await.unwrap();
    }

    async fn list<A: Archetype + for<'r> Deserialize<'r, SqliteRow> + Send>(&self) -> Vec<A> {
        let select = Compound::from(&A::as_description()).to_sql().unwrap();

        let mut entities = Vec::new();
        let mut stream = sqlx::query(&select).fetch(&self.pool);

        while let Some(row) = stream.next().await {
            let sqlite_row = row.unwrap();
            //let row = sqlx::any::AnyRow::map_from(&sqlite_row, std::sync::Arc::default()).unwrap();
            let offset = OffsetRow::new(&sqlite_row);
            let _ = Entity::deserialize(&offset).unwrap();
            let offset1 = offset.offset_by(1);
            let out = A::deserialize(&offset1).unwrap();
            entities.push(out);
        }

        entities
    }

    async fn get<A: Archetype + for<'r> Deserialize<'r, SqliteRow>>(
        &self,
        entity: Entity,
    ) -> Option<A> {
        let select = Compound::from(&A::as_description());
        let column = Column {
            table: select.source.table,
            name: "entity",
        };

        let select = select.filter(column).to_sql().unwrap();

        let q = sqlx::query(&select);
        let q = entity.serialize(q);

        let sqlite_row = q.fetch_optional(&self.pool).await.unwrap()?;
        let offset = OffsetRow::new(&sqlite_row);
        Some(A::deserialize(&offset).unwrap())
    }
}
