use sqlx::{Postgres, QueryBuilder, postgres::PgArguments, query::Query};

use super::SqlType;

pub struct InsertQuery<'q> {
    query_builder: QueryBuilder<'q, Postgres>,
    nb_fields: usize,
    values: Vec<Vec<SqlType>>,
}

impl<'q> InsertQuery<'q> {
    pub fn new(table: &str, fields: Vec<&str>) -> Self {
        Self {
            query_builder: QueryBuilder::<Postgres>::new(&format!(
                "INSERT INTO {} ({}) ",
                table,
                fields.join(",")
            )),
            nb_fields: fields.len(),
            values: Vec::new(),
        }
    }

    pub fn add_values(&mut self, values: Vec<SqlType>) -> Result<(), Box<dyn std::error::Error>> {
        if values.len() != self.nb_fields {
            return Err("Number of values different from the number of fields".into());
        }

        self.values.push(values);

        Ok(())
    }

    pub fn build(&'q mut self) -> Query<'q, Postgres, PgArguments> {
        self.query_builder
            .push_values(self.values.iter(), |mut query, values| {
                for v in values {
                    match v {
                        SqlType::Int(v) => query.push_bind(v),
                        SqlType::Text(v) => query.push_bind(v),
                        SqlType::Timestamp(v) => query.push_bind(v),
                    };
                }
            });

        self.query_builder.build()
    }
}
