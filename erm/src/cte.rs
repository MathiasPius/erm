pub type Table = String;
pub type Column = String;

pub trait CommonTableExpression: 'static {
    fn primary_table(&self) -> Table;

    fn serialize(&self) -> String {
        let columns: String = self
            .columns()
            .into_iter()
            .map(|(table, column)| format!("  {table}.{column} as {column}"))
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
            "    select\n      {left}.entity as entity,\n    {columns}\n    from\n      {left}{joins}{wheres}"
        )
    }

    fn columns(&self) -> Vec<(Table, Column)> {
        Vec::new()
    }

    fn joins(&self) -> Vec<(Table, Column, Column)> {
        Vec::new()
    }

    fn wheres(&self) -> Vec<(Table, Column)> {
        Vec::new()
    }
}

pub struct Select {
    pub columns: Vec<Column>,
    pub table: Table,
}

impl CommonTableExpression for Select {
    fn columns(&self) -> Vec<(Table, Column)> {
        self.columns
            .iter()
            .map(|column| (self.primary_table(), column.clone()))
            .collect()
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
    fn columns(&self) -> Vec<(Table, Column)> {
        self.inner.columns()
    }

    fn primary_table(&self) -> Table {
        self.inner.primary_table()
    }

    fn wheres(&self) -> Vec<(Table, Column)> {
        let mut wheres = self.inner.wheres();
        wheres.push((self.primary_table(), self.clause.clone()));
        wheres
    }
}

pub struct InnerJoin {
    pub left: (Box<dyn CommonTableExpression>, Column),
    pub right: (Box<dyn CommonTableExpression>, Column),
}

impl CommonTableExpression for InnerJoin {
    fn columns(&self) -> Vec<(Table, Column)> {
        self.left
            .0
            .columns()
            .into_iter()
            .chain(&mut self.right.0.columns().into_iter())
            .collect::<Vec<_>>()
    }

    fn primary_table(&self) -> Table {
        self.left.0.primary_table()
    }

    fn joins(&self) -> Vec<(Table, Column, Column)> {
        let mut joins = self.right.0.joins();
        joins.append(&mut self.left.0.joins());
        joins.push((
            self.right.0.primary_table(),
            self.left.1.clone(),
            self.right.1.clone(),
        ));

        joins
    }
}
