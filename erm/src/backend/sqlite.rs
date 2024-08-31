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
use sqlx::{prelude::Type, sqlite::SqliteRow, Encode, Sqlite, SqlitePool};
use std::fmt::Write;

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
    Entity: for<'q> Encode<'q, Sqlite> + Type<Sqlite>,
{
    type DB = Sqlite;

    async fn init<C>(&self)
    where
        C: Component + Send,
    {
        let statement = Create::<Sqlite>::from(&C::DESCRIPTION).to_sql().unwrap();

        sqlx::query(&statement).execute(&self.pool).await.unwrap();
    }

    async fn insert<A>(&self, entity: &Entity, components: A)
    where
        A: Archetype + Send + for<'r> Serialize<'r, Sqlite, Entity>,
    {
        let mut sql = "begin transaction;".to_string();
        for component in <A as Archetype>::COMPONENTS {
            writeln!(
                sql,
                "{};",
                Insert::<Sqlite>::from(component).to_sql().unwrap()
            )
            .unwrap();
        }

        writeln!(sql, "commit transaction;").unwrap();

        let q = sqlx::query(&sql);
        let q = components.serialize(q, entity);

        q.execute(&self.pool).await.unwrap();
    }

    async fn list<A: Archetype + for<'r> Deserialize<'r, SqliteRow> + Send>(&self) -> Vec<A> {
        let select = Compound::from(&A::as_description()).to_sql().unwrap();

        let mut entities = Vec::new();
        let mut stream = sqlx::query(&select).fetch(&self.pool);

        while let Some(row) = stream.next().await {
            let sqlite_row = row.unwrap();
            let offset = OffsetRow::new(&sqlite_row);
            let out = A::deserialize(&offset).unwrap();
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
        let q = q.bind(entity);

        let sqlite_row = q.fetch_optional(&self.pool).await.unwrap()?;
        let offset = OffsetRow::new(&sqlite_row);
        Some(A::deserialize(&offset).unwrap())
    }
}
