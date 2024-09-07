use std::{pin::Pin, sync::OnceLock};

use futures::{Stream, StreamExt};
use sqlx::{query::Query, Database, Executor, FromRow, IntoArguments, Row, Sqlite};

use crate::{
    cte::{CommonTableExpression, InnerJoin, Select},
    offsets::OffsetRow,
};

// #[derive(Debug)]
// pub struct Position {
//     pub x: f32,
//     pub y: f32,
// }

// #[derive(Debug)]
// pub struct RealName {
//     pub real_name: String,
// }

/// FromRow-implementing wrapper around Components
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

/// Describes reading and writing from a Component-specific Table.
pub trait Component<DB: Database>: Sized {
    fn table() -> &'static str;
    fn columns() -> &'static [&'static str];
    fn deserialize_fields(row: &mut OffsetRow<<DB as Database>::Row>) -> Result<Self, sqlx::Error>;
    fn serialize_fields<'q>(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;
}

// impl Component<Sqlite> for Position {
//     fn table() -> &'static str {
//         "positions"
//     }

//     fn columns() -> &'static [&'static str] {
//         &["x", "y"]
//     }

//     fn deserialize_fields(row: &mut OffsetRow<SqliteRow>) -> Result<Self, sqlx::Error> {
//         let x = row.try_get()?;
//         let y = row.try_get()?;

//         Ok(Position { x, y })
//     }

//     fn serialize_fields<'q>(
//         &self,
//         query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
//     ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
//         query.bind(self.x).bind(self.y)
//     }
// }

// impl Component<Sqlite> for RealName {
//     fn table() -> &'static str {
//         "real_names"
//     }

//     fn columns() -> &'static [&'static str] {
//         &["real_name"]
//     }

//     fn deserialize_fields(row: &mut OffsetRow<SqliteRow>) -> Result<Self, sqlx::Error> {
//         let name: String = row.try_get()?;

//         Ok(RealName { real_name: name })
//     }

//     fn serialize_fields<'q>(
//         &'q self,
//         query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
//     ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
//         query.bind(&self.real_name)
//     }
// }

// #[derive(Debug)]
// pub struct Person {
//     position: Position,
//     name: RealName,
// }

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

// impl<Entity> Serializer<Entity, Sqlite> for Person
// where
//     Entity: for<'q> sqlx::Encode<'q, Sqlite> + sqlx::Type<Sqlite> + 'static,
// {
//     fn insert_statement() -> String {
//         vec![
//             <Position as Serializer<Entity, Sqlite>>::insert_statement(),
//             <RealName as Serializer<Entity, Sqlite>>::insert_statement(),
//         ]
//         .join(";\n")
//     }

//     fn serialize_components<'q>(
//         &'q self,
//         entity: &'q Entity,
//         query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
//     ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
//         let query = self.position.serialize_components(entity, query);
//         let query = self.name.serialize_components(entity, query);

//         query
//     }
// }

// impl Deserializer<Sqlite> for Person {
//     fn cte() -> impl CommonTableExpression {
//         <(Position, RealName) as Deserializer<Sqlite>>::cte()
//     }

//     fn deserialize_components(
//         row: &mut OffsetRow<<Sqlite as Database>::Row>,
//     ) -> Result<Self, sqlx::Error> {
//         let (position, name) =
//             <(Position, RealName) as Deserializer<Sqlite>>::deserialize_components(row)?;

//         Ok(Person { position, name })
//     }
// }

macro_rules! impl_compound_for_db{
    ($db:ident, $($list:ident),*) => {
        impl<Entity, $($list),*> Serializer<Entity, $db> for ($($list,)*)
        where
            $($list: Serializer<Entity, $db>,)*
            Entity: for<'q> sqlx::Encode<'q, $db> + sqlx::Type<$db> + 'static,
        {
            fn insert_statement() -> String {
                vec![
                    $(<$list as Serializer<Entity, $db>>::insert_statement()),*
                ]
                .join(";\n")
            }

            fn serialize_components<'q>(
                &'q self,
                entity: &'q Entity,
                query: Query<'q, $db, <$db as Database>::Arguments<'q>>,
            ) -> Query<'q, $db, <$db as Database>::Arguments<'q>> {
                $(
                    #[allow(unused)]
                    const $list: () = ();
                    let query = self.${index()}.serialize_components(entity, query);
                )*

                query
            }
        }

        impl<$($list),*> Deserializer<$db> for ($($list,)*)
        where
            $($list: Deserializer<$db>,)*
        {
            fn cte() -> impl CommonTableExpression {
                InnerJoin {
                    left: (
                        Box::new(<A as Deserializer<$db>>::cte()),
                        "entity".to_string(),
                    ),
                    right: (
                        Box::new(<B as Deserializer<$db>>::cte()),
                        "entity".to_string(),
                    ),
                }
            }

            fn deserialize_components(
                row: &mut OffsetRow<<$db as Database>::Row>,
            ) -> Result<Self, sqlx::Error> {
                Ok((
                    $(
                        <$list as Deserializer<$db>>::deserialize_components(row)?,
                    )*
                ))
            }
        }
    };
}

macro_rules! impl_compound {
    ($($list:ident),*) => {
        impl_compound_for_db!(Sqlite, $($list),*);
    };
}

impl_compound!(A, B);
impl_compound!(A, B, C);
impl_compound!(A, B, C, D);
impl_compound!(A, B, C, D, E);
impl_compound!(A, B, C, D, E, F);
impl_compound!(A, B, C, D, E, F, G);
impl_compound!(A, B, C, D, E, F, G, H);
impl_compound!(A, B, C, D, E, F, G, H, I);
impl_compound!(A, B, C, D, E, F, G, H, I, J);

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
