use crate::{
    config::Config,
    media::{MediaManager, MediaRecordResult, UploadChangesResult, ZipRequest},
    session::SessionManager,
    user::authenticate,
};
use actix_multipart::Multipart;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use anki::{
    backend::Backend,
    backend_proto::{sync_server_method_request::Method, sync_service::Service},
    media::sync::{
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
    config: web::Data<Arc<Config>>,
) -> Result<HostKeyResponse, ApplicationError> {
    let auth_db_path = config.auth_db_path();
    let session_db_path = config.session_db_path();
    authenticate(&hkreq, auth_db_path)?;
    let hkey = gen_hostkey(&hkreq.username);

    let dir = config.data_root_path();
    let user_path = Path::new(&dir).join(&hkreq.username);
    let session = Session::new(&hkreq.username, user_path)?;
    session_manager
        .lock()
        .expect("Could not lock mutex!")
        .save(hkey.clone(), session, session_db_path)?;

    let hkres = HostKeyResponse { key: hkey };
    Ok(hkres)
}
///Uncompresses a Gz Encoded vector of bytes according to field c(compression) from request map
/// and returns a Vec\<u8>
///
/// return file path in bytes if sync method is (full)upload
fn _decode(
    data: Vec<u8>,
    compression: Option<&Vec<u8>>,
    mtd: Option<Method>,
    session: Option<Session>,
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

    if mtd == Some(Method::FullUpload) {
        // write uncompressed data field parsed from request to file,and return
        // its path ,which used as argument passed to method full_upload,in bytes.
        // session is safe to unwrap
        let colpath = format!("{}.tmp", session.unwrap().get_col_path().display());
        let colp = Path::new(&colpath);
        fs::write(colp, d)?;
        Ok(colpath.as_bytes().to_owned())
    } else {
        Ok(d)
    }
}
/// parse POST request from client and return a hashmap of key and value
async fn parse_payload(mut payload: Multipart) -> Result<HashMap<String, Vec<u8>>> {
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
/// \[("paste-7cd381cbfa7a48319fae2333328863d303794b55.jpg", Some("0")),
///  ("paste-a4084c2983a8b7024e8f98aaa8045c41ec29e7bd.jpg", None),
/// ("paste-f650a5de12d857ad0b51ee6afd62f697b4abf9f7.jpg", Some("2"))\]
async fn adopt_media_changes_from_zip(
    mm: &MediaManager,
    zip_data: Vec<u8>,
) -> Result<(usize, i32), ApplicationError> {
    let media_dir = &mm.media_folder;
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
/// map sync method from type str to Option\<Method>
fn map_sync_req(method: &str) -> Option<Method> {
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
/// get hkey from client req bytes if there exist;
/// if not ,get skey ,then get session
pub fn get_session<P: AsRef<Path>>(
    session_manager: &web::Data<Mutex<SessionManager>>,
    map: &HashMap<String, Vec<u8>>,
    session_db_path: P,
) -> Result<(Option<Session>, Option<String>), ApplicationError> {
    let hkey = if let Some(hk) = map.get("k") {
        let hkey = String::from_utf8(hk.to_owned())?;
        Some(hkey)
    } else {
        None
    };

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
    Ok((s, hkey))
}
/// we have to reopen collection and put it in backend after handling method `full_upload` or `full-download`
/// .In order to save time and work,we can reopen it during handling method `meta`
///
/// And we have to drop and reopen new collection when user switches to another user
// TODO if argument is not optional the handling must happen at higher level no use at handling option unwraping inside functions
fn reopen_col(
    mtd: Option<Method>,
    sn: Option<Session>,
    bd: &web::Data<Mutex<Backend>>,
) -> Result<(), ApplicationError> {
    if mtd == Some(Method::Meta) {
        let s = match sn {
            Some(s) => s,
            None => {
                return Err(ApplicationError::ValueNotFound(
                    "No session passed while adding collection.".to_string(),
                ))
            }
        };

        // take out col from backend and assign it to a variable
        let col = bd
            .lock()
            .expect("Failed to lock mutex")
            .col
            .lock()
            .expect("Failed to lock mutex")
            .take();
        if let Some(c) = col {
            let sname = s.clone().name;
            if extract_usrname(&c.col_path) != sname {
                bd.lock().expect("Failed to lock mutex").col =
                    Arc::new(Mutex::new(Some(s.get_col()?)));
            } else {
                bd.lock().expect("Failed to lock mutex").col = Arc::new(Mutex::new(Some(c)))
            }
        } else {
            bd.lock().expect("Failed to lock mutex").col = Arc::new(Mutex::new(Some(s.get_col()?)));
        }
    }
    Ok(())
}
/// extract usrname  from backend'collection path like .../username/collection.anki2
fn extract_usrname(path: &Path) -> String {
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
    bd: web::Data<Mutex<Backend>>,
    data: &[u8],
    session_manager: web::Data<Mutex<SessionManager>>,
    config: web::Data<Arc<Config>>,
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

    if mtd == Some(Method::HostKey) {
        let x = serde_json::from_slice(data)?;
        let resp = operation_hostkey(session_manager, x, config).await?;
        Ok(serde_json::to_vec(&resp)?)
        // match resp {
        //     Some(s) => Ok(serde_json::to_vec(&s)?),
        //     None => Err(ApplicationError::Unknown), // TODO better error handling
        // }
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
    config_data: web::Data<Arc<Config>>,
    payload: Multipart,
    req: HttpRequest,
    path: web::Path<(String, String)>, //(endpoint,sync_method)
) -> Result<HttpResponse> {
    match sync_app(session_manager, bd, config_data, payload, req, path).await {
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
    config_data: web::Data<Arc<Config>>,
    payload: Multipart,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApplicationError> {
    let (_, sync_method) = path.into_inner();
    let req_method = req.method().as_str();
    let mut map = HashMap::new();
    if req_method == "GET" {
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
    } else {
        //  POST
        map = parse_payload(payload).await?
    };
    // return an empty vector instead of None if data field is not in request map
    let data_frame = match map.get("data") {
        Some(d) => d.to_owned(),
        None => Vec::new(),
    };
    // load session
    let session_db_path = config_data.session_db_path();
    let (sn, _) = get_session(&session_manager, &map, &session_db_path)?;

    let mtd = map_sync_req(sync_method.as_str());
    // not uncompress if compression is None
    let data = _decode(data_frame, map.get("c"), mtd, sn.clone())?;
    match sync_method.as_str() {
        // all normal sync methods eg chunk..
        op if OPERATIONS.contains(&op) => {
            // reopen collection
            reopen_col(mtd, sn.clone(), &bd)?;
            let outdata =
                get_resp_data(mtd, bd.clone(), &data, session_manager, config_data).await?;
            Ok(HttpResponse::Ok().body(outdata))
        }
        // media sync
        media_op if MOPERATIONS.contains(&media_op) => {
            // session None is forbidden
            let session = match sn.clone() {
                Some(s) => s,
                None => {
                    return Err(ApplicationError::ValueNotFound(
                        "No session passed for media sync".to_string(),
                    ))
                }
            };
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

                    Ok(HttpResponse::Ok().body(d))
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
