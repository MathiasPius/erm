use std::{marker::PhantomData, sync::OnceLock};

use sqlx::{query::Query, Database, Executor};

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
    fn insertion_query_static() -> &'static OnceLock<String>;
    fn table() -> &'static str;
    fn columns() -> Vec<ColumnDefinition<DB>>;
    fn deserialize_fields(row: &mut OffsetRow<<DB as Database>::Row>) -> Result<Self, sqlx::Error>;
    fn serialize_fields<'q>(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;

    fn insertion_query<'q, Entity>(&'q self, query: &mut InsertionQuery<'q, DB, Entity>)
    where
        Entity: sqlx::Encode<'q, DB> + sqlx::Type<DB> + std::fmt::Debug + Clone + 'q,
    {
        let sql = Self::insertion_query_static();

        let table = Self::table();

        let entity = ColumnDefinition {
            name: "entity",
            type_info: <&Entity as sqlx::Type<DB>>::type_info(),
        };

        let columns = [entity]
            .iter()
            .chain(Self::columns().iter())
            .map(|column| column.name())
            .collect::<Vec<_>>();

        let bindings = std::iter::repeat("?")
            .take(columns.len())
            .collect::<Vec<_>>()
            .join(", ");

        let columns = columns.join(", ");

        let sql = sql.get_or_init(|| format!("insert into {table}({columns}) values({bindings})"));

        println!("{sql}");
        println!("{self:#?}");

        query.query(sql, move |query| self.serialize_fields(query))
    }

    fn create<'e, E>(
        executor: &'e E,
    ) -> impl std::future::Future<Output = Result<<DB as Database>::QueryResult, sqlx::Error>> + Send
    where
        &'e E: Executor<'e, Database = DB>;
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
