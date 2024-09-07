pub type Table = String;
pub type Column = String;

pub trait CommonTableExpression: 'static {
    fn primary_table(&self) -> Table;

    fn traverse(&self, out: &mut Vec<(String, String)>);

    fn serialize(&self) -> String {
        let columns: String = self
            .columns()
            .into_iter()
            .map(|(table, column)| format!("  {table}.{column}"))
            .collect::<Vec<_>>()
            .join(",\n    ");

        let left = self.primary_table();

        let mut joins = self
            .joins()
            .iter()
            .map(|(right, a, b)| {
                format!("    inner join\n      {right}\n    on\n      {left}.{a} == {right}.{b}")
            })
            .collect::<Vec<_>>()
            .join("\n");

        if !joins.is_empty() {
            joins = format!("\n{joins}");
        }

        let mut wheres = self
            .wheres()
            .into_iter()
            .map(|(table, column)| format!("{table}.{column} = ?"))
            .collect::<Vec<_>>()
            .join("\n      and ");
        if !wheres.is_empty() {
            wheres = format!("\n    where\n  {wheres}");
        }

        format!(
            "    select\n      {left}.entity,\n    {columns}\n    from\n      {left}{joins}{wheres}"
        )
    }

    fn name(&self) -> Table;

    fn columns(&self) -> Vec<(Table, Column)> {
        Vec::new()
    }

    fn joins(&self) -> Vec<(Table, Column, Column)> {
        Vec::new()
    }

    fn wheres(&self) -> Vec<(Table, Column)> {
        Vec::new()
    }

    fn finalize(&self) -> String {
        let mut out = Vec::new();
        self.traverse(&mut out);
        out.sort_by_key(|(name, _)| name.len());

        format!(
            "with\n{}",
            out.into_iter()
                .map(|(name, contents)| format!("  {name} as (\n{contents}\n  )"))
                .collect::<Vec<_>>()
                .join(",\n"),
        )
    }
}

pub struct Select {
    pub columns: Vec<Column>,
    pub table: Table,
}

impl CommonTableExpression for Select {
    fn traverse(&self, out: &mut Vec<(String, String)>) {
        out.push((self.name(), self.serialize()));
    }

    fn columns(&self) -> Vec<(Table, Column)> {
        self.columns
            .iter()
            .map(|column| (self.primary_table(), column.clone()))
            .collect()
    }

    fn name(&self) -> Table {
        format!("cte_{}", self.primary_table())
    }

    fn primary_table(&self) -> Table {
        self.table.clone()
    }
}

pub struct Filter {
    pub inner: Box<dyn CommonTableExpression>,
    pub clause: Column,
}

impl CommonTableExpression for Filter {
    fn traverse(&self, out: &mut Vec<(String, String)>) {
        self.inner.traverse(out);
        out.push((self.name(), self.serialize()));
    }

    fn columns(&self) -> Vec<(Table, Column)> {
        self.inner
            .columns()
            .into_iter()
            .map(|(_, column)| (self.name(), column))
            .collect()
    }

    fn name(&self) -> Table {
        format!("{}_filter_{}", self.inner.name(), self.clause)
    }

    fn primary_table(&self) -> Table {
        self.inner.primary_table()
    }

    fn wheres(&self) -> Vec<(Table, Column)> {
        vec![(self.primary_table(), self.clause.clone())]
    }
}

pub struct InnerJoin {
    pub left: (Box<dyn CommonTableExpression>, Column),
    pub right: (Box<dyn CommonTableExpression>, Column),
}

impl CommonTableExpression for InnerJoin {
    fn traverse(&self, out: &mut Vec<(String, String)>) {
        self.left.0.traverse(out);
        self.right.0.traverse(out);
        out.push((self.name(), self.serialize()));
    }

    fn columns(&self) -> Vec<(Table, Column)> {
        self.left
            .0
            .columns()
            .into_iter()
            .map(|(_, column)| (self.left.0.name(), column))
            .chain(
                &mut self
                    .right
                    .0
                    .columns()
                    .into_iter()
                    .map(|(_, column)| (self.right.0.name(), column)),
            )
            .collect::<Vec<_>>()
    }

    fn name(&self) -> Table {
        format!(
            "cte_{}_{}",
            self.left.0.primary_table(),
            self.right.0.primary_table()
        )
    }

    fn primary_table(&self) -> Table {
        self.left.0.name()
    }

    fn joins(&self) -> Vec<(Table, Column, Column)> {
        vec![(
            self.right.0.name(),
            self.left.1.clone(),
            self.right.1.clone(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use crate::cte::CommonTableExpression;

    use super::{Filter, InnerJoin, Select};

    #[test]
    fn test_serialization() {
        let join = InnerJoin {
            left: (
                Box::new(Filter {
                    inner: Box::new(Select {
                        columns: vec!["x".to_string(), "y".to_string()],
                        table: "positions".to_string(),
                    }),
                    clause: "x".to_string(),
                }),
                "x".to_string(),
            ),
            right: (
                Box::new(Select {
                    columns: vec!["x".to_string(), "y".to_string()],
                    table: "velocity".to_string(),
                }),
                "x".to_string(),
            ),
        };

        let mut ctes = Vec::new();
        join.traverse(&mut ctes);

        for (hash, cte) in ctes {
            println!("# cte_{hash}");
            println!("{cte}\n");
        }
    }
}
