#![feature(const_trait_impl)]
#![feature(effects)]

pub mod archetype;
pub mod backend;
pub mod component;
mod create;
mod indent;
pub mod insert;
pub mod select;

use sqlx::{any::AnyRow, ColumnIndex, Decode, Row};

pub struct OffsetRow<'q> {
    pub row: &'q AnyRow,
    pub offset: usize,
}

impl<'q> OffsetRow<'q>
where
    usize: ColumnIndex<AnyRow>,
{
    pub fn offset_by(&self, offset: usize) -> OffsetRow {
        OffsetRow {
            row: self.row,
            offset: self.offset + offset,
        }
    }

    pub fn try_get<'a, T>(&'a self, index: usize) -> Result<T, sqlx::Error>
    where
        T: Decode<'a, sqlx::Any> + sqlx::Type<sqlx::Any>,
    {
        self.row.try_get(index + self.offset)
    }
}

#[cfg(test)]
mod tests {

    use sqlx::{any::AnyTypeInfo, database::HasArguments, query::Query, Database, Encode, Type};

    use crate::{
        archetype::Archetype,
        backend::{Deserialize, Serialize},
        component::{Component, Field},
        create::Create,
        insert::Insert,
        select::{Compound, ToSql as _},
        OffsetRow,
    };

    struct Position {
        pub x: f32,
        pub y: f32,
    }

    impl<'q, DB> Serialize<'q, DB> for Position
    where
        f32: Encode<'q, DB> + Type<DB>,
        DB: Database,
    {
        fn serialize(
            &self,
            query: Query<'q, DB, <DB as HasArguments<'q>>::Arguments>,
        ) -> Query<'q, DB, <DB as HasArguments<'q>>::Arguments> {
            query.bind(self.x).bind(self.y)
        }
    }

    impl Deserialize for Position {
        fn deserialize(row: &OffsetRow) -> Result<Self, sqlx::Error> {
            Ok(Position {
                x: row.try_get(0)?,
                y: row.try_get(1)?,
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

    struct Velocity {
        pub x: f32,
        pub y: f32,
    }

    impl<'q, DB> Serialize<'q, DB> for Velocity
    where
        f32: Encode<'q, DB> + Type<DB>,
        DB: Database,
    {
        fn serialize(
            &self,
            query: Query<'q, DB, <DB as HasArguments<'q>>::Arguments>,
        ) -> Query<'q, DB, <DB as HasArguments<'q>>::Arguments> {
            query.bind(self.x).bind(self.y)
        }
    }

    impl Deserialize for Velocity {
        fn deserialize(row: &OffsetRow) -> Result<Self, sqlx::Error> {
            Ok(Velocity {
                x: row.try_get(0)?,
                y: row.try_get(1)?,
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

    struct PhysicsObject {
        pub position: Position,
        pub velocity: Velocity,
    }

    impl Archetype for PhysicsObject {
        const COMPONENTS: &'static [crate::component::ComponentDesc] =
            &[Position::DESCRIPTION, Velocity::DESCRIPTION];
    }

    impl Deserialize for PhysicsObject {
        fn deserialize(row: &OffsetRow) -> Result<Self, sqlx::Error> {
            Ok(PhysicsObject {
                position: Position::deserialize(&row.offset_by(0))?,
                velocity: Velocity::deserialize(&row.offset_by(2))?,
            })
        }
    }

    #[test]
    fn dump_sql() {
        let select = Compound::from(&PhysicsObject::as_description())
            .to_sql()
            .unwrap();

        println!("-- select\n{select}\n");

        let insert = Insert::<sqlx::Postgres>::from(&Position::DESCRIPTION)
            .to_sql()
            .unwrap();

        println!("-- insert\n{insert}\n");

        let create = Create::<sqlx::Postgres>::from(&Position::DESCRIPTION)
            .to_sql()
            .unwrap();

        println!("-- create\n{create}\n");
    }
}
