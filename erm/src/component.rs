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

impl<'r, R: Row, T: Archetype<<R as Row>::Database>> FromRow<'r, R> for Rowed<T> {
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let mut row = OffsetRow::new(row);
        Ok(Rowed(
            <T as Archetype<<R as Row>::Database>>::deserialize_components(&mut row).unwrap(),
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

pub trait Archetype<DB: Database>: Sized {
    fn insert_statement() -> String;

    fn serialize_components<'q>(
        &'q self,
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>;

    fn select_statement() -> impl CommonTableExpression;

    fn deserialize_components(
        row: &mut OffsetRow<<DB as Database>::Row>,
    ) -> Result<Self, sqlx::Error>;

    fn get<E, Entity>(
        executor: E,
        entity: Entity,
    ) -> impl std::future::Future<Output = Result<Self, sqlx::Error>> + Send
    where
        Entity: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB> + Send + Sync + 'static,
        for<'q> <DB as sqlx::Database>::Arguments<'q>: IntoArguments<'q, DB>,
        E: for<'e> Executor<'e, Database = DB> + Send + Sync,
        Self: Unpin + Send + Sync,
    {
        async move {
            static SQL: OnceLock<String> = OnceLock::new();
            let sql = SQL.get_or_init(|| {
                let cte = <Self as Archetype<DB>>::select_statement();

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
}

impl<T, DB: Database> Archetype<DB> for T
where
    T: Component<DB>,
{
    fn insert_statement() -> String {
        let table = <T as Component<DB>>::table();
        let columns = <T as Component<DB>>::columns();

        format!(
            "insert into {}(entity, {}) values(?1, {})",
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
        query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) -> Query<'q, DB, <DB as Database>::Arguments<'q>> {
        <Self as Component<DB>>::serialize_fields(self, query)
    }

    fn select_statement() -> impl CommonTableExpression {
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

macro_rules! impl_compound_for_db{
    ($db:ident, $($list:ident),*) => {
        impl<$($list),*> Archetype<$db> for ($($list,)*)
        where
            $($list: Archetype<$db>,)*
        {
            fn insert_statement() -> String {
                vec![
                    $(<$list as Archetype<$db>>::insert_statement()),*
                ]
                .join(";\n")
            }

            fn serialize_components<'q>(
                &'q self,
                query: Query<'q, $db, <$db as Database>::Arguments<'q>>,
            ) -> Query<'q, $db, <$db as Database>::Arguments<'q>> {
                $(
                    #[allow(unused)]
                    const $list: () = ();
                    let query = self.${index()}.serialize_components(query);
                )*

                query
            }

            fn select_statement() -> impl CommonTableExpression {
                InnerJoin {
                    left: (
                        Box::new(<A as Archetype<$db>>::select_statement()),
                        "entity".to_string(),
                    ),
                    right: (
                        Box::new(<B as Archetype<$db>>::select_statement()),
                        "entity".to_string(),
                    ),
                }
            }

            fn deserialize_components(
                row: &mut OffsetRow<<$db as Database>::Row>,
            ) -> Result<Self, sqlx::Error> {
                Ok((
                    $(
                        <$list as Archetype<$db>>::deserialize_components(row)?,
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

pub trait List<'q, DB: Database>: Archetype<DB> + Sized + Unpin + Send + Sync + 'static {
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
            let cte = <Self as Archetype<DB>>::select_statement();

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
    T: Archetype<DB> + Sized + Unpin + Send + Sync + 'static,
{
}

pub trait Insert<Entity, DB: Database>:
    Archetype<DB> + Sized + Unpin + Send + Sync + 'static
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

        let sql = SQL.get_or_init(|| <Self as Archetype<DB>>::insert_statement());

        let query = sqlx::query(sql).bind(entity);
        let query = self.serialize_components(query);

        executor.execute(query).await
    }
}

impl<Entity, DB, T> Insert<Entity, DB> for T
where
    DB: Database,
    T: Archetype<DB> + Sized + Unpin + Send + Sync + 'static,
    Entity: for<'q> sqlx::Encode<'q, DB> + sqlx::Type<DB>,
{
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
