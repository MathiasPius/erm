use std::{pin::Pin, sync::OnceLock};

use futures::{Stream, StreamExt};
use sqlx::{
    query::Query, sqlite::SqliteRow, Database, Executor, FromRow, IntoArguments, Row, Sqlite,
};

use crate::{
    cte::{CommonTableExpression, InnerJoin, Select},
    OffsetRow,
};

#[derive(Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub struct RealName {
    pub real_name: String,
}

pub trait Component<DB: Database>: Sized {
    fn table() -> &'static str;
    fn columns() -> &'static [&'static str];
    fn deserialize_fields(row: &mut OffsetRow<<DB as Database>::Row>) -> Result<Self, sqlx::Error>;
    fn serialize_fields<'q>(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;
}

impl Component<Sqlite> for Position {
    fn table() -> &'static str {
        "positions"
    }

    fn columns() -> &'static [&'static str] {
        &["x", "y"]
    }

    fn deserialize_fields(row: &mut OffsetRow<SqliteRow>) -> Result<Self, sqlx::Error> {
        let x = row.try_get()?;
        let y = row.try_get()?;

        Ok(Position { x, y })
    }

    fn serialize_fields<'q>(
        &self,
        query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
    ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
        query.bind(self.x).bind(self.y)
    }
}

impl Component<Sqlite> for RealName {
    fn table() -> &'static str {
        "real_names"
    }

    fn columns() -> &'static [&'static str] {
        &["real_name"]
    }

    fn deserialize_fields(row: &mut OffsetRow<SqliteRow>) -> Result<Self, sqlx::Error> {
        let name: String = row.try_get()?;

        Ok(RealName { real_name: name })
    }

    fn serialize_fields<'q>(
        &'q self,
        query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
    ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
        query.bind(&self.real_name)
    }
}

#[derive(Debug)]
pub struct Person {
    position: Position,
    name: RealName,
}

pub trait Serializer<Entity, DB: Database>: Sized
where
    Entity: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    fn insert_statement() -> String;

    fn serialize_components<'q>(
        &'q self,
        entity: &'q Entity,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;
}

pub trait Deserializer<DB: Database>: Sized {
    fn cte() -> impl CommonTableExpression;

    fn deserialize_components(
        row: &mut OffsetRow<<DB as Database>::Row>,
    ) -> Result<Self, sqlx::Error>;
}

#[derive(Debug)]
struct Rowed<T>(pub T);

impl<'r, R: Row, T: Deserializer<<R as Row>::Database>> FromRow<'r, R> for Rowed<T> {
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let mut row = OffsetRow::new(row);
        Ok(Rowed(
            <T as Deserializer<<R as Row>::Database>>::deserialize_components(&mut row).unwrap(),
        ))
    }
}

impl<Entity, T, DB: Database> Serializer<Entity, DB> for T
where
    T: Component<DB>,
    Entity: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    fn insert_statement() -> String {
        let table = <T as Component<DB>>::table();
        let columns = <T as Component<DB>>::columns();

        format!(
            "insert into {}(entity, {}) values(?, {})",
            table,
            columns.join(", "),
            std::iter::repeat("?")
                .take(columns.len())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn serialize_components<'q>(
        &'q self,
        entity: &'q Entity,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>> {
        let query = query.bind(entity);
        <Self as Component<DB>>::serialize_fields(self, query)
    }
}

impl<T, DB: Database> Deserializer<DB> for T
where
    T: Component<DB>,
{
    fn cte() -> impl CommonTableExpression {
        Select {
            table: <T as Component<DB>>::table().to_string(),
            columns: <T as Component<DB>>::columns()
                .into_iter()
                .map(|column| column.to_string())
                .collect(),
        }
    }

    fn deserialize_components(
        row: &mut OffsetRow<<DB as Database>::Row>,
    ) -> Result<Self, sqlx::Error> {
        <Self as Component<DB>>::deserialize_fields(row)
    }
}

impl<Entity> Serializer<Entity, Sqlite> for Person
where
    Entity: for<'q> sqlx::Encode<'q, Sqlite> + sqlx::Type<Sqlite> + 'static,
{
    fn insert_statement() -> String {
        vec![
            <Position as Serializer<Entity, Sqlite>>::insert_statement(),
            <RealName as Serializer<Entity, Sqlite>>::insert_statement(),
        ]
        .join(";\n")
    }

    fn serialize_components<'q>(
        &'q self,
        entity: &'q Entity,
        query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
    ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
        let query = self.position.serialize_components(entity, query);
        let query = self.name.serialize_components(entity, query);

        query
    }
}

impl Deserializer<Sqlite> for Person {
    fn cte() -> impl CommonTableExpression {
        <(Position, RealName) as Deserializer<Sqlite>>::cte()
    }

    fn deserialize_components(
        row: &mut OffsetRow<<Sqlite as Database>::Row>,
    ) -> Result<Self, sqlx::Error> {
        let (position, name) =
            <(Position, RealName) as Deserializer<Sqlite>>::deserialize_components(row)?;

        Ok(Person { position, name })
    }
}

impl<Entity, A, B> Serializer<Entity, Sqlite> for (A, B)
where
    A: Serializer<Entity, Sqlite>,
    B: Serializer<Entity, Sqlite>,
    Entity: for<'q> sqlx::Encode<'q, Sqlite> + sqlx::Type<Sqlite> + 'static,
{
    fn insert_statement() -> String {
        vec![
            <A as Serializer<Entity, Sqlite>>::insert_statement(),
            <B as Serializer<Entity, Sqlite>>::insert_statement(),
        ]
        .join(";\n")
    }

    fn serialize_components<'q>(
        &'q self,
        entity: &'q Entity,
        query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
    ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
        let query = self.0.serialize_components(entity, query);
        let query = self.1.serialize_components(entity, query);

        query
    }
}

