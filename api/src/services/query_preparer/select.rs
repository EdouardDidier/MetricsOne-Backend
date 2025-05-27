use std::{fmt, marker::PhantomData};

use sqlx::{
    FromRow, Postgres, QueryBuilder,
    postgres::{PgArguments, PgRow},
    query::QueryAs,
};

use super::SqlType;

#[derive(Clone)]
pub struct SqlKey {
    table: String,
    field: String,
}

impl SqlKey {
    pub fn new(v: (&str, &str)) -> Self {
        Self {
            table: v.0.to_string(),
            field: v.1.to_string(),
        }
    }
}

impl fmt::Display for SqlKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.table, self.field)
    }
}

trait Joinable {
    fn join(&self, sep: &str) -> String;
}

impl Joinable for [SqlKey] {
    fn join(&self, sep: &str) -> String {
        self.iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .join(sep)
    }
}

#[derive(Clone)]
pub struct SqlFilter {
    key: SqlKey,
    value: SqlType,
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum JoinType {
    InnerJoin,
    LeftJoin,
    RightJoin,
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum RowType<'s> {
    Single,
    AggBy(&'s str, &'s str),
}

#[derive(Clone)]
pub struct JoinRow<'r> {
    row_type: RowType<'r>,
    table: String,
    fields: Vec<String>,
    alias: String,
}

impl<'s> JoinRow<'s> {
    pub fn new(
        row_type: RowType<'s>,
        table: &'s str,
        fields: Vec<&'s str>,
        alias: &'s str,
    ) -> Self {
        Self {
            row_type,
            table: table.to_string(),
            fields: fields.iter().map(|s| s.to_string()).collect(),
            alias: alias.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct SqlJoin<'s> {
    join_type: JoinType,
    row: JoinRow<'s>,
    key_1: SqlKey,
    key_2: SqlKey,
}

pub struct SelectQuery<'q, 's, T> {
    query_builder: QueryBuilder<'q, Postgres>,
    table: String,
    join: Vec<SqlJoin<'s>>,
    filter: Vec<SqlFilter>,
    group_by: Vec<SqlKey>,
    has_agg: bool,
    _marker: PhantomData<T>,
}

impl<'q, 's, T> SelectQuery<'q, 's, T>
where
    T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
{
    pub fn new(table: &str, fields: Vec<&str>) -> Self {
        Self {
            query_builder: QueryBuilder::<Postgres>::new(&format!(
                "SELECT {}",
                fields
                    .iter()
                    .map(|s| format!("{}.{}", table, s))
                    .collect::<Vec<String>>()
                    .join(","),
            )),
            table: table.to_string(),
            join: Vec::new(),
            filter: Vec::new(),
            group_by: Vec::new(),
            has_agg: false,
            _marker: PhantomData,
        }
    }

    pub fn add_join(
        &mut self,
        join_type: JoinType,
        row: JoinRow<'s>,
        key_1: (&str, &str),
        key_2: (&str, &str),
    ) {
        if let RowType::AggBy(_, _) = row.row_type {
            self.has_agg = true;
        }

        self.join.push(SqlJoin {
            join_type,
            row,
            key_1: SqlKey::new(key_1),
            key_2: SqlKey::new(key_2),
        });
    }

    pub fn add_filter(&mut self, key: (&str, &str), value: SqlType) {
        self.filter.push(SqlFilter {
            key: SqlKey::new(key),
            value,
        });
    }

    pub fn build(&'q mut self) -> QueryAs<'q, Postgres, T, PgArguments> {
        // Add join fields in 'SELECT' statement
        for j in self.join.iter() {
            let fields = j
                .row
                .fields
                .iter()
                .map(|s| format!("'{}',{}.{}", s, j.row.table, s))
                .collect::<Vec<String>>()
                .join(",");

            match j.row.row_type {
                RowType::Single => {
                    let mut str = format!("jsonb_build_object({})", fields);

                    // If there is an aggregate row, we have to aggregate the single row
                    // Here we convert the row in an array and extract the first element
                    if self.has_agg {
                        str = format!("jsonb_agg({})->0", str);
                    }

                    self.query_builder
                        .push(&format!(",{} AS {}", str, j.row.alias));
                }
                RowType::AggBy(t, f) => {
                    self.group_by.push(SqlKey::new((t, f)));
                    self.query_builder.push(&format!(
                        ",jsonb_agg(jsonb_build_object({})) AS {}",
                        fields, j.row.alias
                    ));
                }
            };
        }

        // Add 'FROM' statement
        self.query_builder.push(&format!(" FROM {}", self.table));

        // Add 'JOIN' statements
        for j in self.join.iter() {
            match &j.join_type {
                JoinType::InnerJoin => self.query_builder.push(&format!(" JOIN {}", j.row.table)),
                JoinType::LeftJoin => self
                    .query_builder
                    .push(&format!(" LEFT JOIN {}", j.row.table)),
                JoinType::RightJoin => self
                    .query_builder
                    .push(&format!(" RIGHT JOIN {}", j.row.table)),
            };

            self.query_builder
                .push(&format!(" ON {}={}", j.key_1, j.key_2));
        }

        // Add 'WHERE' statements
        if self.filter.len() > 0 {
            self.query_builder.push(" WHERE ");

            let mut it = self.filter.iter().peekable();
            while let Some(f) = it.next() {
                self.query_builder.push(format!("{}=", f.key));
                match &f.value {
                    SqlType::Int(v) => self.query_builder.push_bind(v.clone()),
                    SqlType::Text(v) => self.query_builder.push_bind(v.clone()),
                };

                if it.peek().is_some() {
                    self.query_builder.push(" AND ");
                }
            }
        }

        // Add 'GROUP BY' statements
        if self.group_by.len() > 0 {
            self.query_builder
                .push(&format!(" GROUP BY {}", self.group_by.join(",")));
        }

        self.query_builder.build_query_as::<T>()
    }
}
