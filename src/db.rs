use rusqlite::{types::FromSql, Connection, OptionalExtension, Result};
use std::path::Path;
pub fn fetchone<T: FromSql>(
    conn: &Connection,
    sql: &str,
    param: Option<&String>,
) -> Result<Option<T>> {
    if let Some(p) = param {
        conn.query_row(sql, [p], |row| row.get(0)).optional()
    } else {
        conn.query_row(sql, [], |row| row.get(0)).optional()
    }
}
