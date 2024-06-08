use sqlx::{database::HasArguments, query::Query, Database, Executor};

use crate::{archetype::Archetype, OffsetRow};

pub trait Serialize<'q, DB: Database> {
    fn serialize(
        &self,
        query: Query<'q, DB, <DB as HasArguments<'q>>::Arguments>,
    ) -> Query<'q, DB, <DB as HasArguments<'q>>::Arguments>;
}

pub trait Deserialize: Sized {
    fn deserialize(row: &OffsetRow) -> Result<Self, sqlx::Error>;
}

pub trait Backend {
    fn list<A: Archetype + Deserialize>(&self);
}

impl<'c, E: Executor<'c>> Backend for E {
    fn list<A: Archetype + Deserialize>(&self) {}
}
