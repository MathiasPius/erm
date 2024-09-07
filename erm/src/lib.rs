pub mod archetype;
pub mod backend;
pub mod component;
mod create;
pub mod entity;
mod indent;
mod insert;
mod select;
pub mod types;

mod r#const;
pub mod cte;

use sqlx::{sqlite::SqliteRow, ColumnIndex, Decode, Row, Sqlite};

pub struct OffsetRow<'q, R> {
    pub row: &'q R,
    pub offset: usize,
}

impl<'q, R> OffsetRow<'q, R> {
    pub fn new(row: &'q R) -> Self {
        OffsetRow { row, offset: 0 }
    }

    pub fn skip(&mut self, offset: usize) {
        self.offset += offset;
    }
}

// impl<'q, R: Row> OffsetRow<'q, R>
// where
//     usize: ColumnIndex<R>,
// {
//     pub fn try_get<'a, T>(&'a self, index: usize) -> Result<T, sqlx::Error>
//     where
//         T: Decode<'a, <R as Row>::Database> + sqlx::Type<<R as Row>::Database>,
//     {
//         self.row.try_get(index + self.offset)
//     }
// }

impl<'q, R: Row> OffsetRow<'q, R> {
    pub fn try_get<'a, T>(&'a mut self) -> Result<T, sqlx::Error>
    where
        T: Decode<'a, <R as Row>::Database> + sqlx::Type<<R as Row>::Database>,
        usize: ColumnIndex<R>,
    {
        self.offset += 1;
        self.row.try_get(self.offset - 1)
    }
}

