pub trait GenerateUnique {
    fn generate_unique() -> Self;
}

#[cfg(feature = "uuid")]
mod uuid {
    use sqlx::{ColumnIndex, Database, Decode, Encode, Row, Type};
    pub use uuid::Uuid;

    use crate::backend::{Deserialize, Serialize};

    use super::GenerateUnique;

    impl GenerateUnique for Uuid {
        fn generate_unique() -> Self {
            Uuid::new_v4()
        }
    }

    impl<'q, DB> Serialize<'q, DB> for Uuid
    where
        Uuid: Encode<'q, DB> + Type<DB> + Send + Clone + 'q,
        DB: Database,
    {
        fn serialize(
            &self,
            query: sqlx::query::Query<'q, DB, <DB as Database>::Arguments<'q>>,
        ) -> sqlx::query::Query<'q, DB, <DB as Database>::Arguments<'q>> {
            query.bind(self.clone())
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
