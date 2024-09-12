use sqlx::{prelude::FromRow, ColumnIndex, Decode, Row};
use tracing::trace;

use crate::Archetype;

pub struct OffsetRow<'r, R: Row> {
    pub row: &'r R,
    pub offset: usize,
}

impl<'r, R: Row> OffsetRow<'r, R> {
    pub fn new(row: &'r R) -> Self {
        OffsetRow { row, offset: 0 }
    }

    pub fn skip(&mut self, offset: usize) {
        self.offset += offset;
    }
}

impl<'r, R: Row> OffsetRow<'r, R> {
    pub fn try_get<'a, T>(&'a mut self) -> Result<T, sqlx::Error>
    where
        T: Decode<'a, <R as Row>::Database> + sqlx::Type<<R as Row>::Database>,
        usize: ColumnIndex<R>,
    {
        trace!(
            "reading row {} from {:?} as {:?}",
            self.offset,
            self.row.try_column(self.offset),
            std::any::type_name::<T>()
        );

        let result = self.row.try_get::<'a, T, usize>(self.offset);
        self.offset += 1;
        result
    }
}

/// FromRow-implementing wrapper around Components
#[derive(Debug)]
pub struct Rowed<Entity, T> {
    pub entity: Entity,
    pub inner: T,
}

impl<'r, R, Entity, T> FromRow<'r, R> for Rowed<Entity, T>
where
    R: Row,
    Entity: for<'e> sqlx::Decode<'e, <R as sqlx::Row>::Database>
        + sqlx::Type<<R as sqlx::Row>::Database>,
    T: Archetype<<R as Row>::Database>,
    usize: ColumnIndex<R>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        trace!("parsing row with columns {:?}", row.columns());
        let mut row = OffsetRow::new(row);
        let entity = row.try_get::<Entity>()?;

        Ok(Rowed {
            entity,
            inner: <T as Archetype<<R as Row>::Database>>::deserialize_components(&mut row)?,
        })
    }
}
