use sqlx::{query::Query, Database};

pub struct InsertionQuery<'q, DB, Entity>
where
    DB: Database,
{
    pub queries: Vec<Query<'q, DB, <DB as Database>::Arguments<'q>>>,
    pub entity: Entity,
}

impl<'q, DB, Entity> InsertionQuery<'q, DB, Entity>
where
    DB: Database,
    Entity: sqlx::Encode<'q, DB> + sqlx::Type<DB> + Clone + 'q,
{
    pub fn query(
        &mut self,
        sql: &'static str,
        f: impl Fn(
            Query<'q, DB, <DB as Database>::Arguments<'q>>,
        ) -> Query<'q, DB, <DB as Database>::Arguments<'q>>,
    ) {
        let query = sqlx::query(sql).bind(self.entity.clone());

        self.queries.push(f(query));
    }
}

#[cfg(test)]
mod tests {
    use sqlx::Sqlite;

    use super::InsertionQuery;

    #[test]
    fn test_db() {
        let mut insert = InsertionQuery::<'_, Sqlite, _> {
            queries: vec![],
            entity: 12345,
        };

        insert.query("a", |query| query.bind(1));
        insert.query("a", |query| query.bind(1));
    }
}
