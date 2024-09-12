use std::marker::PhantomData;

use sqlx::{query::QueryAs, Database};

pub trait Condition<Entity>: Sized {
    fn serialize(&self) -> String;
    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        Entity: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q;

    fn and<B: Condition<Entity>>(self, other: B) -> And<Entity, Self, B> {
        And::new(self, other)
    }

    fn or<B: Condition<Entity>>(self, other: B) -> Or<Entity, Self, B> {
        Or::new(self, other)
    }
}

pub struct All;

impl<Entity> Condition<Entity> for All {
    fn serialize(&self) -> String {
        "1 == 1".to_string()
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        Entity: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
    {
        query
    }
}

pub struct Equality<Entity> {
    column: &'static str,
    entity: Entity,
}

impl<Entity> Equality<Entity> {
    pub fn new(column: &'static str, value: Entity) -> Self {
        Self {
            column,
            entity: value,
        }
    }
}

impl<Entity> Condition<Entity> for Equality<Entity> {
    fn serialize(&self) -> String {
        format!("{} == ?", self.column)
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        Entity: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
    {
        query.bind(self.entity)
    }
}

pub struct Inequality<Entity> {
    column: &'static str,
    entity: Entity,
}

impl<Entity> Inequality<Entity> {
    pub fn new(column: &'static str, value: Entity) -> Self {
        Self {
            column,
            entity: value,
        }
    }
}

impl<Entity> Condition<Entity> for Inequality<Entity> {
    fn serialize(&self) -> String {
        format!("{} <> ?", self.column)
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        Entity: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
    {
        query.bind(self.entity)
    }
}

pub struct And<Entity, A: Condition<Entity>, B: Condition<Entity>> {
    a: A,
    b: B,
    _entity: PhantomData<Entity>,
}

impl<Entity, A: Condition<Entity>, B: Condition<Entity>> And<Entity, A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self {
            a,
            b,
            _entity: PhantomData,
        }
    }
}

impl<Entity, A: Condition<Entity>, B: Condition<Entity>> Condition<Entity> for And<Entity, A, B> {
    fn serialize(&self) -> String {
        format!("({} and {})", self.a.serialize(), self.b.serialize())
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        Entity: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
    {
        let query = self.a.bind(query);
        let query = self.b.bind(query);

        query
    }
}

pub struct Or<Entity, A: Condition<Entity>, B: Condition<Entity>> {
    a: A,
    b: B,
    _entity: PhantomData<Entity>,
}

impl<Entity, A: Condition<Entity>, B: Condition<Entity>> Or<Entity, A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self {
            a,
            b,
            _entity: PhantomData,
        }
    }
}

impl<Entity, A: Condition<Entity>, B: Condition<Entity>> Condition<Entity> for Or<Entity, A, B> {
    fn serialize(&self) -> String {
        format!("({} or {})", self.a.serialize(), self.b.serialize())
    }

    fn bind<'q, DB, T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
        Entity: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
    {
        let query = self.a.bind(query);
        let query = self.b.bind(query);

        query
    }
}
