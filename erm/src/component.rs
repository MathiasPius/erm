use sqlx::{query::Query, Database};

use crate::OffsetRow;

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
pub trait Component<DB: Database>: Sized {
    fn table() -> &'static str;
    fn columns() -> Vec<ColumnDefinition<DB>>;
    fn deserialize_fields(row: &mut OffsetRow<<DB as Database>::Row>) -> Result<Self, sqlx::Error>;
    fn serialize_fields<'q>(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;
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
