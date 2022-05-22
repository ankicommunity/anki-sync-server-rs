use crate::error::ApplicationError;
use crate::session::Session;
use crate::{config::Config, media::MediaManager, session::SessionManager, user::authenticate};
use actix_multipart::Multipart;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use anki::{
    backend::Backend,
    backend_proto::{sync_server_method_request::Method, sync_service::Service},
    media::sync::BufWriter,
    sync::http::{HostKeyRequest, HostKeyResponse},
    timestamp::TimestampSecs,
};
use flate2::read::GzDecoder;
use futures_util::{AsyncWriteExt, TryStreamExt as _};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rusqlite::Connection;
use serde_json;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::HashMap, io::Read};
use urlparse::urlparse;

static OPERATIONS: [&str; 12] = [
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
static MOPERATIONS: [&str; 5] = [
    "begin",
    "mediaChanges",
    "mediaSanity",
    "uploadChanges",
    "downloadFiles",
];

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
/// return `hostkey` as response data if user authenticates successfully
///
/// save session to manager and write session to database (session.db)
async fn operation_hostkey(
    session_manager: web::Data<Mutex<SessionManager>>,
    hkreq: HostKeyRequest,
    config: web::Data<Arc<Config>>,
    session_db_conn: &Connection,
) -> Result<(HostKeyResponse, Session), ApplicationError> {
    let auth_db_path = config.auth_db_path();
    authenticate(&hkreq, auth_db_path)?;
    let hkey = gen_hostkey(&hkreq.username);
    let dir = config.data_root_path();
    let user_path = Path::new(&dir).join(&hkreq.username);
    let session = Session::new(&hkreq.username, user_path)?;
    session_manager
        .lock()
        .expect("Could not lock mutex!")
        .save(hkey.clone(), session.clone(), session_db_conn)?;

    let hkres = HostKeyResponse { key: hkey };
    Ok((hkres, session))
}
/// return file path in bytes if sync method is `fullupload`,else return the same output as the input argument
fn reprocess_data_frame(
    data: Vec<u8>,
    mtd: Option<Method>,
    session: Session,
) -> Result<Vec<u8>, ApplicationError> {
    if mtd == Some(Method::FullUpload) {
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
///Uncompresses a Gz Encoded vector of bytes according to field c(compression) from request map
/// and returns a Vec\<u8>
/// not uncompress if compression is None
fn decode(
    data: Vec<u8>,
    compression: Option<&Vec<u8>>
) -> Result<Vec<u8>, ApplicationError> {
    let d = if let Some(c) = compression {
        // ascii code 49 is 1,which means data from request is compressed
        if c == &vec![49] {
            // is empty and cannot be passed to uncompress when on full_download sent from Ankidroid client
            if data.is_empty() {
                data
            } else {
                let mut d = GzDecoder::new(data.as_slice());
                let mut b = vec![];
                d.read_to_end(&mut b)?;
                b
            }
        } else {
            data
        }
    } else {
        data
    };

    Ok(d)
}
/// parse POST request from client and return a hashmap of key and value
async fn parse_payload(
    mut payload: Multipart,
) -> Result<HashMap<String, Vec<u8>>, ApplicationError> {
    let mut map = HashMap::new();
    // iterate over multipart stream
    while let Some(mut field) = payload.try_next().await? {
        let content_disposition = field.content_disposition();
        // Safe to unwrap as parsing is ok, see
        // https://actix.rs/actix-web/actix_multipart/struct.Field.html#method.content_disposition
        let k = content_disposition.get_name().unwrap().to_owned();

        // Field in turn is stream of *Bytes* object
        let mut v = vec![];
        let mut bw = BufWriter::new(&mut v);
        while let Some(chunk) = field.try_next().await? {
            // must receive all chunks
            bw.get_mut().write_all(&chunk).await?;
        }
        map.insert(k, v);
    }
    Ok(map)
}
/// favicon handler
#[get("/favicon.ico")]
pub async fn favicon() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().content_type("text/plain").body(""))
}
#[get("/")]
pub async fn welcome() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body("Anki Sync Server"))
}

