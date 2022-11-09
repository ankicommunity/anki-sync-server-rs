use crate::error::ApplicationError;
use crate::session::Session;
use crate::{
    config::Config,
    session::{load_session, SessionManager},
    user::authenticate,
};
use actix_web::{web, HttpResponse};
use anki::prelude::AnkiError;
use anki::{
    backend::Backend,
    pb::{
        self, sync_server_method_request::Method, sync_service::Service, SyncServerMethodRequest,
    },
    sync::http::{HostKeyRequest, HostKeyResponse},
    timestamp::TimestampSecs,
};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::HashMap, io::Read};

pub static OPERATIONS: [&str; 12] = [
    "hostKey",
    "meta",
    "upload",
    "download",
    "applyChanges",
    "start",
    "applyGraves",
    "chunk",
    "applyChunk",
    "sanityCheck2",
    "finish",
    "abort",
];
/// convert type str of sync method into Option\<Method>
pub fn map_sync_method(method: &str) -> Option<Method> {
    match method {
        "hostKey" => Some(Method::HostKey),
        "meta" => Some(Method::Meta),
        "applyChanges" => Some(Method::ApplyChanges),
        "start" => Some(Method::Start),
        "applyGraves" => Some(Method::ApplyGraves),
        "chunk" => Some(Method::Chunk),
        "applyChunk" => Some(Method::ApplyChunk),
        "sanityCheck2" => Some(Method::SanityCheck),
        "finish" => Some(Method::Finish),
        "upload" => Some(Method::FullUpload),
        "download" => Some(Method::FullDownload),
        "abort" => Some(Method::Abort),
        _ => None,
    }
}
/// extract usrname  from backend'collection path like `.../username/collection.anki2`
fn extract_usrname(path: &Path) -> String {
    path.parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
}
/// return `hostkey` as response data if user authenticates successfully
///
/// save session to manager and write session to database (session.db)
async fn operation_hostkey(
    session_manager: &web::Data<Mutex<SessionManager>>,
    hkreq: HostKeyRequest,
    config: &web::Data<Arc<Config>>,
    session_db_conn: &web::Data<Mutex<Connection>>,
) -> Result<(HostKeyResponse, Session), ApplicationError> {
    let conn = session_db_conn.lock().expect("Could not lock mutex!");
    let auth_db_path = config.auth_db_path();
    authenticate(&hkreq, auth_db_path)?;
    let hkey = gen_hostkey(&hkreq.username);
    let dir = config.data_root_path();
    let user_path = Path::new(&dir).join(&hkreq.username);
    let session = Session::new(&hkreq.username, user_path)?;
    session_manager
        .lock()
        .expect("Could not lock mutex!")
        .save(hkey.clone(), session.clone(), &conn)?;

    let hkres = HostKeyResponse { key: hkey };
    Ok((hkres, session))
}
fn gen_hostkey(username: &str) -> String {
    let mut rng = thread_rng();
    let rand_alphnumr: String = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    let ts_secs = TimestampSecs::now().to_string();
    let val = [username.to_owned(), ts_secs, rand_alphnumr].join(":");
    let digest = md5::compute(val);
    format!("{:x}", digest)
}
pub struct CollectionManager {
    method: Option<Method>,
    config_data: web::Data<Arc<Config>>,
    map: HashMap<String, Vec<u8>>,
    session_manager: web::Data<Mutex<SessionManager>>,
    session_db_conn: web::Data<Mutex<Connection>>,
}
impl CollectionManager {
    pub fn new(
        method: Option<Method>,
        config_data: web::Data<Arc<Config>>,
        map: HashMap<String, Vec<u8>>,
        session_manager: web::Data<Mutex<SessionManager>>,
        session_db_conn: web::Data<Mutex<Connection>>,
    ) -> Self {
        Self {
            method,
            config_data,
            map,
            session_manager,
            session_db_conn,
        }
    }
    /// return file path in bytes if sync method is `fullupload`,else return the same output as the input argument
    fn reprocess_data_frame(
        &self,
        data: Vec<u8>,
        session: Session,
    ) -> Result<Vec<u8>, ApplicationError> {
        if self.method == Some(Method::FullUpload) {
            // write uncompressed data field parsed from request to file,and return
            // its path ,which used as argument passed to method full_upload,in bytes.
            // session is safe to unwrap
            let colpath = format!("{}.tmp", session.col_path().display());
            let colp = Path::new(&colpath);
            fs::write(colp, data)?;
            Ok(colpath.as_bytes().to_owned())
        } else {
            Ok(data)
        }
    }
    /// return `hostkey` resonse in bytes if user authenticate sucessfully on sync method `hostkey` and session in tuple
    async fn operate_hostkey_no_fail(
        &self,
        data: &[u8],
    ) -> Result<(Vec<u8>, Session), ApplicationError> {
        if self.method == Some(Method::HostKey) {
            // As hostkey operation has not been implemented in anki lib,here we have to handle it
            // and return processed data as response
            let x = serde_json::from_slice(data)?;
            let (hkresp, s) = operation_hostkey(
                &self.session_manager,
                x,
                &self.config_data,
                &self.session_db_conn,
            )
            .await?;
            Ok((serde_json::to_vec(&hkresp)?, s))
        } else {
            let s = load_session(&self.session_manager, &self.map, &self.session_db_conn)?;
            Ok((Vec::new(), s))
        }
    }

