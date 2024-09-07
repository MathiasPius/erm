use std::{marker::PhantomData, pin::Pin, sync::OnceLock};

use futures::{Stream, StreamExt};
use sqlx::{
    query::{Query, QueryAs},
    sqlite::SqliteRow,
    Database, Executor, FromRow, IntoArguments, Row, Sqlite,
};

use crate::{
    archetype::Archetype,
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
pub struct Rowed<T>(pub T);

impl<'r, R: Row, T: Deserializer<<R as Row>::Database>> FromRow<'r, R> for Rowed<T> {
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let mut row = OffsetRow::new(row);
        Ok(Rowed(
            <T as Deserializer<<R as Row>::Database>>::deserialize_components(&mut row).unwrap(),
        ))
    }
}

impl<Entity, T> Serializer<Entity, Sqlite> for T
where
    T: Component<Sqlite>,
    Entity: for<'q> sqlx::Encode<'q, Sqlite> + sqlx::Type<Sqlite>,
{
    fn serialize_components<'q>(
        &'q self,
        entity: &'q Entity,
        query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
    ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
        let query = query.bind(entity);
        <Self as Component<Sqlite>>::serialize_fields(self, query)
    }
}

impl<T> Deserializer<Sqlite> for T
where
    T: Component<Sqlite>,
{
    fn cte() -> impl CommonTableExpression {
        Select {
            table: <T as Component<Sqlite>>::table().to_string(),
            columns: <T as Component<Sqlite>>::columns()
                .into_iter()
                .map(|column| column.to_string())
                .collect(),
        }
    }

    fn deserialize_components(row: &mut OffsetRow<SqliteRow>) -> Result<Self, sqlx::Error> {
        <Self as Component<Sqlite>>::deserialize_fields(row)
    }
}

impl<Entity> Serializer<Entity, Sqlite> for Person
where
    Entity: for<'q> sqlx::Encode<'q, Sqlite> + sqlx::Type<Sqlite> + 'static,
{
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

pub trait Fetch<'q, DB: Database>: Deserializer<DB> + Sized + Send + Sync {
    fn list() -> List<'q, Self, DB>;
}

pub struct List<'q, A: Deserializer<DB> + Send + Sync, DB: Database> {
    query: QueryAs<'q, DB, Rowed<A>, <DB as Database>::Arguments<'q>>,
}

impl<'q, A: Deserializer<DB> + Unpin + Send + Sync + 'static, DB: Database> List<'q, A, DB> {
    pub fn get<'e, E>(
        self,
        db: &'e E,
    ) -> Pin<Box<dyn Stream<Item = Result<Rowed<A>, sqlx::Error>> + Send + 'e>>
    where
        <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        &'e E: Executor<'e, Database = DB>,
        'q: 'e,
    {
        self.query.fetch(db)
    }
}

impl<'q, DB: Database, T: Deserializer<DB> + Send + Sync> Fetch<'q, DB> for T {
    fn list() -> List<'q, Self, DB> {
        let cte = <T as Deserializer<DB>>::cte();

        static SQL: OnceLock<String> = OnceLock::new();
        let sql = SQL.get_or_init(|| {
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

        List {
            query: sqlx::query_as(&sql),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use sqlx::{sqlite::SqliteConnectOptions, Executor as _, SqlitePool};

    use crate::{
        r#const::{Deserializer as _, Fetch as _, Person},
        OffsetRow,
    };

    #[tokio::test]
    async fn test_func() {
        let options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename("test.sqlite3");

        let db = SqlitePool::connect_with(options).await.unwrap();

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

        let query = Person::list();

        let mut results = query.get(&db);

        while let Some(result) = results.next().await {
            println!("{:#?}", result);
        }

        // for result in db.fetch_all(query).await.unwrap() {
        //     let mut offset = OffsetRow::new(&result);
        //     let person = Person::deserialize_components(&mut offset).unwrap();
        //     println!("{person:#?}");
        // }
    }
}
