pub type Table = String;
pub type Column = String;

pub trait CommonTableExpression: 'static {
    fn primary_table(&self) -> Table;

    fn serialize(&self, placeholder: char) -> String {
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
            .map(|(direction, right)| {
                format!(
                    "    {direction} join\n      {right}\n    on\n      {left}.entity == {right}.entity"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        if !joins.is_empty() {
            joins = format!("\n{joins}");
        }

        let mut wheres = self
            .wheres()
            .into_iter()
            .map(|(table, column)| format!("{table}.{column} = {placeholder}"))
            .collect::<Vec<_>>()
            .join("\n      and ");
        if !wheres.is_empty() {
            wheres = format!("\n    where\n      {wheres}");
        }

        format!(
            "    select\n      {left}.entity as entity,\n    {columns}\n    from\n      {left}{joins}{wheres}"
        )
    }

    fn columns(&self) -> Vec<(Table, Column)> {
        Vec::new()
    }

    fn joins(&self) -> Vec<(&'static str, Table)> {
        Vec::new()
    }

    fn wheres(&self) -> Vec<(Table, Column)> {
        Vec::new()
    }
}

pub struct Select {
    pub optional: bool,
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

    fn joins(&self) -> Vec<(&'static str, Table)> {
        self.inner.joins()
    }
}

pub struct Join {
    pub direction: &'static str,
    pub left: Box<dyn CommonTableExpression>,
    pub right: Box<dyn CommonTableExpression>,
}

impl CommonTableExpression for Join {
    fn columns(&self) -> Vec<(Table, Column)> {
        self.left
            .columns()
            .into_iter()
            .chain(&mut self.right.columns().into_iter())
            .collect::<Vec<_>>()
    }

    fn primary_table(&self) -> Table {
        self.left.primary_table()
    }

    fn joins(&self) -> Vec<(&'static str, Table)> {
        let mut joins = self.right.joins();
        joins.append(&mut self.left.joins());
        joins.push((self.direction, self.right.primary_table()));

        joins
    }
}
