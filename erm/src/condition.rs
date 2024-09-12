use sqlx::{query::QueryAs, Database};

pub trait Condition {
    fn serialize(&self) -> String;
    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        T: sqlx::Encode<'q, DB> + sqlx::Type<DB> + 'q,
        str: sqlx::Type<DB>,
        &'q str: sqlx::Encode<'q, DB>;
}

pub struct All;

impl Condition for All {
    fn serialize(&self) -> String {
        "1 == 1".to_string()
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        T: sqlx::Encode<'q, DB> + sqlx::Type<DB> + 'q,
    {
        query
    }
}

pub struct Equality(&'static str);

impl Condition for Equality {
    fn serialize(&self) -> String {
        format!("{} == ?", self.0)
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        T: sqlx::Encode<'q, DB> + sqlx::Type<DB> + 'q,
        str: sqlx::Type<DB>,
        &'q str: sqlx::Encode<'q, DB>,
    {
        query.bind(self.0)
    }
}

pub struct Inequality(&'static str);

impl Condition for Inequality {
    fn serialize(&self) -> String {
        format!("{} <> ?", self.0)
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        T: sqlx::Encode<'q, DB> + sqlx::Type<DB> + 'q,
        str: sqlx::Type<DB>,
        &'q str: sqlx::Encode<'q, DB>,
    {
        query.bind(self.0)
    }
}

pub struct And<A: Condition, B: Condition>(A, B);

impl<A: Condition, B: Condition> Condition for And<A, B> {
    fn serialize(&self) -> String {
        format!("({} and {})", self.0.serialize(), self.1.serialize())
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        T: sqlx::Encode<'q, DB> + sqlx::Type<DB> + 'q,
        str: sqlx::Type<DB>,
        &'q str: sqlx::Encode<'q, DB>,
    {
        let query = self.0.bind(query);
        let query = self.1.bind(query);

        query
    }
}

pub struct Or<A: Condition, B: Condition>(A, B);

impl<A: Condition, B: Condition> Condition for Or<A, B> {
    fn serialize(&self) -> String {
        format!("({} or {})", self.0.serialize(), self.1.serialize())
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        T: sqlx::Encode<'q, DB> + sqlx::Type<DB> + 'q,
        str: sqlx::Type<DB>,
        &'q str: sqlx::Encode<'q, DB>,
    {
        let query = self.0.bind(query);
        let query = self.1.bind(query);

        query
    }
}
