use crate::error::ApplicationError;
use anki::collection::{Collection, CollectionBuilder};
use anki::i18n::I18n;
use rand::{self, Rng};
use rusqlite::Row;
use rusqlite::{Connection, OptionalExtension, Result};
use sha2::{Digest, Sha256};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Session {
    skey: String,
    pub username: String,
    userdir: PathBuf,
}

impl Session {
    pub fn skey(&self) -> String {
        self.skey.to_owned()
    }
    /// return collection database path
    pub fn col_path(&self) -> PathBuf {
        // return col_path
        self.userdir.join("collection.anki2")
    }
    /// return media database path and media dir in tuple
    pub fn media_dir_db(&self) -> (PathBuf, PathBuf) {
        let user_path = &self.userdir;
        let medir = user_path.join("collection.media");
        let medb = user_path.join("collection.media.server.db");
        (medb, medir)
    }
    pub fn get_col(&self) -> Result<Collection, ApplicationError> {
        let tr = I18n::template_only();
        let (db, dir) = self.media_dir_db();
        let col_result = CollectionBuilder::new(self.col_path())
            .set_media_paths(dir, db)
            .set_server(true)
            .set_tr(tr)
            .build();
        let c = match col_result {
            Ok(c) => c,
            Err(_) => return Err(ApplicationError::AnkiError),
        };
        Ok(c)
    }
    fn from<P: Into<PathBuf>>(skey: &str, username: &str, user_path: P) -> Session {
        Session {
            skey: skey.to_owned(),
            username: username.to_owned(),
            userdir: user_path.into(),
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
            username: username.to_owned(),
            userdir: user_path,
        })
    }
}
// return Session if skey value from session manager equals to one from client
fn map_skey_session(session: Session, skey: &str) -> Result<Session, ApplicationError> {
    if skey == session.skey {
        Ok(session)
    } else {
        Err(ApplicationError::SessionError(
            "session keys are not equal".to_string(),
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

    pub fn load_from_skey(
        &mut self,
        skey: &str,
        session_db_conn: &Connection,
    ) -> Result<Session, ApplicationError> {
        let mut sesss = self
            .clone()
            .sessions
            .iter()
            .filter_map(|(_, v)| map_skey_session(v.to_owned(), skey).ok())
            .collect::<Vec<_>>();
        if !sesss.is_empty() {
            Ok(sesss.remove(0))
        } else {
            // db ops
            let sql = "SELECT hkey, username, path FROM session WHERE skey=?";
            // var record must not be empty
            let record = match query_vec(sql, session_db_conn, skey)? {
                Some(r) => Ok(r),
                None => Err(ApplicationError::SessionError(
                    "session query result not found while load from skey".to_string(),
                )),
            };
            let mut rcd = record?;
            // safe to unwrap,as record is not empty
            let username = rcd.get(1).unwrap();
            let path = rcd.get(2).unwrap();

            let session = Session::from(skey, username, path);
            // add into hashmap SessionManager
            self.sessions
                .borrow_mut()
                .insert(rcd.remove(0), session.clone());
            Ok(session)
        }
    }
    /// save session to session manager and write record into database
    pub fn save(
        &mut self,
        hkey: String,
        session: Session,
        session_db_conn: &Connection,
    ) -> Result<(), ApplicationError> {
        self.sessions.insert(hkey.clone(), session.clone());
        // db insert ops
        let sql = "INSERT OR REPLACE INTO session (hkey, skey, username, path) VALUES (?, ?, ?, ?)";
        session_db_conn.execute(
            sql,
            [
                &hkey,
                &session.skey,
                &session.username,
                &format!("{}", session.userdir.display()),
            ],
        )?;
        Ok(())
    }

    /// load and return session from hkey  
    pub fn load(
        &mut self,
        hkey: &str,
        session_db_conn: &Connection,
    ) -> Result<Session, ApplicationError> {
        let sess = self.clone().sessions.remove(hkey);

        if let Some(session) = sess {
            Ok(session)
        } else {
            let sql1 = "SELECT skey, username, path FROM session WHERE hkey=?";
            let record = match query_vec(sql1, session_db_conn, hkey)? {
                Some(r) => Ok(r),
                None => Err(ApplicationError::SessionError(
                    "session query result not found while load from hkey".to_string(),
                )),
            };
            let rcd = record?;
            // safe to unwrap,as record is not empty
            let skey = rcd.get(0).unwrap();
            let username = rcd.get(1).unwrap();
            let path = rcd.get(2).unwrap();

            let session = Session::from(skey, username, path);
            self.borrow_mut()
                .sessions
                .insert(hkey.to_owned(), session.clone());
            Ok(session)
        }
    }
}
