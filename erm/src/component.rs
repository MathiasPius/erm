use std::future::Future;

use sqlx::{Database, Pool};

use crate::{
    entity::EntityPrefixedQuery,
    serialization::{Deserializeable, Serializable},
};

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
pub trait Component<DB: Database>: Serializable<DB> + Deserializeable<DB> + Sized {
    const OPTIONAL: bool = false;
    const INSERT: &'static str;
    const UPDATE: &'static str;
    const DELETE: &'static str;

    fn table() -> &'static str;

    fn columns() -> Vec<ColumnDefinition<DB>>;

    fn remove_component<'query, Entity>(query: &mut EntityPrefixedQuery<'query, DB, Entity>)
    where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query,
    {
        query.query(Self::DELETE, |query| query)
    }

    fn create_component_table<'pool, Entity>(
        pool: &'pool Pool<DB>,
    ) -> impl Future<Output = Result<<DB as Database>::QueryResult, sqlx::Error>> + Send
    where
        Entity: sqlx::Type<DB>;
}
