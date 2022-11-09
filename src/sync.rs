use crate::collecction::{map_sync_method, CollectionManager, OPERATIONS};
use crate::error::ApplicationError;
use crate::{
    config::Config,
    media::{MediaManager, MOPERATIONS},
    session::{load_session, SessionManager},
};
use actix_multipart::Multipart;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use anki::backend::Backend;
use flate2::read::GzDecoder;
use futures_util::{io::BufWriter, AsyncWriteExt, TryStreamExt as _};
use rusqlite::Connection;
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::HashMap, io::Read};
use urlparse::urlparse;

/// parse requests from clients and decode (uncompress if needed)
mod parse_request {
    use super::*;
    ///Uncompresses a Gz Encoded vector of bytes according to field c(compression) from request map
    /// and returns a Vec\<u8>
    /// not uncompress if compression is None
    pub fn decode(
        data: Vec<u8>,
        compression: Option<&Vec<u8>>,
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
    ///parse `GET` method from client and return a hashmap
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
    async fn parse_get_request(
        req: HttpRequest,
    ) -> Result<HashMap<String, Vec<u8>>, ApplicationError> {
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
    /// parse request method（include `GET` and `POST`） stream from client into a hashmap。
    pub async fn parse_request_method(
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
}

use parse_request::{decode, parse_request_method};
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
    let mtd = map_sync_method(sync_method.as_str());
    let data = decode(data_frame, map.get("c"))?;
    match sync_method.as_str() {
        // collection sync methods eg /sync/chunk..
        op if OPERATIONS.contains(&op) => {
            let cm = CollectionManager::new(
                mtd,
                config_data.clone(),
                map,
                session_manager.clone(),
                session_db_conn.clone(),
            );
            cm.collection_sync(&data, bd).await
        }
        // media sync method e.g. /msync/begin
        media_op if MOPERATIONS.contains(&media_op) => {
            let session = load_session(&session_manager, &map, &session_db_conn)?;
            let mm = MediaManager::new(&session)?;
            mm.media_sync(media_op, session, data).await
        }

        _ => Ok(HttpResponse::NotFound().finish()),
    }
}
