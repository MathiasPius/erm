use sqlx::{query::Query, Database};

pub struct EntityPrefixedQuery<'q, DB, Entity>
where
    DB: Database,
{
    pub queries: Vec<Query<'q, DB, <DB as Database>::Arguments<'q>>>,
    pub entity: Entity,
}

impl<'query, DB, Entity> EntityPrefixedQuery<'query, DB, Entity>
where
    DB: Database,
    Entity: sqlx::Encode<'query, DB> + sqlx::Type<DB> + Clone + 'query,
{
    pub fn new(entity: Entity) -> Self {
        EntityPrefixedQuery {
            queries: Vec::new(),
            entity,
        }
    }

    pub fn query(
        &mut self,
        sql: &'static str,
        f: impl Fn(
            Query<'query, DB, <DB as Database>::Arguments<'query>>,
        ) -> Query<'query, DB, <DB as Database>::Arguments<'query>>,
    ) {
        let query = sqlx::query(sql).bind(self.entity.clone());

        self.queries.push(f(query));
    }
}

#[cfg(test)]
mod tests {
    use sqlx::Sqlite;

    use super::EntityPrefixedQuery;

    #[test]
    fn test_db() {
        let mut insert = EntityPrefixedQuery::<'_, Sqlite, _> {
            queries: vec![],
            entity: 12345,
        };

        insert.query("a", |query| query.bind(1));
        insert.query("a", |query| query.bind(1));
    }
}
