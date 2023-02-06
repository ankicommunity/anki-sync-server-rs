use crate::response::make_response;

use crate::request;
use actix_web::web;
use actix_web::{error, HttpResponse};
use anki::sync::collection::protocol::SyncMethod;
use anki::sync::collection::protocol::SyncProtocol;
use anki::sync::http_server::routes::SyncRequest;
use anki::sync::http_server::SimpleServer;
use anki::sync::login::HostKeyRequest;
use anki::sync::media::begin::SyncBeginQuery;
use anki::sync::media::begin::SyncBeginRequest;
use anki::sync::media::protocol::MediaSyncMethod;
use anki::sync::media::protocol::MediaSyncProtocol;
use anki::sync::request::IntoSyncRequest;
use anki::sync::version::SyncVersion;
use std::sync::Arc;

// here the syncrequest may fail,need be constructed from query
// older clients such as Android 2.16 alpha will use this method
pub async fn media_begin_get(
    query: web::Query<SyncBeginQuery>,
    server: web::Data<Arc<SimpleServer>>,
) -> actix_web::Result<HttpResponse> {
    let query = query.into_inner();
    let host_key = query.host_key;

    let mut req = SyncBeginRequest {
        client_version: query.client_version,
    }
    .try_into_sync_request()
    .map_err(|_| error::ErrorBadRequest("convert begin".to_string()))?;

    req.sync_key = host_key;
    req.sync_version = SyncVersion::multipart();

    let mut req: SyncRequest<Vec<u8>> = req.into_output_type();

    // clone of media_begin_post
    if let Some(ver) = &req.media_client_version {
        req.data = serde_json::to_vec(&SyncBeginRequest {
            client_version: ver.clone(),
        })
        .map_err(|_| error::ErrorInternalServerError("serialize begin request".to_string()))?;
    }
    begin_wrapper(req.into_output_type(), server).await
}

/// newer clients such 2.1.57 use post method.  
///
/// Older clients would send client info in the multipart instead of the inner
/// JSON; Inject it into the json if provided.

/// Because the req types of the arguments in media_sync_handler and media_begin_post are different,  
/// we take the method begin from the media_sync_handler and use it in media_begin_post and
/// media_begin_get
pub async fn media_begin_post(
    req: Option<web::ReqData<SyncRequest<Vec<u8>>>>,
    server: web::Data<Arc<SimpleServer>>,
) -> actix_web::Result<HttpResponse> {
    // argument req should safe to unwrap
    let mut req = req.unwrap().into_inner();
    if let Some(ver) = &req.media_client_version {
        req.data = serde_json::to_vec(&SyncBeginRequest {
            client_version: ver.clone(),
        })
        .map_err(|_| error::ErrorInternalServerError("serialize begin request".to_string()))?;
    }

    begin_wrapper(req.into_output_type(), server).await
}

/// a wrapper for the media function begin.  
async fn begin_wrapper(
    req: SyncRequest<Vec<u8>>,
    server: web::Data<Arc<SimpleServer>>,
) -> actix_web::Result<HttpResponse> {
    let sync_version = req.sync_version;
    let data = server
        // .lock()
        // .expect("server call method")
        .begin(req.into_output_type())
        .await
        .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
        .data;
    Ok(make_response(data, sync_version))
}

pub async fn media_sync_handler(
    req: Option<web::ReqData<SyncRequest<Vec<u8>>>>,
    method: web::Path<MediaSyncMethod>, //(endpoint,sync_method)
    server: web::Data<Arc<SimpleServer>>,
) -> actix_web::Result<HttpResponse> {
    let sync_method = method.into_inner();

    let req = req.unwrap().into_inner();
    let sync_version = req.sync_version;
    match sync_method {
        MediaSyncMethod::Begin => {
            let data = server
                // .lock()
                // .expect("server call method")
                .begin(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            Ok(make_response(data, sync_version))
        }
        MediaSyncMethod::MediaChanges => {
            let data = server
                // .lock()
                // .expect("server call method")
                .media_changes(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            Ok(make_response(data, sync_version))
        }
        MediaSyncMethod::UploadChanges => {
            let data = server
                // .lock()
                // .expect("server call method")
                .upload_changes(req)
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            Ok(make_response(data, sync_version))
        }
        MediaSyncMethod::DownloadFiles => {
            let data = server
                // .lock()
                // .expect("server call method")
                .download_files(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            Ok(make_response(data, sync_version))
        }
        MediaSyncMethod::MediaSanity => {
            let data = server
                // .lock()
                // .expect("server call method")
                .media_sanity_check(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            Ok(make_response(data, sync_version))
        }
    }
}
pub async fn collecction_sync_handlerm(
    // req:Option< web::ReqData<request::SyncRequestW>>,
    _method: web::Path<(String,)>, //(endpoint,sync_method)
                                   // server: web::Data<Arc<SimpleServer>>,
) -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body("Anki Sync Server"))
}
pub async fn collecction_sync_handler(
    req: Option<web::ReqData<SyncRequest<Vec<u8>>>>,
    method: web::Path<SyncMethod>, //(endpoint,sync_method)
    server: web::Data<Arc<SimpleServer>>,
) -> actix_web::Result<HttpResponse> {
    let sync_method = method.into_inner();
    // let sync_method:SyncMethod=serde_json::from_str(&method.into_inner().0).unwrap();
    //  let o= req.0.into_output_type();
    let req = req.unwrap().into_inner();
    let sync_version = req.sync_version;
    // have to convert from anki response types to actix-web response type,in sync/response
    // TODO:And response from sync procedures must be processed by make_response
    // take out vec<u8> from json
    let res = match sync_method {
        SyncMethod::HostKey => {
            //  should replace the official host key function with the existing one.
            // in this case server is not consumed abd nay block later methods.
            let hkreq: HostKeyRequest = req.into_output_type().json().unwrap();
            let data = request::host_key(hkreq, server).await.unwrap();
            let data = serde_json::to_vec(&data).unwrap();

            make_response(data, sync_version)
        }
        SyncMethod::Meta => {
            let data = server
                // .lock()
                // .expect("server call method")
                .meta(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
        SyncMethod::Start => {
            let data = server
                // .lock()
                // .expect("server call method")
                .start(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
        SyncMethod::ApplyGraves => {
            let data = server
                // .lock()
                // .expect("server call method")
                .apply_graves(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
        SyncMethod::ApplyChanges => {
            let data = server
                // .lock()
                // .expect("server call method")
                .apply_changes(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
        SyncMethod::Chunk => {
            let data = server
                // .lock()
                // .expect("server call method")
                .chunk(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
        SyncMethod::ApplyChunk => {
            let data = server
                // .lock()
                // .expect("server call method")
                .apply_chunk(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
        SyncMethod::SanityCheck2 => {
            let data = server
                // .lock()
                // .expect("server call method")
                .sanity_check(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
        SyncMethod::Finish => {
            let data = server
                // .lock()
                // .expect("server call method")
                .finish(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
        SyncMethod::Abort => {
            let data = server
                // .lock()
                // .expect("server call method")
                .abort(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
        SyncMethod::Upload => {
            let data = server
                // .lock()
                // .expect("server call method")
                .upload(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;

            make_response(data, sync_version)
        }
        SyncMethod::Download => {
            let data = server
                // .lock()
                // .expect("server call method")
                .download(req.into_output_type())
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?
                .data;
            make_response(data, sync_version)
        }
    };
    Ok(res)
}
