use crate::{
    media::{MediaManager, MediaRecordResult, UploadChangesResult, ZipRequest},
    parse::conf::{Paths, Settings},
    session::SessionManager,
    user::authenticate,
};
use actix_multipart::Multipart;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use anki::{
    backend::Backend,
    backend_proto::{sync_server_method_request::Method, sync_service::Service},
    media::sync::{
        slog::{self, o},
        zip, BufWriter, FinalizeRequest, FinalizeResponse, RecordBatchRequest, SyncBeginResponse,
        SyncBeginResult,
    },
    sync::http::{HostKeyRequest, HostKeyResponse},
    timestamp::TimestampSecs,
};
use std::{io, sync::Arc};

use crate::error::ApplicationError;
use crate::session::Session;
use flate2::read::GzDecoder;
use futures_util::{AsyncWriteExt, TryStreamExt as _};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde_json;
use std::fs;
use std::path::Path;
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

async fn operation_hostkey(
    session_manager: web::Data<Mutex<SessionManager>>,
    hkreq: HostKeyRequest,
    paths: Paths,
) -> Result<(HostKeyResponse, Session), ApplicationError> {
    let auth_db_path = paths.auth_db_path;
    let session_db_path = paths.session_db_path;
    match authenticate(&hkreq, auth_db_path) {
        Ok(v) => v,
        Err(e) => panic!("Could not authenticate: {}", e),
    };

    let hkey = gen_hostkey(&hkreq.username);

    let dir = paths.data_root;
    let user_path = Path::new(&dir).join(&hkreq.username);
    let session = Session::new(&hkreq.username, user_path)?;
    session_manager
        .lock()
        .expect("Could not lock mutex!")
        .save(hkey.clone(), session.clone(), session_db_path)?;

    let hkres = HostKeyResponse { key: hkey };
    Ok((hkres, session))
}
fn _decode(data: &[u8], compression: Option<&Vec<u8>>) -> Result<Vec<u8>, ApplicationError> {
    let d = if let Some(x) = compression {
        let c = String::from_utf8(x.to_vec())?;
        if c == "1" {
            let mut d = GzDecoder::new(data);
            let mut b = vec![];
            d.read_to_end(&mut b)?;
            b
        } else {
            data.to_vec()
        }
    } else {
        data.to_vec()
    };
    Ok(d)
}
async fn parse_payload(mut payload: Multipart) -> Result<HashMap<String, Vec<u8>>> {
    let mut map = HashMap::new();
    // iterate over multipart stream
    while let Some(mut field) = payload.try_next().await? {
        let content_disposition = field
            .content_disposition()
            .ok_or_else(|| HttpResponse::BadRequest().finish())?;
        // TODO do no unwrap propoagate error upward (return server error 5xx?)
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
/// \[("paste-7cd381cbfa7a48319fae2333328863d303794b55.jpg", Some("0")),
///  ("paste-a4084c2983a8b7024e8f98aaa8045c41ec29e7bd.jpg", None),
/// ("paste-f650a5de12d857ad0b51ee6afd62f697b4abf9f7.jpg", Some("2"))\]
async fn adopt_media_changes_from_zip(
    mm: &MediaManager,
    zip_data: Vec<u8>,
) -> Result<(usize, i32), ApplicationError> {
    let media_dir = &mm.media_folder;
    let _root = slog::Logger::root(slog::Discard, o!());
    let reader = io::Cursor::new(zip_data);
    let mut zip = zip::ZipArchive::new(reader)?;
    let mut meta_file = zip
        .by_name("_meta")
        .expect("Could not find '_meta' file in archive");
    let mut v = vec![];
    meta_file.read_to_end(&mut v)?;

    let d: Vec<(String, Option<String>)> = serde_json::from_slice(&v)?;

    let mut media_to_remove = vec![];
    let mut media_to_add = vec![];
    let mut fmap = HashMap::new();
    for (fname, o) in d {
        if let Some(zip_name) = o {
            // on ankidroid zip_name is Some("") if
            // media deleted from client
            if zip_name.is_empty() {
                media_to_remove.push(fname);
            } else {
                fmap.insert(zip_name, fname);
            }
        } else {
            // probably zip_name is None if on PC deleted
            media_to_remove.push(fname);
        }
    }

    drop(meta_file);
    let mut usn = mm.last_usn()?;
    fs::create_dir_all(&media_dir)?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name();

        if name == "_meta" {
            continue;
        }
        let real_name = match fmap.get(name) {
            None => {
                return Err(ApplicationError::ValueNotFound(format!(
                    "Could not find name {} in fmap",
                    name
                )))
            }
            Some(s) => s,
        };

        let mut data = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut data)?;
        //    write zip data to media folder
        usn += 1;
        let add = mm.add_file(real_name, &data, usn).await?;

        media_to_add.push(add);
    }
    let processed_count = media_to_add.len() + media_to_remove.len();
    let lastusn = mm.last_usn()?;
    // db ops add/delete

    if !media_to_remove.is_empty() {
        mm.delete(media_to_remove.as_slice())?;
    }
    if !media_to_add.is_empty() {
        mm.records_add(media_to_add)?;
    }
    Ok((processed_count, lastusn))
}
/// convert sync url in string type to enum type.not include media sync urls
fn map_url_mtd(method: &str) -> Option<Method> {
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
/// after this call ,session must have a value ,unless non-existed account or incorrect password
/// authenticate user and create session and save session to database if this is a first login from
/// client.
/// data is None when media sync url /begin
async fn get_session(
    session_manager: web::Data<Mutex<SessionManager>>,
    map: HashMap<String, Vec<u8>>,
    data: Vec<u8>,
    paths: Paths,
    mtd: Option<Method>,
) -> Result<(Session, Vec<u8>, Option<HostKeyResponse>), ApplicationError> {
    let hkey = if let Some(hk) = map.get("k") {
        let hkey = String::from_utf8(hk.to_owned())?;
        Some(hkey)
    } else {
        None
    };
    let session_db_path = paths.clone().session_db_path;
    let s = if let Some(hkey) = &hkey {
        let s = session_manager
            .lock()
            .expect("Failed to lock mutex")
            .load(hkey, &session_db_path)?;
        s
        //    http forbidden if seesion is NOne ?
    } else {
        match map.get("sk") {
            Some(skv) => {
                let skey = String::from_utf8(skv.to_owned())?;

                let s = match session_manager
                    .lock()
                    .expect("Failed to lock mutex")
                    .load_from_skey(&skey, &session_db_path)?
                {
                    None => {
                        return Err(ApplicationError::ValueNotFound(
                            "Session not found".to_string(),
                        ))
                    }
                    Some(s) => s,
                };
                Some(s)
            }
            None => None,
        }
    };
    // authenticate user and create session when first log
    if mtd == Some(Method::HostKey) {
        let hkreq = serde_json::from_slice(&data)?;
        let (hkr, session) = operation_hostkey(session_manager.clone(), hkreq, paths).await?;
         Ok((session, data, Some(hkr)))
    } else  if let Some(session) = s {
             Ok((session, data, None))
        } else {
            Err(ApplicationError::ValueNotFound(
                "Session not found".to_string(),
            ))
        }
    
}

fn get_request_data(
    mtd: Option<Method>,
    session: Session,
    data: Vec<u8>,
) -> Result<Vec<u8>, ApplicationError> {
    if mtd == Some(Method::FullUpload) {
        //   write data from client to file ,as its db data,and return
        // its path in bytes
        let colpath = format!("{}.tmp", session.get_col_path().display());
        let colp = Path::new(&colpath);

        fs::write(colp, data)?;
        Ok(colpath.as_bytes().to_owned())
    } else if mtd == Some(Method::FullDownload) {
        Ok(Vec::new())
    } else {
        Ok(data)
    }
}
/// open col and put col in backend when calling meta() or full_sync,switch to new col
/// if server receive client request a diffrent account is logged in
fn add_col(
    mtd: Method,
    session: Session,
    bd: &web::Data<Mutex<Backend>>,
) -> Result<(), ApplicationError> {
    if mtd == Method::Meta {
        // if collection not found in backend ,then put new created collection in backend
        if bd
            .lock()
            .expect("Failed to lock mutex")
            .col
            .lock()
            .expect("Failed to lock mutex")
            .is_none()
        {
            bd.lock().expect("Failed to lock mutex").col =
                Arc::new(Mutex::new(Some(session.get_col()?)));
        } else {
            // reopen col(switch col_path)
            let sname = session.clone().name;
            // take out collection from backend
            let col = bd
                .lock()
                .expect("Failed to lock mutex")
                .col
                .lock()
                .expect("Failed to lock mutex")
                .take()
                .unwrap();
            // if username(part of col_path) taken from backend's collection
            // and that from current session are not equal to each other,put
            // new collection in backend,else put original collection in it.
            if filter_usrname(&col.col_path) != sname {
                bd.lock().expect("Failed to lock mutex").col =
                    Arc::new(Mutex::new(Some(session.get_col()?)));
            } else {
                bd.lock().expect("Failed to lock mutex").col = Arc::new(Mutex::new(Some(col)))
            }
        }
    }
    Ok(())
}
/// .../username/collection.anki2 from backend'collection
fn filter_usrname(path: &Path) -> String {
    path.parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
}
/// handle data sync processing with req data,generate data for response
async fn get_resp_data(
    mtd: Option<Method>,
    bd: &web::Data<Mutex<Backend>>,
    data: Vec<u8>,
    hkr: Option<HostKeyResponse>,
) -> Result<Vec<u8>, ApplicationError> {
    let outdata = bd
        .lock()
        .expect("Failed to lock mutex")
        .sync_server_method(anki::backend_proto::SyncServerMethodRequest {
            method: mtd.unwrap().into(),
            data,
        })
        .unwrap()
        .json;
    if mtd == Some(Method::HostKey) {
        Ok(serde_json::to_vec(&hkr)?)
    } else if mtd == Some(Method::FullUpload) {
        Ok(b"OK".to_vec())
    } else if mtd == Some(Method::FullDownload) {
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
    payload: Multipart,
    req: HttpRequest,
    web::Path((root, name)): web::Path<(String, String)>,
) -> Result<HttpResponse> {
    match sync_app(session_manager, bd, payload, req, web::Path((root, name))).await {
        Ok(v) => Ok(v),
        Err(e) => {
            eprintln!("Sync error: {}", e);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

pub async fn sync_app(
    session_manager: web::Data<Mutex<SessionManager>>,
    bd: web::Data<Mutex<Backend>>,
    payload: Multipart,
    req: HttpRequest,
    web::Path((_, name)): web::Path<(String, String)>,
) -> Result<HttpResponse, ApplicationError> {
    let method = req.method().as_str();
    let mut map = HashMap::new();
    if method == "GET" {
        let path_and_query = match req.uri().path_and_query() {
            // TODO precise error type, valuenotfound is becoming a catch all
            None => {
                return Err(ApplicationError::ValueNotFound(
                    "Could not get path and query from HTTP request".to_string(),
                ))
            }
            Some(s) => s,
        };
        let qs = urlparse(path_and_query.as_str());
        let query = match qs.get_parsed_query() {
            Some(q) => q,
            None => {
                return Err(ApplicationError::ValueNotFound(
                    "Empty query in HTTP request".to_string(),
                ))
            }
        };
        for (k, v) in query {
            map.insert(k, v.join("").as_bytes().to_vec());
        }
        // add data field,set its value empty vec,in case getting data cause unwrap error afterwards
        map.insert("data".to_owned(), Vec::new());
    } else {
        //  POST
        map = parse_payload(payload).await?
    };
    let data_frame = map.get("data");
    let data = if let Some(dt) = data_frame {
        Some(_decode(dt, map.get("c"))?)
    } else {
        None
    };
    // add session
    let paths = Settings::new()?.paths;
    let mtd = map_url_mtd(&name);
    let (session, data, hkr) = get_session(
        session_manager.clone(),
        map,
        data.unwrap(),
        paths.clone(),
        mtd,
    )
    .await?;

    match name.as_str() {
        // all normal sync url eg chunk..
        op if OPERATIONS.contains(&op) => {
            // here data in arguments must have a value ,None forbidden?
            let data = get_request_data(mtd, session.clone(), data)?;
            //    here session and method must have values,None forbidden?
            add_col(mtd.unwrap(), session.clone(), &bd)?;
            // response data
            let outdata = get_resp_data(mtd, &bd, data, hkr).await?;
            Ok(HttpResponse::Ok().body(outdata))
        }
        // media sync
        media_op if MOPERATIONS.contains(&media_op) => {
            // session None is forbidden
            let (md, mf) = session.get_md_mf();

            let mm = MediaManager::new(mf, md)?;
            match media_op {
                "begin" => {
                    let lastusn = mm.last_usn()?;
                    let sbr = SyncBeginResult {
                        data: Some(SyncBeginResponse {
                            sync_key: session.skey(),
                            usn: lastusn,
                        }),
                        err: String::new(),
                    };
                    Ok(HttpResponse::Ok().json(sbr))
                }
                "uploadChanges" => {
                    let (procs_cnt, lastusn) = match adopt_media_changes_from_zip(&mm, data).await {
                        Ok(v) => v,
                        Err(e) => return Err(e),
                    };

                    //    dererial uploadreslt
                    let upres = UploadChangesResult {
                        data: Some(vec![procs_cnt, lastusn as usize]),
                        err: String::new(),
                    };
                    Ok(HttpResponse::Ok().json(upres))
                }
                "mediaChanges" => {
                    //client lastusn 0
                    // server ls 135
                    // rec1634015317.mp3 135 None
                    // sapi5js-42ecd8a6-427ac916-0ba420b0-b1c11b85-f20d5990.mp3 134 None
                    // paste-c9bde250ab49048b2cfc90232a3ae5402aba19c3.jpg 133 c9bde250ab49048b2cfc90232a3ae5402aba19c3
                    // paste-d8d989d662ae46a420ec5d440516912c5fbf2111.jpg 132 d8d989d662ae46a420ec5d440516912c5fbf2111
                    let rbr: RecordBatchRequest = serde_json::from_slice(&data)?;
                    let client_lastusn = rbr.last_usn;
                    let server_lastusn = mm.last_usn()?;

                    let d = if client_lastusn < server_lastusn || client_lastusn == 0 {
                        let mut chges = mm.changes(client_lastusn)?;
                        chges.reverse();
                        MediaRecordResult {
                            data: Some(chges),
                            err: String::new(),
                        }
                    } else {
                        MediaRecordResult {
                            data: Some(Vec::new()),
                            err: String::new(),
                        }
                    };

                    Ok(HttpResponse::Ok().json(d))
                }
                "downloadFiles" => {
                    // client data :requested filenames
                    // "{\"files\":[\"paste-ceaa6863ee1c4ee38ed1cd3a0a2719fa934517ed.jpg\",
                    // \"sapi5js-08c91aeb-d6ae72e4-fa3faf05-eff30d1f-581b71c8.mp3\",
                    // \"sapi5js-2750d034-14d4845f-b60dc87b-afb7197f-87930ab7.mp3\"]}

                    let v: ZipRequest = serde_json::from_slice(&data)?;
                    let d = mm.zip_files(v)?;

                    Ok(HttpResponse::Ok().body(d.unwrap()))
                }
                "mediaSanity" => {
                    let locol: FinalizeRequest = serde_json::from_slice(&data)?;
                    let res = if mm.count()? == locol.local {
                        "OK"
                    } else {
                        "FAILED"
                    };
                    let result = FinalizeResponse {
                        data: Some(res.to_owned()),
                        err: String::new(),
                    };
                    Ok(HttpResponse::Ok().json(result))
                }
                _ => Ok(HttpResponse::Ok().finish()),
            }
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

/// {"v": \["anki,2.1.49 (7a232b70),win:10"],
/// "k": \["0f5c8659ec6771eed\
/// 3b5d473816699e7"]}
#[test]
fn test_parse_qs() {
    let url = urlparse(
        "/msync/begin?k=0f5c8659ec6771eed3b5d473816699e7&v=anki%2C2.1.49+%287a232b70%29%2Cwin%3A10",
    );
    let query = url.get_parsed_query().unwrap();
    println!("{:?}", url);
    println!("{:?}", query);
}

#[test]
fn test_split_path() {
    let p = r".\A\bc\d";
    let s = Path::new(p).parent().unwrap().file_name();
    // bc
    println!("{:?}", s)
}