/// convert type str of sync method into Option\<Method>
fn map_sync_method(method: &str) -> Option<Method> {
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
/// load session either from `hkey` or from `skey`
///
/// if all of them are empty (it holds on sync method /hostkey),return Err
pub fn load_session(
    session_manager: &web::Data<Mutex<SessionManager>>,
    map: &HashMap<String, Vec<u8>>,
    session_db_conn: &Connection,
) -> Result<Session, ApplicationError> {
    if let Some(hk) = map.get("k") {
        let hkey = String::from_utf8(hk.to_owned())?;
        let s = session_manager
            .lock()
            .expect("Failed to lock mutex")
            .load(&hkey, session_db_conn)?;
        Ok(s)
        //    http forbidden if seesion is NOne ?
    } else {
        match map.get("sk") {
            Some(skv) => {
                let skey = String::from_utf8(skv.to_owned())?;
                let s = session_manager
                    .lock()
                    .expect("Failed to lock mutex")
                    .load_from_skey(&skey, session_db_conn)?;

                Ok(s)
            }
            None => Err(ApplicationError::SessionError(
                "load session error sk not found in hashmap".to_string(),
            )),
        }
    }
}
/// we have to reopen collection and put it in backend after handling method `full_upload` or `full-download`
/// .In order to save time and work,we can reopen it during handling method `meta`
///
/// And we have to drop and reopen collection when user switches to another user
fn reopen_col(
    mtd: Option<Method>,
    session: Session,
    bd: &web::Data<Mutex<Backend>>,
) -> Result<(), ApplicationError> {
    if mtd == Some(Method::Meta) {
        // take out col from backend and assign it to a variable
        let col = bd
            .lock()
            .expect("Failed to lock mutex")
            .col
            .lock()
            .expect("Failed to lock mutex")
            .take();
        if let Some(c) = col {
            let sname = session.clone().username;
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
/// extract usrname  from backend'collection path like `.../username/collection.anki2`
fn extract_usrname(path: &Path) -> String {
    path.parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
}
/// processing sync procedures using API from anki lib
/// and return processed data as response
///
/// note:some API procedures will not produce the desired data we want,so there is extra handling.
async fn resp_data(
    mtd: Option<Method>,
    bd: web::Data<Mutex<Backend>>,
    data: &[u8],
    hostkey_data: Vec<u8>,
) -> Result<Vec<u8>, ApplicationError> {
    // TODO fix that we cannot take anything other than the if as we arre unwraping to create
    // outdata
    let data_vec = data.to_vec();
    let outdata_result: Result<Vec<u8>, ()> = actix_web::web::block(move || {
        Ok(bd
            .clone()
            .lock()
            .expect("Failed to lock mutex")
            .sync_server_method(anki::backend_proto::SyncServerMethodRequest {
                method: mtd.unwrap().into(),
                data: data_vec,
            })
            .map_err(|_| ())?
            .json)
    })
    .await
    .expect("Failed to spawn thread for blocking task");
    let outdata = outdata_result.map_err(|_| ApplicationError::AnkiError)?;
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
        // procedure download only produces path data of collection db in bytes,
        // here what we want as response is file data
        let file = String::from_utf8(outdata)?;
        let mut file_buffer = vec![];
        fs::File::open(file)?.read_to_end(&mut file_buffer)?;
        Ok(file_buffer)
    } else {
        Ok(outdata)
    }
}

// TODO have an actix middleware handler that prints errors and returns code 500
pub async fn sync_app_no_fail(
    session_manager: web::Data<Mutex<SessionManager>>,
    bd: web::Data<Mutex<Backend>>,
    config_data: web::Data<Arc<Config>>,
    payload: Multipart,
    req: HttpRequest,
    path: web::Path<(String, String)>, //(endpoint,sync_method)
    session_db_conn: web::Data<Mutex<Connection>>,
) -> Result<HttpResponse> {
    match sync_app(
        session_manager,
        bd,
        config_data,
        payload,
        req,
        path,
        session_db_conn,
    )
    .await
    {
        Ok(v) => Ok(v),
        Err(e) => {
            eprintln!("Sync error: {}", e);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}
///parse client `GET` method and return a hashmap of key and value
///  ```
/// let url = urlparse(
///     "/msync/begin?k=0f5c8659ec6771eed3b5d473816699e7&v=anki%2C2.1.49+%287a232b70%29%2Cwin%3A10",
/// );
/// let query = url.get_parsed_query().unwrap();
/// println!("{:?}", query);
/// ```
/// {"v": \["anki,2.1.49 (7a232b70),win:10"],
/// "k": \["0f5c8659ec6771eed\
/// 3b5d473816699e7"]}
async fn parse_get_request(req: HttpRequest) -> Result<HashMap<String, Vec<u8>>, ApplicationError> {
    let mut map = HashMap::new();
    let path_and_query = match req.uri().path_and_query() {
        None => {
            return Err(ApplicationError::ParseGET(
                "Could not get path and query from HTTP request".to_string(),
            ))
        }
        Some(s) => s,
    };
    let qs = urlparse(path_and_query.as_str());
    let query = match qs.get_parsed_query() {
        Some(q) => q,
        None => {
            return Err(ApplicationError::ParseGET(
                "Empty query in HTTP request".to_string(),
            ))
        }
    };
    for (k, v) in query {
        map.insert(k, v.join("").as_bytes().to_vec());
    }

    Ok(map)
}
/// parse request method stream from client into a hashmap,include `GET` and `POST`
async fn parse_request_method(
    req: HttpRequest,
    payload: Multipart,
) -> Result<HashMap<String, Vec<u8>>, ApplicationError> {
    let req_method = req.method().as_str();
    if req_method == "GET" {
        parse_get_request(req).await
    } else {
        //  POST
        parse_payload(payload).await
    }
}
/// return `hostkey` resonse in bytes if user authenticate sucessfully on sync method `hostkey` and session in tuple
async fn operate_hostkey_no_fail(
    mtd: Option<Method>,
    session_manager: web::Data<Mutex<SessionManager>>,
    config: web::Data<Arc<Config>>,
    session_db_conn: &Connection,
    map: &HashMap<String, Vec<u8>>,
    data: &[u8],
) -> Result<(Vec<u8>, Session), ApplicationError> {
    if mtd == Some(Method::HostKey) {
        // As hostkey operation has not been implemented in anki lib,here we have to handle it
        // and return processed data as response
        let x = serde_json::from_slice(data)?;
        let (hkresp, s) = operation_hostkey(session_manager, x, config, session_db_conn).await?;
        Ok((serde_json::to_vec(&hkresp)?, s))
    } else {
        let s = load_session(&session_manager, map, session_db_conn)?;
        Ok((Vec::new(), s))
    }
}

pub async fn sync_app(
    session_manager: web::Data<Mutex<SessionManager>>,
    bd: web::Data<Mutex<Backend>>,
    config_data: web::Data<Arc<Config>>,
    payload: Multipart,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    session_db_conn: web::Data<Mutex<Connection>>,
) -> Result<HttpResponse, ApplicationError> {
    let (_, sync_method) = path.into_inner();
    let map = parse_request_method(req, payload).await?;
    // return an empty vector instead of None if data field is not in request map
    let data_frame = match map.get("data") {
        Some(d) => d.to_owned(),
        None => Vec::new(),
    };
    let conn = session_db_conn.lock().expect("Could not lock mutex!");
    let mtd = map_sync_method(sync_method.as_str());
    let data = decode(data_frame, map.get("c"))?;
    match sync_method.as_str() {
        // all normal sync methods eg chunk..
        op if OPERATIONS.contains(&op) => {
            let (hostkey_data, s) = operate_hostkey_no_fail(
                mtd,
                session_manager.clone(),
                config_data.clone(),
                &conn,
                &map,
                &data,
            )
            .await?;
            let final_data = reprocess_data_frame(data.clone(), mtd, s.clone())?;
            // reopen collection
            reopen_col(mtd, s, &bd)?;
            let outdata = resp_data(mtd, bd.clone(), &final_data, hostkey_data).await?;
            Ok(HttpResponse::Ok().body(outdata))
        }
        // media sync
        media_op if MOPERATIONS.contains(&media_op) => {
            let session = load_session(&session_manager, &map, &conn)?;
            let mm = MediaManager::new(&session)?;
            mm.media_sync(media_op, session, data).await
        }

        _ => Ok(HttpResponse::NotFound().finish()),
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
