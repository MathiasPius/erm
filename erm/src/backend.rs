use sqlx::{database::HasArguments, query::Query, Database, Executor};

use crate::{archetype::Archetype, OffsetRow};

pub trait Serialize<'q, DB: Database> {
    fn serialize(
        &'q self,
        query: Query<'q, DB, <DB as HasArguments<'q>>::Arguments>,
    ) -> Query<'q, DB, <DB as HasArguments<'q>>::Arguments>;
}

pub trait Deserialize<'r>: Sized {
    fn deserialize(row: &'r OffsetRow) -> Result<Self, sqlx::Error>;
}

pub trait Backend {
    fn list<'r, A: Archetype + Deserialize<'r>>(&self);
}

impl<'c, E: Executor<'c>> Backend for E {
    fn list<'r, A: Archetype + Deserialize<'r>>(&self) {}
}
