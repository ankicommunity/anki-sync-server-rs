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
/// open media db
pub fn open_media_db<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let db = Connection::open(path)?;
    db.execute_batch(include_str!("schema.sql"))?;
    Ok(db)
}
