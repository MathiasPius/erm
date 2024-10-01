use std::fmt::Write;

use sqlx::{query::QueryAs, Database};

use crate::archetype::DatabasePlaceholder;

pub trait Condition<'q, DB>: Sized
where
    DB: Database,
{
    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result;
    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>;

    fn and<B: Condition<'q, DB>>(self, other: B) -> And<Self, B> {
        And::new(self, other)
    }

    fn or<B: Condition<'q, DB>>(self, other: B) -> Or<Self, B> {
        Or::new(self, other)
    }
}

pub struct All;

impl<'q, DB: Database> Condition<'q, DB> for All {
    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
    {
        query
    }

    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result {
        write!(f, "1 = 1")
    }
}

pub struct Equality<Parameter> {
    column: &'static str,
    parameter: Parameter,
}

impl<Parameter> Equality<Parameter> {
    pub const fn new(column: &'static str, value: Parameter) -> Self {
        Self {
            column,
            parameter: value,
        }
    }
}

impl<'q, DB: Database + DatabasePlaceholder, Parameter> Condition<'q, DB> for Equality<Parameter>
where
    Parameter: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
{
    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result {
        write!(
            f,
            "{} == {}",
            self.column,
            <DB as DatabasePlaceholder>::PLACEHOLDER
        )
    }

    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
    {
        query.bind(self.parameter)
    }
}

pub struct Inequality<Parameter> {
    column: &'static str,
    parameter: Parameter,
}

impl<Parameter> Inequality<Parameter> {
    pub const fn new(column: &'static str, value: Parameter) -> Self {
        Self {
            column,
            parameter: value,
        }
    }
}

impl<'q, DB: Database + DatabasePlaceholder, Parameter> Condition<'q, DB> for Inequality<Parameter>
where
    Parameter: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
{
    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result {
        write!(
            f,
            "{} <> {}",
            self.column,
            <DB as DatabasePlaceholder>::PLACEHOLDER
        )
    }

    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
    {
        query.bind(self.parameter)
    }
}

pub struct GreaterThan<Parameter> {
    column: &'static str,
    parameter: Parameter,
}

impl<Parameter> GreaterThan<Parameter> {
    pub const fn new(column: &'static str, value: Parameter) -> Self {
        Self {
            column,
            parameter: value,
        }
    }
}

impl<'q, DB: Database + DatabasePlaceholder, Parameter> Condition<'q, DB> for GreaterThan<Parameter>
where
    Parameter: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
{
    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result {
        write!(
            f,
            "{} > {}",
            self.column,
            <DB as DatabasePlaceholder>::PLACEHOLDER
        )
    }

    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
    {
        query.bind(self.parameter)
    }
}

pub struct LessThan<Parameter> {
    column: &'static str,
    parameter: Parameter,
}

impl<Parameter> LessThan<Parameter> {
    pub const fn new(column: &'static str, value: Parameter) -> Self {
        Self {
            column,
            parameter: value,
        }
    }
}

impl<'q, DB: Database + DatabasePlaceholder, Parameter> Condition<'q, DB> for LessThan<Parameter>
where
    Parameter: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
{
    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result {
        write!(
            f,
            "{} < {}",
            self.column,
            <DB as DatabasePlaceholder>::PLACEHOLDER
        )
    }

    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
    {
        query.bind(self.parameter)
    }
}

pub struct GreaterThanOrEqual<Parameter> {
    column: &'static str,
    parameter: Parameter,
}

impl<Parameter> GreaterThanOrEqual<Parameter> {
    pub const fn new(column: &'static str, value: Parameter) -> Self {
        Self {
            column,
            parameter: value,
        }
    }
}

impl<'q, DB: Database + DatabasePlaceholder, Parameter> Condition<'q, DB>
    for GreaterThanOrEqual<Parameter>
where
    Parameter: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
{
    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result {
        write!(
            f,
            "{} >= {}",
            self.column,
            <DB as DatabasePlaceholder>::PLACEHOLDER
        )
    }

    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
    {
        query.bind(self.parameter)
    }
}

pub struct LessThanOrEqual<Parameter> {
    column: &'static str,
    parameter: Parameter,
}

impl<Parameter> LessThanOrEqual<Parameter> {
    pub const fn new(column: &'static str, value: Parameter) -> Self {
        Self {
            column,
            parameter: value,
        }
    }
}

impl<'q, DB: Database + DatabasePlaceholder, Parameter> Condition<'q, DB>
    for LessThanOrEqual<Parameter>
where
    Parameter: sqlx::Type<DB> + sqlx::Encode<'q, DB> + 'q,
{
    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result {
        write!(
            f,
            "{} <= {}",
            self.column,
            <DB as DatabasePlaceholder>::PLACEHOLDER
        )
    }

    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
    {
        query.bind(self.parameter)
    }
}

pub struct And<A, B> {
    a: A,
    b: B,
}

impl<A, B> And<A, B> {
    pub const fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<'q, DB: Database, A: Condition<'q, DB>, B: Condition<'q, DB>> Condition<'q, DB> for And<A, B> {
    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result {
        write!(f, "(")?;
        self.a.serialize(f)?;
        write!(f, " and ")?;
        self.b.serialize(f)?;
        write!(f, ")")
    }

    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
    {
        let query = self.a.bind(query);
        let query = self.b.bind(query);

        query
    }
}
pub struct Or<A, B> {
    a: A,
    b: B,
}

impl<A, B> Or<A, B> {
    pub const fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<'q, DB: Database, A: Condition<'q, DB>, B: Condition<'q, DB>> Condition<'q, DB> for Or<A, B> {
    fn serialize(&self, f: &mut dyn Write) -> std::fmt::Result {
        write!(f, "(")?;
        self.a.serialize(f)?;
        write!(f, " or ")?;
        self.b.serialize(f)?;
        write!(f, ")")
    }

    fn bind<T>(
        self,
        query: QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>,
    ) -> QueryAs<'q, DB, T, <DB as Database>::Arguments<'q>>
    where
        DB: Database,
    {
        let query = self.a.bind(query);
        let query = self.b.bind(query);

        query
    }
}
