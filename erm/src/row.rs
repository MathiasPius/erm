use sqlx::{prelude::FromRow, ColumnIndex, Decode, Row};

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
        self.offset += 1;
        self.row.try_get(self.offset - 1)
    }
}

/// FromRow-implementing wrapper around Components
#[derive(Debug)]
pub(crate) struct Rowed<T>(pub T);

impl<'r, R: Row, T: Archetype<<R as Row>::Database>> FromRow<'r, R> for Rowed<T> {
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let mut row = OffsetRow::new(row);
        Ok(Rowed(
            <T as Archetype<<R as Row>::Database>>::deserialize_components(&mut row).unwrap(),
        ))
    }
}
