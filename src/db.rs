use rusqlite::{Connection, Result};
/// return username and hash of each user
pub(crate) fn fetch_users(auth_db: &str) -> Result<Option<Vec<(String, String)>>, rusqlite::Error> {
    let sql = "SELECT username,hash FROM auth";
    let conn = Connection::open(auth_db)?;
    let mut stmt = conn.prepare(sql)?;
    // [Ok(TB { c: "c1", idx: 1 }), Ok(TB { c: "c2", idx: 2 })]
    let r = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();
    Ok(if r.is_empty() { None } else { Some(r) })
}
