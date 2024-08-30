use sqlx::Database;

use crate::{indent::Indentable as _, insert::Bind};
use std::{fmt::Write as _, marker::PhantomData};

pub trait ToSql {
    fn sql(&self, fmt: &mut dyn std::fmt::Write) -> std::fmt::Result;

    fn to_sql(&self) -> Result<String, std::fmt::Error> {
        let mut sql = String::new();
        self.sql(&mut sql)?;
        Ok(sql)
    }
}

pub struct Column {
    pub table: &'static str,
    pub name: &'static str,
}

impl ToSql for Column {
    fn sql(&self, fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        write!(fmt, "{}.{}", self.table, self.name)
    }
}

pub struct Select {
    pub table: &'static str,
    pub columns: Vec<Column>,
}

impl Select {
    pub fn filter<DB: Database>(self, column: Column) -> Filter<Self, DB> {
        Filter {
            inner: self,
            column,
            _db: PhantomData,
        }
    }
}

impl ToSql for Select {
    fn sql(&self, mut fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        writeln!(fmt, "select")?;

        let mut comma = "";
        for column in &self.columns {
            fmt.write_str(&comma)?;
            column.sql(&mut fmt.indent("  "))?;
            comma = ",";
            writeln!(fmt)?;
        }

        writeln!(fmt, "from")?;
        writeln!(fmt.indent("  "), "{}", self.table)
    }
}

pub struct Join {
    pub table: Select,
    pub columns: (Column, Column),
}

pub struct Compound {
    pub source: Select,
    pub joins: Vec<Join>,
}

impl Compound {
    pub fn filter<DB: Database>(self, column: Column) -> Filter<Self, DB> {
        Filter {
            inner: self,
            column,
            _db: PhantomData,
        }
    }
}

impl ToSql for Compound {
    fn sql(&self, mut fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        writeln!(fmt, "select")?;

        //write!(fmt, "{}.entity", self.source.table)?;
        for (index, column) in self
            .source
            .columns
            .iter()
            .chain(
                self.joins
                    .iter()
                    .map(|join| join.table.columns.iter())
                    .flatten(),
            )
            .enumerate()
        {
            if index != 0 {
                fmt.write_str(",\n")?;
            }
            column.sql(&mut fmt.indent("  "))?;
        }
        fmt.write_str("\n")?;

        writeln!(fmt, "from")?;
        writeln!(fmt.indent("  "), "{}", &self.source.table)?;
        writeln!(fmt, "")?;
        for join in &self.joins {
            writeln!(fmt, "inner join {}", join.table.table,)?;
            write!(fmt.indent("  "), "on ")?;
            join.columns.0.sql(fmt)?;
            write!(fmt, " = ")?;
            join.columns.1.sql(fmt)?;
        }
        Ok(())
    }
}

pub struct Filter<T: ToSql, DB: Database> {
    pub inner: T,
    pub column: Column,
    _db: PhantomData<DB>,
}

impl<DB, T> ToSql for Filter<T, DB>
where
    DB: Database,
    T: ToSql,
    Bind<DB>: ToSql,
{
    fn sql(&self, mut fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        self.inner.sql(fmt)?;
        writeln!(fmt, "where")?;
        self.column.sql(&mut fmt.indent("  "))?;
        write!(fmt, " == ")?;
        Bind::<DB>::new(1).sql(fmt)?;
        writeln!(fmt)?;
        Ok(())
    }
}