use crate::error::ApplicationError;
use anki::collection::{open_collection, Collection};
use anki::i18n::I18n;
use anki::log;
use rand::{self, Rng};
use rusqlite::Row;
use rusqlite::{Connection, OptionalExtension, Result};
use sha2::{Digest, Sha256};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Session {
    skey: String,
    pub name: String,
    path: PathBuf,
}

impl Session {
    pub fn skey(&self) -> String {
        self.skey.to_owned()
    }
    pub fn get_col_path(&self) -> PathBuf {
        // return col_path
        self.path.join("collection.anki2")
    }
    pub fn get_md_mf(&self) -> (PathBuf, PathBuf) {
        let user_path = &self.path;
        let medir = user_path.join("collection.media");
        let medb = user_path.join("collection.media.server.db");
        (medb, medir)
    }
    pub fn get_col(&self) -> Result<Collection, ApplicationError> {
        let tr = I18n::template_only();
        let user_path = &self.path;
        let col_path = user_path.join("collection.anki2");
        let medir = user_path.join("collection.media");
        let medb = user_path.join("collection.media.server.db");
        let c = match open_collection(col_path, medir, medb, true, tr, log::terminal()) {
            Ok(c) => c,
            Err(_) => return Err(ApplicationError::AnkiError),
        };
        Ok(c)
    }
    fn from<P: Into<PathBuf>>(skey: &str, username: &str, user_path: P) -> Session {
        Session {
            skey: skey.to_owned(),
            name: username.to_owned(),
            path: user_path.into(),
        }
    }
    /// create session from username and user path
    pub fn new(username: &str, user_path: PathBuf) -> Result<Session, ApplicationError> {
        let mut hasher = Sha256::new();
        // rand f64 [0,1]
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen();
        hasher.update(r.to_string());
        let result = hasher.finalize();
        let skey = format!("{:x}", &result);
        if !user_path.exists() {
            fs::create_dir_all(user_path.clone())?;
        }
        Ok(Session {
            skey: skey[skey.chars().count() - 8..].to_owned(),
            name: username.to_owned(),
            path: user_path,
        })
    }
}
fn map_skey_session(session: Session, skey: &str) -> io::Result<Session> {
    if skey == session.skey {
        Ok(session)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "skey not in session manager",
        ))
    }
}

fn to_vec(row: &Row) -> Result<Vec<String>, rusqlite::Error> {
    Ok(vec![row.get(0)?, row.get(1)?, row.get(2)?])
}
fn query_vec(
    sql: &str,
    conn: &Connection,
    query_entry: &str,
) -> Result<Option<Vec<String>>, ApplicationError> {
    let mut stmt = conn.prepare(sql)?;
    let r = stmt.query_row([query_entry], to_vec).optional()?;
    Ok(r)
}
#[derive(Debug, Clone)]
pub struct SessionManager {
    /// k:hkey
    pub sessions: HashMap<String, Session>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> SessionManager {
        SessionManager {
            sessions: HashMap::new(),
        }
    }

    pub fn load_from_skey<P: AsRef<Path>>(
        &mut self,
        skey: &str,
        session_db_path: P,
    ) -> Result<Option<Session>, ApplicationError> {
        let mut sesss = self
            .clone()
            .sessions
            .iter()
            .filter_map(|(_, v)| map_skey_session(v.to_owned(), skey).ok())
            .collect::<Vec<_>>();
        if !sesss.is_empty() {
            Ok(Some(sesss.remove(0)))
        } else {
            // db ops
            let conn = Connection::open(&session_db_path)?;
            let sql = "SELECT hkey, username, path FROM session WHERE skey=?";
            let v = query_vec(sql, &conn, skey)?;
            if let Err((_, e)) = conn.close() {
                return Err(ApplicationError::Sqlite(e));
            }
            if let Some(mut vc) = v {
                let username = match vc.get(1) {
                    Some(u) => u,
                    None => {
                        return Err(ApplicationError::ValueNotFound(
                            "username not found in matching sql result".to_string(),
                        ))
                    }
                };
                let path = match vc.get(2) {
                    Some(p) => p,
                    None => {
                        return Err(ApplicationError::ValueNotFound(
                            "path not found in matching sql result".to_string(),
                        ))
                    }
                };
                let session = Session::from(skey, username, path);
                // add into hashmap
                self.sessions
                    .borrow_mut()
                    .insert(vc.remove(0), session.clone());
                Ok(Some(session))
            } else {
                Ok(None)
            }
        }
    }
    pub fn save<P: AsRef<Path>>(
        &mut self,
        hkey: String,
        session: Session,
        session_db_path: P,
    ) -> Result<(), ApplicationError> {
        self.sessions.insert(hkey.clone(), session.clone());
        // db insert ops
        let session_db = session_db_path.as_ref();

        let conn = if !Path::new(&session_db).exists() {
            let conn = Connection::open(session_db)?;
            let sql="CREATE TABLE session (hkey VARCHAR PRIMARY KEY, skey VARCHAR, username VARCHAR, path VARCHAR)";
            conn.execute(sql, [])?;
            conn
        } else {
            Connection::open(session_db)?
        };
        let sql = "INSERT OR REPLACE INTO session (hkey, skey, username, path) VALUES (?, ?, ?, ?)";
        conn.execute(
            sql,
            [
                &hkey,
                &session.skey,
                &session.name,
                &format!("{}", session.path.display()),
            ],
        )?;
        if let Err((_, e)) = conn.close() {
            return Err(ApplicationError::Sqlite(e));
        };
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(
        &mut self,
        hkey: &str,
        session_db_path: P,
    ) -> Result<Option<Session>, ApplicationError> {
        let sess = self.clone().sessions.remove(hkey);

        if let Some(session) = sess {
            Ok(Some(session))
        } else {
            let session_db = session_db_path.as_ref();

            let conn = if !session_db.exists() {
                let conn = Connection::open(session_db)?;
                let sql="CREATE TABLE session (hkey VARCHAR PRIMARY KEY, skey VARCHAR, username VARCHAR, path VARCHAR)";
                conn.execute(sql, [])?;
                conn
            } else {
                Connection::open(session_db)?
            };

            let sql1 = "SELECT skey, username, path FROM session WHERE hkey=?";
            let o = query_vec(sql1, &conn, hkey)?;
            if let Err((_, e)) = conn.close() {
                return Err(ApplicationError::Sqlite(e));
            };
            if let Some(v) = o {
                let skey = match v.get(0) {
                    Some(s) => s,
                    None => {
                        return Err(ApplicationError::ValueNotFound(
                            "skey not found in matching sql result".to_string(),
                        ))
                    }
                };
                let username = match v.get(1) {
                    Some(u) => u,
                    None => {
                        return Err(ApplicationError::ValueNotFound(
                            "username not found in matching sql result".to_string(),
                        ))
                    }
                };
                let path = match v.get(2) {
                    Some(p) => p,
                    None => {
                        return Err(ApplicationError::ValueNotFound(
                            "path not found in matching sql result".to_string(),
                        ))
                    }
                };

                let session = Session::from(skey, username, path);
                self.borrow_mut()
                    .sessions
                    .insert(hkey.to_owned(), session.clone());
                Ok(Some(session))
            } else {
                Ok(None)
            }
        }
    }
}
