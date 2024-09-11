use std::future::Future;

use sqlx::{query::Query, Database, Pool};

use crate::{insert::InsertionQuery, OffsetRow};

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

    fn table() -> &'static str;

    fn columns() -> Vec<ColumnDefinition<DB>>;

    fn deserialize_fields(row: &mut OffsetRow<<DB as Database>::Row>) -> Result<Self, sqlx::Error>;

    fn serialize_fields<'query>(
        &'query self,
        query: Query<'query, DB, <DB as Database>::Arguments<'query>>,
    ) -> Query<'query, DB, <DB as Database>::Arguments<'query>>;

    fn insert_component<'query, Entity>(
        &'query self,
        query: &mut InsertionQuery<'query, DB, Entity>,
    ) where
        Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query,
    {
        query.query(Self::INSERT, move |query| self.serialize_fields(query))
    }

    fn create_table<'pool, Entity>(
        pool: &'pool Pool<DB>,
    ) -> impl Future<Output = Result<<DB as Database>::QueryResult, sqlx::Error>> + Send
    where
        Entity: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB> + Clone;
}

#[cfg(test)]
mod tests {
    use sqlx::{
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
        Executor as _,
    };

    #[tokio::test]
    async fn test_func() {
        let options = SqliteConnectOptions::new().in_memory(true);

        let db = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await
            .unwrap();

        db.execute(
            r#"
            create table if not exists positions(
                entity text primary key,
                x real,
                y real
            );
            "#,
        )
        .await
        .unwrap();

        db.execute(
            r#"
            create table if not exists real_names(
                entity text primary key,
                real_name text
            );
            "#,
        )
        .await
        .unwrap();

        db.execute(
            r#"
            insert or ignore into positions(entity, x, y) values('a', 10.0, 20.0);
            insert or ignore into positions(entity, x, y) values('b', 30.0, 40.0);
            insert or ignore into real_names(entity, real_name) values("a", "first");
            insert or ignore into real_names(entity, real_name) values("b", "second");
        "#,
        )
        .await
        .unwrap();
    }
}
