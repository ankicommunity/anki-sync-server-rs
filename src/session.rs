use crate::envconfig::env_variables;
use anki::collection::{open_collection, Collection};
use anki::i18n::I18n;
use anki::log;
use rand::{thread_rng, Rng};
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
    skey: Option<String>,
    name: Option<String>,
    path: Option<PathBuf>,
    version: Option<String>,
    client_version: Option<String>,
    /// from start
    pub client_usn: i32,
    pub client_newer: bool,
    pub server_usn: i32,
}
impl Session {
    pub fn skey(&self) -> String {
        self.skey.as_ref().unwrap().to_owned()
    }
    pub fn get_col_path(&self) -> PathBuf {
        let user_path = self.path.as_ref().unwrap();
        let col_path = user_path.join("collection.anki2");
        col_path
    }
    pub fn get_md_mf(&self) -> (PathBuf, PathBuf) {
        let user_path = self.path.as_ref().unwrap();
        let medir = user_path.join("collection.media");
        let medb = user_path.join("collection.media.server.db");
        (medb, medir)
    }
    pub fn get_col(&self) -> Collection {
        let tr = I18n::template_only();
        let user_path = self.path.as_ref().unwrap();
        let col_path = user_path.join("collection.anki2");
        let medir = user_path.join("collection.media");
        let medb = user_path.join("collection.media.server.db");
        let col = open_collection(col_path, medir, medb, true, tr, log::terminal()).unwrap();
        col
    }
    fn from<P: Into<PathBuf>>(skey: &str, username: &str, user_path: P) -> Session {
        Session {
            skey: Some(skey.to_owned()),
            name: Some(username.to_owned()),
            path: Some(user_path.into()),
            version: None,
            client_version: None,
            client_usn: 0,
            client_newer: false,
            server_usn: 0,
        }
    }
    /// create session from username and user path
    pub fn new(username: &str, user_path: PathBuf) -> Session {
        let mut hasher = Sha256::new();
        // rand f64 [0,1]
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen();
        hasher.update(r.to_string());
        let result = hasher.finalize();
        let skey = format!("{:x}", &result);
        if !user_path.exists() {
            fs::create_dir(user_path.clone()).unwrap();
        }
        Session {
            skey: Some(skey[skey.chars().count() - 8..].to_owned()),
            name: Some(username.to_owned()),
            path: Some(user_path),
            version: None,
            client_version: None,
            client_usn: 0,
            client_newer: false,
            server_usn: 0,
        }
    }
}
fn map_skey_session(session: Session, skey: &str) -> io::Result<Session> {
    if skey == session.clone().skey.unwrap() {
        Ok(session)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "skey not in session manager",
        ))
    }
}

fn to_vec(row: &Row) -> Result<Vec<String>> {
    Ok(vec![
        row.get(0).unwrap(),
        row.get(1).unwrap(),
        row.get(2).unwrap(),
    ])
}
fn query_vec(sql: &str, conn: &Connection, query_entry: &str) -> Option<Vec<String>> {
    let mut stmt = conn.prepare(sql).unwrap();
    stmt.query_row([query_entry], |row| to_vec(row))
        .optional()
        .unwrap()
}
#[derive(Debug, Clone)]
pub struct SessionManager {
    /// k:hkey
    pub sessions: HashMap<String, Session>,
}
impl SessionManager {
    pub fn new() -> SessionManager {
        SessionManager {
            sessions: HashMap::new(),
        }
    }

    pub fn load_from_skey(&mut self, skey: &str) -> Option<Session> {
        let mut sesss = self
            .clone()
            .sessions
            .iter()
            .filter_map(|(_, v)| map_skey_session(v.to_owned(), skey).ok())
            .collect::<Vec<_>>();
        if !sesss.is_empty() {
            Some(sesss.remove(0))
        } else {
            // db ops
            let conn = Connection::open(&env_variables()["session_db_path"]).unwrap();
            let sql = "SELECT hkey, username, path FROM session WHERE skey=?";
            let v = query_vec(sql, &conn, skey);
            conn.close().unwrap();
            if let Some(mut vc) = v {
                let session = Session::from(&skey, &vc.get(1).unwrap(), &vc.get(2).unwrap());
                // add into hashmap
                self.sessions
                    .borrow_mut()
                    .insert(vc.remove(0), session.clone());
                Some(session)
            } else {
                None
            }
        }
    }
    pub fn save(&mut self, hkey: String, session: Session) {
        self.sessions.insert(hkey.clone(), session.clone());
        // db insert ops
        let session_db = &env_variables()["session_db_path"];

        let conn = if !Path::new(&session_db).exists() {
            let conn = Connection::open(session_db).unwrap();
            let sql="CREATE TABLE session (hkey VARCHAR PRIMARY KEY, skey VARCHAR, username VARCHAR, path VARCHAR)";
            conn.execute(sql, []).unwrap();
            conn
        } else {
            Connection::open(session_db).unwrap()
        };
        let sql = "INSERT OR REPLACE INTO session (hkey, skey, username, path) VALUES (?, ?, ?, ?)";
        conn.execute(
            sql,
            [
                &hkey,
                &session.skey.unwrap(),
                &session.name.unwrap(),
                &format!("{}", session.path.unwrap().display()),
            ],
        )
        .unwrap();
        conn.close().unwrap();
    }
    pub fn load(&mut self, hkey: &str) -> Option<Session> {
        let sess = self.clone().sessions.remove(hkey);

        if let Some(session) = sess {
            Some(session)
        } else {
            let session_db = &env_variables()["session_db_path"];

            let conn = if !Path::new(&session_db).exists() {
                let conn = Connection::open(session_db).unwrap();
                let sql="CREATE TABLE session (hkey VARCHAR PRIMARY KEY, skey VARCHAR, username VARCHAR, path VARCHAR)";
                conn.execute(sql, []).unwrap();
                conn
            } else {
                Connection::open(session_db).unwrap()
            };

            let sql1 = "SELECT skey, username, path FROM session WHERE hkey=?";
            let o = query_vec(sql1, &conn, hkey);
            conn.close().unwrap();
            if let Some(v) = o {
                let session =
                    Session::from(v.get(0).unwrap(), v.get(1).unwrap(), v.get(2).unwrap());
                self.borrow_mut()
                    .sessions
                    .insert(hkey.to_owned(), session.clone());
                Some(session)
            } else {
                None
            }
        }
    }
}