//pub use component::{Component, Field};
/*
#[cfg(test)]
mod tests {
    use sqlx::{
        any::AnyTypeInfo, query::Query, sqlite::SqliteRow, Database, Encode, Sqlite, SqlitePool,
        Type,
    };

    use crate::{
        archetype::Archetype,
        backend::{Deserialize, Serialize},
        component::{Component, Field},
        create::Create,
        insert::Insert,
        select::{Compound, ToSql as _},
        OffsetRow,
    };

    #[derive(Debug)]
    struct Position {
        pub x: f32,
        pub y: f32,
    }

    impl<'q, Entity> Serialize<'q, Sqlite, Entity> for Position
    where
        Entity: Encode<'q, Sqlite> + Type<Sqlite> + 'q,
    {
        fn serialize(
            &self,
            query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
            entity: &'q Entity,
        ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
            query.bind(entity).bind(self.x).bind(self.y)
        }
    }

    impl<'r> Deserialize<'r, SqliteRow> for Position {
        fn deserialize(row: &'r OffsetRow<SqliteRow>) -> Result<Self, sqlx::Error> {
            Ok(Position {
                x: row.try_get::<f64>(0)? as f32,
                y: row.try_get::<f64>(1)? as f32,
            })
        }
    }

    impl Component for Position {
        const TABLE_NAME: &'static str = "erm_position";

        const FIELDS: &'static [Field] = &[
            Field {
                name: "x",
                optional: false,
                type_info: AnyTypeInfo {
                    kind: sqlx::postgres::any::AnyTypeInfoKind::Double,
                },
            },
            Field {
                name: "y",
                optional: false,
                type_info: AnyTypeInfo {
                    kind: sqlx::postgres::any::AnyTypeInfoKind::Double,
                },
            },
        ];
    }

    #[derive(Debug)]
    struct Velocity {
        pub x: f32,
        pub y: f32,
    }

    impl<'q, Entity> Serialize<'q, Sqlite, Entity> for Velocity
    where
        Entity: Encode<'q, Sqlite> + Type<Sqlite> + 'q,
    {
        fn serialize(
            &self,
            query: Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>>,
            entity: &'q Entity,
        ) -> Query<'q, Sqlite, <Sqlite as Database>::Arguments<'q>> {
            query.bind(entity).bind(self.x).bind(self.y)
        }
    }

    impl<'r> Deserialize<'r, SqliteRow> for Velocity {
        fn deserialize(row: &'r OffsetRow<SqliteRow>) -> Result<Self, sqlx::Error> {
            Ok(Velocity {
                x: row.try_get::<f64>(0)? as f32,
                y: row.try_get::<f64>(1)? as f32,
            })
        }
    }

    impl Component for Velocity {
        const TABLE_NAME: &'static str = "erm_velocity";

        const FIELDS: &'static [Field] = &[
            Field {
                name: "x",
                optional: false,
                type_info: AnyTypeInfo {
                    kind: sqlx::postgres::any::AnyTypeInfoKind::Double,
                },
            },
            Field {
                name: "y",
                optional: false,
                type_info: AnyTypeInfo {
                    kind: sqlx::postgres::any::AnyTypeInfoKind::Double,
                },
            },
        ];
    }

    #[derive(Debug)]
    struct PhysicsObject {
        pub position: Position,
        pub velocity: Velocity,
    }

    impl Archetype for PhysicsObject {
        const COMPONENTS: &'static [crate::component::ComponentDesc] =
            &[Position::DESCRIPTION, Velocity::DESCRIPTION];
    }

    impl<'row> Deserialize<'row, SqliteRow> for PhysicsObject {
        fn deserialize(row: &'row OffsetRow<SqliteRow>) -> Result<Self, sqlx::Error> {
            Ok(PhysicsObject {
                position: Position::deserialize(&row.offset_by(0))?,
                velocity: Velocity::deserialize(&row.offset_by(2))?,
            })
        }
    }

    #[tokio::test]
    async fn dump_sql() {
        /*
        let options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(":memory:");

        let db = SqlitePool::connect_with(options).await.unwrap();
         */
        let db = SqlitePool::connect(":memory:").await.unwrap();

        // create component tables
        let position = Create::<sqlx::Postgres>::from(&Position::DESCRIPTION)
            .to_sql()
            .unwrap();

        println!("-- create position\n{position}\n");
        sqlx::query(&position).execute(&db).await.unwrap();

        let velocity = Create::<sqlx::Postgres>::from(&Velocity::DESCRIPTION)
            .to_sql()
            .unwrap();

        println!("-- create velocity\n{velocity}\n");
        sqlx::query(&velocity).execute(&db).await.unwrap();

        // Insert components
        let entity_id = "hello";

        let obj = PhysicsObject {
            position: Position { x: 1.0, y: 2.0 },
            velocity: Velocity { x: 3.0, y: 4.0 },
        };

        let position = Insert::<sqlx::Sqlite>::from(&Position::DESCRIPTION)
            .to_sql()
            .unwrap();

        println!("-- insert position\n{position}\n");
        let q = sqlx::query(&position);

        let q = obj.position.serialize(q, &entity_id);
        q.execute(&db).await.unwrap();

        let velocity = Insert::<sqlx::Sqlite>::from(&Velocity::DESCRIPTION)
            .to_sql()
            .unwrap();

        println!("-- insert velocity\n{velocity}\n");
        let q = sqlx::query(&velocity);
        let q = obj.velocity.serialize(q, &entity_id);
        q.execute(&db).await.unwrap();

        let select = Compound::from(&PhysicsObject::as_description())
            .to_sql()
            .unwrap();

        println!("-- select physicsobject\n{select}\n");

        let row = sqlx::query(&select).fetch_one(&db).await.unwrap();

        let offset = OffsetRow::new(&row);
        let out = PhysicsObject::deserialize(&offset).unwrap();

        println!("{out:#?}");

        //let row = AnyRow::map_from(&result, Arc::default()).unwrap();

        //let offset = OffsetRow::new(&row);
        //let entity = GenericEntity::<String>::deserialize(&offset).unwrap();
        //let out = PhysicsObject::deserialize(&offset.offset_by(1)).unwrap();

        //println!("{entity:?}: {out:#?}");
    }
}
 */
