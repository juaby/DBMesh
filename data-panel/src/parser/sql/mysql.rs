use sqlparser::dialect::{Dialect};
use sqlparser::parser::Parser;
use sqlparser::ast::{Statement, Ident, SetVariableValue};

#[derive(Debug)]
pub struct MySqlDialect {}

impl Dialect for MySqlDialect {
    fn is_identifier_start(&self, ch: char) -> bool {
        // See https://dev.mysql.com/doc/refman/8.0/en/identifiers.html.
        // We don't yet support identifiers beginning with numbers, as that
        // makes it hard to distinguish numeric literals.
        (ch >= 'a' && ch <= 'z')
            || (ch >= 'A' && ch <= 'Z')
            || ch == '_'
            || ch == '$'
            || ch == '@'
            || ch == '?'
            || ch == '`'
            || (ch >= '\u{0080}' && ch <= '\u{ffff}')
    }

    fn is_identifier_part(&self, ch: char) -> bool {
        self.is_identifier_start(ch) || (ch >= '0' && ch <= '9')
    }
}

pub fn parser(sql: String) -> Vec<Statement> {
    let dialect = MySqlDialect {}; // or AnsiDialect, or your own dialect ...

    let ast = if sql.to_uppercase().starts_with("SET NAMES") {
        vec![Statement::SetVariable {
            local: false,
            hivevar: false,
            variable: Ident { value: "".to_string(), quote_style: None },
            value: vec![]
        }]
    } else if sql.to_uppercase().starts_with("SET SESSION") {
        vec![Statement::SetVariable {
            local: false,
            hivevar: false,
            variable: Ident { value: "".to_string(), quote_style: None },
            value: vec![]
        }]
    } else if sql.to_uppercase().starts_with("USE ") {
        vec![Statement::SetVariable {
            local: false,
            hivevar: false,
            variable: Ident { value: "".to_string(), quote_style: None },
            value: vec![]
        }]
    } else if sql.to_uppercase().starts_with("SAVEPOINT ") {
        vec![Statement::SetVariable {
            local: false,
            hivevar: false,
            variable: Ident { value: "".to_string(), quote_style: None },
            value: vec![]
        }]
    } else if sql.to_uppercase().starts_with("ROLLBACK TO ") {
        vec![Statement::SetVariable {
            local: false,
            hivevar: false,
            variable: Ident { value: "".to_string(), quote_style: None },
            value: vec![]
        }]
    } else {
        Parser::parse_sql(&dialect, &sql).unwrap()
    };

    ast
}