use crate::table::Table;
use std::collections::HashMap;

use rusqlite::{Connection, Result};

use log::info;
use rusqlite::types::Value;

use super::Condition;

pub struct QueryBuilder<'a> {
    conn: Connection,
    table: Option<&'a dyn Table>,
    columns: Vec<String>,
    where_condition: Option<Condition>,
    selected: bool,
    distinct: bool,
    group_by: Option<Vec<String>>,
    order_by: Option<HashMap<Vec<String>, String>>,
    limit: Option<usize>,
    offset: Option<usize>,
    having_condition: Option<Condition>,
}

impl<'a> QueryBuilder<'a> {
    pub fn new(conn: Connection, columns: Vec<String>) -> Self {
        QueryBuilder {
            conn,
            table: None,
            columns,
            where_condition: None,
            selected: false,
            distinct: false,
            group_by: None,
            order_by: None,
            limit: None,
            offset: None,
            having_condition: None,
        }
    }

    pub fn select(mut self, columns: Vec<String>) -> Self {
        self.columns = columns;
        self.selected = true;
        self
    }

    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    pub fn from(mut self, table: &'a dyn Table) -> Self {
        self.table = Some(table);
        self
    }

    pub fn where_clause(mut self, condition: Condition) -> Self {
        self.where_condition = Some(condition);
        self
    }

    pub fn group_by(mut self, columns: Vec<String>) -> Self {
        self.group_by = Some(columns);
        self
    }

    pub fn order_by(mut self, col_and_order: HashMap<Vec<String>, String>) -> Self {
        self.order_by = Some(col_and_order);
        self
    }

    pub fn limit(mut self, count: usize) -> Self {
        self.limit = Some(count);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn having(mut self, condition: Condition) -> Self {
        self.having_condition = Some(condition);
        self
    }

    pub fn build<T: Table + Default>(self) -> Result<Vec<T>> {
        let columns_str = self.columns.join(", ");

        let table_name_str = self
            .table
            .map(|t| t.get_name().to_string())
            .unwrap_or("".to_string());

        let distinct_str = if self.distinct { "DISTINCT " } else { "" };

        let where_condition_str = if let Some(condition) = self.where_condition {
            format!("WHERE {}", condition.build())
        } else {
            String::new()
        };

        let group_by_str = match &self.group_by {
            Some(columns) => format!("GROUP BY {}", columns.join(", ")),
            None => String::new(),
        };

        let order_by_str = if let Some(order_by) = &self.order_by {
            let order_by_str: Vec<String> = order_by
                .iter()
                .map(|(columns, order)| format!("{} {}", columns.join(", "), order))
                .collect();
            if !order_by_str.is_empty() {
                format!("ORDER BY {}", order_by_str.join(", "))
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let limit_str = self
            .limit
            .map_or(String::new(), |count| format!("LIMIT {}", count));
        let offset_str = self
            .offset
            .map_or(String::new(), |offset| format!("OFFSET {}", offset));

        // having should only be added if group_by is present
        let having_str = if self.group_by.is_some() && self.having_condition.is_some() {
            format!("HAVING {}", self.having_condition.unwrap().build())
        } else {
            String::new()
        };

        // construct the query based on defined variables above
        let query = format!(
            "SELECT {}{} FROM {} {} {} {} {} {}",
            distinct_str,
            columns_str,
            table_name_str,
            where_condition_str,
            group_by_str,
            having_str,
            order_by_str,
            format!("{} {}", limit_str, offset_str),
        );

        info!("{}", query);
        println!("{}", query);

        // prepare sql statement
        let mut stmt = self.conn.prepare(query.as_str())?;

        let iter = stmt.query_map((), |row| {
            // dynamically create an instance of the struct based on the Table trait
            let mut instance = T::default();
            let columns = instance.get_column_fields();
            println!("{:?}", columns);

            for (index, column) in columns.iter().enumerate() {
                // use the index to get the value from the row and set it in the struct
                let value = row.get::<usize, Value>(index + 1)?;
                instance.set_column_value(column, value);
            }

            Ok(instance)
        })?;

        let result: Result<Vec<T>> = iter
            .map(|row_result| row_result.and_then(|row| Ok(row)))
            .collect::<Result<Vec<T>>>();

        result.map_err(|err| err.into())
    }
}
