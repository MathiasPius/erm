use sqlx::{query::Query, sqlite::SqliteRow, Database, Sqlite};

use crate::{
    cte::{CommonTableExpression, InnerJoin, Select},
    OffsetRow,
};

pub struct Position {
    pub x: i64,
    pub y: i64,
}

pub struct Name {
    pub name: String,
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
        "position"
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

impl Component<Sqlite> for Name {
    fn table() -> &'static str {
        "name"
    }

    fn columns() -> &'static [&'static str] {
        &["name"]
    }

    fn deserialize_fields(row: &mut OffsetRow<SqliteRow>) -> Result<Self, sqlx::Error> {
        let name: String = row.try_get()?;

        Ok(Name { name })
    }

    fn serialize_fields<'q>(
        &'q self,
        query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
    ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
        query.bind(&self.name)
    }
}

pub struct Person {
    position: Position,
    name: Name,
}

pub trait Archetype<Entity, DB: Database>: Sized
where
    Entity: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
    fn into_common_table_expression() -> impl CommonTableExpression;

    fn deserialize_components(
        row: &mut OffsetRow<<DB as Database>::Row>,
    ) -> Result<Self, sqlx::Error>;
    fn serialize_components<'q>(
        &'q self,
        entity: &'q Entity,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;
}

impl<Entity, T> Archetype<Entity, Sqlite> for T
where
    T: Component<Sqlite>,
    Entity: for<'q> sqlx::Encode<'q, Sqlite> + sqlx::Type<Sqlite>,
{
    fn into_common_table_expression() -> impl CommonTableExpression {
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

    fn serialize_components<'q>(
        &'q self,
        entity: &'q Entity,
        query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
    ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
        let query = query.bind(entity);
        <Self as Component<Sqlite>>::serialize_fields(self, query)
    }
}

impl<Entity> Archetype<Entity, Sqlite> for Person
where
    Entity: for<'q> sqlx::Encode<'q, Sqlite> + sqlx::Type<Sqlite> + 'static,
{
    fn into_common_table_expression() -> impl CommonTableExpression {
        InnerJoin {
            left: (
                Box::new(<Position as Archetype<Entity, Sqlite>>::into_common_table_expression()),
                "entity".to_string(),
            ),
            right: (
                Box::new(<Name as Archetype<Entity, Sqlite>>::into_common_table_expression()),
                "entity".to_string(),
            ),
        }
    }

    fn deserialize_components(
        row: &mut OffsetRow<<Sqlite as Database>::Row>,
    ) -> Result<Self, sqlx::Error> {
        let position = <Position as Archetype<Entity, Sqlite>>::deserialize_components(row)?;
        let name = <Name as Archetype<Entity, Sqlite>>::deserialize_components(row)?;

        Ok(Person { position, name })
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

impl<Entity, A, B> Archetype<Entity, Sqlite> for (A, B)
where
    A: Archetype<Entity, Sqlite>,
    B: Archetype<Entity, Sqlite>,
    Entity: for<'q> sqlx::Encode<'q, Sqlite> + sqlx::Type<Sqlite> + 'static,
{
    fn into_common_table_expression() -> impl CommonTableExpression {
        InnerJoin {
            left: (
                Box::new(<A as Archetype<Entity, Sqlite>>::into_common_table_expression()),
                "entity".to_string(),
            ),
            right: (
                Box::new(<B as Archetype<Entity, Sqlite>>::into_common_table_expression()),
                "entity".to_string(),
            ),
        }
    }

    fn deserialize_components(
        row: &mut OffsetRow<<Sqlite as Database>::Row>,
    ) -> Result<Self, sqlx::Error> {
        let a = <A as Archetype<Entity, Sqlite>>::deserialize_components(row)?;
        let b = <B as Archetype<Entity, Sqlite>>::deserialize_components(row)?;

        Ok((a, b))
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

#[test]
fn test_func() {
    let cte = <Person as Archetype<String, Sqlite>>::into_common_table_expression().finalize();

    println!("{cte}");
}
