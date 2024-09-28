use sqlx::{prelude::FromRow, ColumnIndex, Decode, Row, ValueRef};

use crate::serialization::Deserializeable;

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

    pub fn is_null(&self) -> bool
    where
        usize: ColumnIndex<R>,
    {
        self.row
            .try_get_raw(self.offset)
            .is_ok_and(|field| field.is_null())
    }
}

impl<'r, R: Row> OffsetRow<'r, R> {
    pub fn try_get<'a, T>(&'a mut self) -> Result<T, sqlx::Error>
    where
        T: Decode<'a, <R as Row>::Database> + sqlx::Type<<R as Row>::Database>,
        usize: ColumnIndex<R>,
    {
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
    T: Deserializeable<<R as Row>::Database>,
    usize: ColumnIndex<R>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let mut row = OffsetRow::new(row);
        let entity = row.try_get::<Entity>()?;

        Ok(Rowed {
            entity,
            inner: <T as Deserializeable<<R as Row>::Database>>::deserialize(&mut row)?,
        })
    }
}
