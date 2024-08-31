pub trait GenerateUnique {
    fn generate_unique() -> Self;
}

#[cfg(feature = "uuid")]
mod uuid {
    use sqlx::{ColumnIndex, Decode, Row, Type};
    pub use uuid::Uuid;

    use crate::backend::Deserialize;

    use super::GenerateUnique;

    impl GenerateUnique for Uuid {
        fn generate_unique() -> Self {
            Uuid::new_v4()
        }
    }

    impl<'r, R: Row> Deserialize<'r, R> for Uuid
    where
        Uuid: Decode<'r, <R as Row>::Database> + Type<<R as Row>::Database> + Clone,
        usize: ColumnIndex<R>,
    {
        fn deserialize(row: &'r crate::OffsetRow<R>) -> Result<Self, sqlx::Error> {
            let id: Uuid = row.try_get(0)?;
            Ok(id)
        }
    }
}

#[cfg(feature = "uuid")]
pub use uuid::Uuid;
