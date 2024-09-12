use std::future::Future;

use sqlx::{query::Query, Database, Pool};

use crate::{entity::EntityPrefixedQuery, OffsetRow};

pub struct ColumnDefinition<DB: Database> {
    pub name: &'static str,
    pub type_info: <DB as Database>::TypeInfo,
}

impl<DB: Database> ColumnDefinition<DB> {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn type_info(&self) -> &<DB as Database>::TypeInfo {
        &self.type_info
    }
}

/// Describes reading and writing from a Component-specific Table.
pub trait Component<DB: Database>: std::fmt::Debug + Sized {
    const INSERT: &'static str;
    const UPDATE: &'static str;

    fn table() -> &'static str;

    fn columns() -> Vec<ColumnDefinition<DB>>;

    fn deserialize_fields(row: &mut OffsetRow<<DB as Database>::Row>) -> Result<Self, sqlx::Error>;

    fn serialize_fields<'query>(
        &'query self,
        query: Query<'query, DB, <DB as Database>::Arguments<'query>>,
    ) -> Query<'query, DB, <DB as Database>::Arguments<'query>>;

    fn insert_component<'query, Entity>(
        &'query self,
        query: &mut EntityPrefixedQuery<'query, DB, Entity>,
    ) where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query,
    {
        query.query(Self::INSERT, move |query| self.serialize_fields(query))
    }

    fn update_component<'query, Entity>(
        &'query self,
        query: &mut EntityPrefixedQuery<'query, DB, Entity>,
    ) where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query,
    {
        query.query(Self::UPDATE, move |query| self.serialize_fields(query))
    }

    fn create_component_table<'pool, Entity>(
        pool: &'pool Pool<DB>,
    ) -> impl Future<Output = Result<<DB as Database>::QueryResult, sqlx::Error>> + Send
    where
        Entity: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB> + Clone;
}
