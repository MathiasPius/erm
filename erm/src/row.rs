use std::ops::{Deref, DerefMut};

use sqlx::{prelude::FromRow, ColumnIndex, Decode, Row, ValueRef};

use crate::serialization::Deserializeable;

/// Wrapper around a database-specific row which tracks the offset
/// into the row as columns are parsed.
pub struct OffsetRow<'r, R: Row> {
    pub row: &'r R,
    pub offset: usize,
}

impl<'r, R: Row> OffsetRow<'r, R> {
    /// Construct a new OffsetRow from an existing row.
    pub fn new(row: &'r R) -> Self {
        OffsetRow { row, offset: 0 }
    }

    /// Skip `count` columns.
    pub fn skip(&mut self, count: usize) {
        self.offset += count;
    }

    /// Detemines if the column indicated by `self.offset` is null.
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
    /// Attempt to read the decode the current column as `T`.
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

/// Returned row of an Entity ID and associated components from a query.
#[derive(Debug)]
pub struct Entity<EntityId, T> {
    id: EntityId,
    inner: T,
}

impl<EntityId, T> Entity<EntityId, T> {
    /// Borrow the EntityID of the returned entity.
    pub fn id(&self) -> &EntityId {
        &self.id
    }

    /// Consumes the Entity, returning its ID
    pub fn into_id(self) -> EntityId {
        self.id
    }

    /// Borrow the entity's components.
    pub fn components(&self) -> &T {
        &self.inner
    }

    /// Consumes the Entity, returning its components.
    pub fn into_components(self) -> T {
        self.inner
    }
}

impl<EntityId, T> AsRef<T> for Entity<EntityId, T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<EntityId, T> Deref for Entity<EntityId, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<EntityId, T> DerefMut for Entity<EntityId, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'r, R, EntityId, T> FromRow<'r, R> for Entity<EntityId, T>
where
    R: Row,
    EntityId: for<'e> sqlx::Decode<'e, <R as sqlx::Row>::Database>
        + sqlx::Type<<R as sqlx::Row>::Database>,
    T: Deserializeable<<R as Row>::Database>,
    usize: ColumnIndex<R>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let mut row = OffsetRow::new(row);
        let id = row.try_get::<EntityId>()?;

        Ok(Entity {
            id,
            inner: <T as Deserializeable<<R as Row>::Database>>::deserialize(&mut row)?,
        })
    }
}
