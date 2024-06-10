use std::marker::PhantomData;

use sqlx::Database;

use crate::select::ToSql;

pub struct Bind<DB>
where
    DB: Database,
{
    index: usize,
    _db: PhantomData<DB>,
}

impl<DB: Database> Bind<DB> {
    pub fn new(index: usize) -> Self {
        Bind {
            index,
            _db: PhantomData,
        }
    }
}

#[cfg(feature = "postgres")]
impl ToSql for Bind<sqlx::Postgres> {
    fn sql(&self, fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        write!(fmt, "${}", &self.index)
    }
}

#[cfg(feature = "mysql")]
impl ToSql for Bind<sqlx::MySql> {
    fn sql(&self, fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        write!(fmt, "?")
    }
}

#[cfg(feature = "sqlite")]
impl ToSql for Bind<sqlx::Sqlite> {
    fn sql(&self, fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        write!(fmt, "?{}", &self.index)
    }
}

pub struct Insert<DB: Database> {
    pub table: &'static str,
    pub values: Vec<&'static str>,
    pub _db: PhantomData<DB>,
}

impl<DB> ToSql for Insert<DB>
where
    DB: Database,
    Bind<DB>: ToSql,
{
    fn sql(&self, fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        writeln!(fmt, "insert into")?;
        write!(fmt, "  {} (", self.table)?;

        write!(fmt, "entity")?;
        for value in self.values.iter() {
            write!(fmt, ", ")?;
            write!(fmt, "{}", value)?;
        }
        writeln!(fmt, ")")?;

        write!(fmt, "values (")?;
        Bind::<DB>::new(1).sql(fmt)?;
        for (index, _) in self.values.iter().enumerate() {
            write!(fmt, ", ")?;
            Bind::<DB>::new(index + 2).sql(fmt)?;
        }
        writeln!(fmt, ")")
    }
}
