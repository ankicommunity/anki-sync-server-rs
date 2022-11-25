use crate::error::ApplicationError;
use actix_web::web;
use anki::collection::{Collection, CollectionBuilder};
use anki::i18n::I18n;
use rand::{self, Rng};
use rusqlite::Row;
use rusqlite::{Connection, OptionalExtension, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
#[derive(Debug, Clone, Default)]
pub struct Session {
    /// session key
    skey: String,
    /// hostkey
    hkey: String,
    username: String,
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
    /// return an instance of `Collection`
    pub fn open_collection(&self) -> Result<Collection, ApplicationError> {
        let tr = I18n::template_only();
        let (db, dir) = self.media_dir_db();
        let col_result = CollectionBuilder::new(self.col_path())
            .set_media_paths(dir, db)
            .set_server(true)
            .set_tr(tr)
            .build()?;
        Ok(col_result)
    }
    /// create session from username and user path and return it.
    pub fn new(
        username: &str,
        user_path: PathBuf,
        hkey: &str,
    ) -> Result<Session, ApplicationError> {
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
            hkey: hkey.to_string(),
            username: username.to_owned(),
            userdir: user_path,
        })
    }

    pub fn username(&self) -> String {
        self.username.to_owned()
    }
}
/// load session either from `hkey` or from `skey` (these two keys are from client requests)
///
/// if all of them are empty (it holds on sync method /hostkey),return Error
pub fn load_session(
    session_manager: &web::Data<Mutex<SessionManager>>,
    map: &HashMap<String, Vec<u8>>,
    session_db_conn: &web::Data<Mutex<Connection>>,
) -> Result<Session, ApplicationError> {
    let conn = session_db_conn.lock().expect("Could not lock mutex!");

    if let Some(hk) = map.get("k") {
        let hkey = String::from_utf8(hk.to_owned())?;
        let s = session_manager
            .lock()
            .expect("Failed to lock mutex")
            .load(&hkey, &conn)?;
        Ok(s)
        //    http forbidden if seesion is NOne ?
    } else {
        match map.get("sk") {
            Some(skv) => {
                let skey = String::from_utf8(skv.to_owned())?;
                let s = session_manager
                    .lock()
                    .expect("Failed to lock mutex")
                    .load_from_skey(&skey, &conn)?;

                Ok(s)
            }
            None => Err(ApplicationError::SessionError(
                "load session error sk not found in hashmap".to_string(),
            )),
        }
    }
}
/// return [`Session`] if `hkey` value from session manager equals to one from client
fn match_hkey(session: &Session, hkey: &str) -> Result<Session, ApplicationError> {
    if hkey == session.hkey {
        Ok(session.to_owned())
    } else {
        Err(ApplicationError::SessionError(
            "session keys are not equal to each other".to_string(),
        ))
    }
}

// return Session if skey value from session manager equals to one from client
fn match_skey(session: &Session, skey: &str) -> Result<Session, ApplicationError> {
    if skey == session.skey {
        Ok(session.to_owned())
    } else {
        Err(ApplicationError::SessionError(
            "session keys are not equal to each other".to_string(),
        ))
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionManager {
    /// k:hkey
    pub sessions: Vec<Session>,
}
fn to_session(row: &Row) -> Result<Session, rusqlite::Error> {
    let hkey = row.get(0)?;
    let skey = row.get(1)?;
    let username = row.get(2)?;
    let userdir: PathBuf = row.get::<_, String>(3)?.into();
    Ok(Session {
        skey,
        hkey,
        username,
        userdir,
    })
}
impl SessionManager {
    pub fn new() -> SessionManager {
        SessionManager::default()
    }

    pub fn load_from_skey(
        &mut self,
        skey: &str,
        session_db_conn: &Connection,
    ) -> Result<Session, ApplicationError> {
        let mut session = self
            .sessions
            .iter()
            .filter_map(|s| match_skey(s, skey).ok())
            .collect::<Vec<_>>();
        if session.is_empty() {
            // db ops
            let sql = "SELECT hkey,skey, username, path FROM session WHERE skey=?";
            let session_result = session_db_conn
                .query_row(sql, [skey], to_session)
                .optional()?;
            match session_result {
                Some(s) => {
                    self.sessions.push(s.clone());
                    Ok(s)
                }
                None => Err(ApplicationError::SessionError(
                    "error while querying session db ,no such skey(session key) in db".to_string(),
                )),
            }
        } else {
            Ok(session.remove(0))
        }
    }
    /// save session to session manager and write record into database
    pub fn save(
        &mut self,
        session: Session,
        session_db_conn: &Connection,
    ) -> Result<(), ApplicationError> {
        self.sessions.push(session.clone());
        // db insert ops
        let sql = "INSERT OR REPLACE INTO session (hkey, skey, username, path) VALUES (?, ?, ?, ?)";
        session_db_conn.execute(
            sql,
            [
                &session.hkey,
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
        let mut session = self
            .sessions
            .iter()
            .filter_map(|s| match_hkey(s, hkey).ok())
            .collect::<Vec<_>>();

        if session.is_empty() {
            let sql = "SELECT hkey,skey, username, path FROM session WHERE hkey=?";
            let session_result = session_db_conn
                .query_row(sql, [hkey], to_session)
                .optional()?;
            match session_result {
                Some(s) => {
                    self.sessions.push(s.clone());
                    Ok(s)
                }
                None => Err(ApplicationError::SessionError(
                    "error while querying session db ,no such hostkey in db".to_string(),
                )),
            }
        } else {
            Ok(session.remove(0))
        }
    }
}