impl<A, B> Deserializer<Sqlite> for (A, B)
where
    A: Deserializer<Sqlite>,
    B: Deserializer<Sqlite>,
{
    fn cte() -> impl CommonTableExpression {
        InnerJoin {
            left: (
                Box::new(<A as Deserializer<Sqlite>>::cte()),
                "entity".to_string(),
            ),
            right: (
                Box::new(<B as Deserializer<Sqlite>>::cte()),
                "entity".to_string(),
            ),
        }
    }

    fn deserialize_components(
        row: &mut OffsetRow<<Sqlite as Database>::Row>,
    ) -> Result<Self, sqlx::Error> {
        let a = <A as Deserializer<Sqlite>>::deserialize_components(row)?;
        let b = <B as Deserializer<Sqlite>>::deserialize_components(row)?;

        Ok((a, b))
    }
}

pub trait List<'q, DB: Database>: Deserializer<DB> + Sized + Unpin + Send + Sync + 'static {
    fn list<'e, E>(
        executor: &'e E,
    ) -> Pin<Box<dyn Stream<Item = Result<Self, sqlx::Error>> + Send + 'e>>
    where
        <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        &'e E: Executor<'e, Database = DB>,
        'q: 'e,
    {
        static SQL: OnceLock<String> = OnceLock::new();
        let sql = SQL.get_or_init(|| {
            let cte = <Self as Deserializer<DB>>::cte();

            format!(
                "{}\nselect {} from {}",
                cte.finalize(),
                cte.columns()
                    .iter()
                    .map(|(_, column)| format!("{}.{column}", cte.name()))
                    .collect::<Vec<_>>()
                    .join(", "),
                cte.name()
            )
        });

        println!("{sql}");
        let query = sqlx::query_as(&sql);

        Box::pin(
            query
                .fetch(executor)
                .map(|row| row.map(|result: Rowed<Self>| result.0)),
        )
    }
}

impl<'q, DB, T> List<'q, DB> for T
where
    DB: Database,
    T: Deserializer<DB> + Sized + Unpin + Send + Sync + 'static,
{
}

pub trait Get<'q, DB: Database>: Deserializer<DB> + Sized + Unpin + Send + Sync + 'static {
    async fn get<'e, E, Entity>(executor: &'e E, entity: Entity) -> Result<Self, sqlx::Error>
    where
        Entity: sqlx::Encode<'q, DB> + sqlx::Type<DB> + 'q,
        <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        &'e E: Executor<'e, Database = DB>,
        'q: 'e,
    {
        static SQL: OnceLock<String> = OnceLock::new();
        let sql = SQL.get_or_init(|| {
            let cte = <Self as Deserializer<DB>>::cte();

            format!(
                "{}\nselect {} from {} where entity = ?",
                cte.finalize(),
                cte.columns()
                    .iter()
                    .map(|(_, column)| format!("{}.{column}", cte.name()))
                    .collect::<Vec<_>>()
                    .join(", "),
                cte.name()
            )
        });

        println!("{sql}");
        let result: Rowed<Self> = sqlx::query_as(&sql)
            .bind(entity)
            .fetch_one(executor)
            .await?;

        Ok(result.0)
    }
}

impl<'q, DB, T> Get<'q, DB> for T
where
    DB: Database,
    T: Deserializer<DB> + Sized + Unpin + Send + Sync + 'static,
{
}

pub trait Insert<Entity, DB: Database>:
    Serializer<Entity, DB> + Sized + Unpin + Send + Sync + 'static
where
    Entity: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    async fn insert<'q, 'e, E>(
        &'q self,
        executor: &'e E,
        entity: &'q Entity,
    ) -> Result<<DB as Database>::QueryResult, sqlx::Error>
    where
        <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        &'e E: Executor<'e, Database = DB>,
        'q: 'e,
    {
        static SQL: OnceLock<String> = OnceLock::new();

        let sql = SQL.get_or_init(|| <Self as Serializer<Entity, DB>>::insert_statement());

        let query = sqlx::query(sql);
        let query = self.serialize_components(&entity, query);

        executor.execute(query).await
    }
}

impl<Entity, DB, T> Insert<Entity, DB> for T
where
    DB: Database,
    T: Serializer<Entity, DB> + Sized + Unpin + Send + Sync + 'static,
    Entity: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
}

#[cfg(test)]
mod tests {
    use futures::StreamExt as _;
    use sqlx::{
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
        Executor as _, Sqlite,
    };

    use crate::r#const::{
        Get as _, Insert as _, List as _, Person, Position, RealName, Serializer,
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

        let mut results = Person::list(&db);

        while let Some(Ok(result)) = results.next().await {
            println!("{:#?}", result);
        }

        let a = Person::get(&db, &"a").await.unwrap();

        println!("{a:?}");

        // for result in db.fetch_all(query).await.unwrap() {
        //     let mut offset = OffsetRow::new(&result);
        //     let person = Person::deserialize_components(&mut offset).unwrap();
        //     println!("{person:#?}");
        // }

        let c = Person {
            position: Position { x: 111.0, y: 222.0 },
            name: RealName {
                real_name: "third".to_string(),
            },
        };

        c.insert(&db, &"c".to_string()).await.unwrap();

        let mut results = Person::list(&db);

        while let Some(Ok(result)) = results.next().await {
            println!("NEW LINES:\n{:?}", result);
        }
    }

    #[test]
    fn inserts() {
        let insert = <Person as Serializer<String, Sqlite>>::insert_statement();
        println!("{insert}");
    }
}