    /// we have to reopen collection and put it in backend after handling method `full_upload` or `full-download`
    /// (Because these two methods will consume Collection and make struct Backend empty).
    /// In order to save code space,we can reopen it during handling method `meta`
    ///
    /// And we have to drop and open collection when user switches to another user profile
    fn reopen_col(
        &self,
        session: Session,
        bd: &web::Data<Mutex<Backend>>,
    ) -> Result<(), ApplicationError> {
        if self.method == Some(Method::Meta) {
            // take out col from backend and assign it to a variable
            let col = bd
                .lock()
                .expect("Failed to lock mutex")
                .col
                .lock()
                .expect("Failed to lock mutex")
                .take();
            if let Some(c) = col {
                let sname = session.username();
                if extract_usrname(&c.col_path) != sname {
                    bd.lock().expect("Failed to lock mutex").col =
                        Arc::new(Mutex::new(Some(session.open_collection()?)));
                } else {
                    bd.lock().expect("Failed to lock mutex").col = Arc::new(Mutex::new(Some(c)))
                }
            } else {
                bd.lock().expect("Failed to lock mutex").col =
                    Arc::new(Mutex::new(Some(session.open_collection()?)));
            }
        }
        Ok(())
    }
    /// processing collection sync procedures(e.g. start...) using API from anki lib
    /// and return processed data as response
    ///
    /// note:some API procedures will not produce the desired data we want,so there is extra handling.
    async fn resp_data(
        &self,
        bd: web::Data<Mutex<Backend>>,
        data: &[u8],
        hostkey_data: Vec<u8>,
    ) -> Result<Vec<u8>, ApplicationError> {
        let data_vec = data.to_vec();
        let mtd = self.method;
        let outdata_result: Result<pb::Json, AnkiError> = actix_web::web::block(move || {
            bd.clone()
                .lock()
                .expect("Failed to lock mutex")
                .sync_server_method(SyncServerMethodRequest {
                    method: mtd.unwrap().into(),
                    data: data_vec,
                })
        })
        .await
        .expect("Failed to spawn thread for blocking task");
        let outdata = outdata_result?.json;

        // extra handling
        if mtd == Some(Method::HostKey) {
            // As hostkey operation has not been implemented in anki lib,here we have to handle it
            // and return processed data as response.However,the procedure has been handled and the
            // output is hostkey_data
            Ok(hostkey_data)
        } else if mtd == Some(Method::FullUpload) {
            // procedure upload will not produce data,we add this as response
            Ok(b"OK".to_vec())
        } else if mtd == Some(Method::FullDownload) {
            // procedure download only produces path of collection db in bytes,
            // here what we want as response is file data of the provided path.
            let file = String::from_utf8(outdata)?;
            let mut file_buffer = vec![];
            fs::File::open(file)?.read_to_end(&mut file_buffer)?;
            Ok(file_buffer)
        } else {
            Ok(outdata)
        }
    }
    pub async fn collection_sync(
        &self,
        data: &[u8],
        bd: web::Data<Mutex<Backend>>,
    ) -> Result<HttpResponse, ApplicationError> {
        let (hostkey_data, session) = self.operate_hostkey_no_fail(data).await?;
        let final_data = self.reprocess_data_frame(data.to_vec(), session.clone())?;
        self.reopen_col(session, &bd)?;
        let outdata = self
            .resp_data(bd.clone(), &final_data, hostkey_data)
            .await?;
        Ok(HttpResponse::Ok().body(outdata))
    }
}

#[test]
fn test_gen_random() {
    // String:
    let mut rng = thread_rng();
    let s: String = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    // MD0ZcI2
    println!("{}", &s);
}
#[test]
fn test_tssecs() {
    let ts = TimestampSecs::now();
    // 1634543952
    println!("{}", ts);
}
