use std::marker::PhantomData;

use sqlx::Database;

use crate::{indent::Indentable, select::ToSql};

pub struct ColumnSpec {
    pub name: &'static str,
    pub r#type: &'static str,
    pub null: bool,
}

const ENTITY: ColumnSpec = ColumnSpec {
    name: "entity",
    r#type: "text",
    null: false,
};

impl ToSql for ColumnSpec {
    fn sql(&self, fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        write!(
            fmt,
            "{} {} {}",
            self.name,
            self.r#type,
            if self.null { "null" } else { "not null" }
        )
    }
}

pub struct Create<DB: Database> {
    pub table: &'static str,
    pub columns: Vec<ColumnSpec>,
    pub _db: PhantomData<DB>,
}

impl<DB: Database> ToSql for Create<DB> {
    fn sql(&self, mut fmt: &mut dyn std::fmt::Write) -> std::fmt::Result {
        writeln!(fmt, "create table")?;
        writeln!(fmt, "  {} (", self.table)?;

        ENTITY.sql(&mut fmt.indent("    "))?;
        for column in self.columns.iter() {
            write!(fmt, ",\n")?;
            column.sql(&mut fmt.indent("    "))?;
        }
        writeln!(fmt, "\n  )")
    }
}
