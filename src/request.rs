// middleware to construct SyncRequst struct from request
// refer to anki/rslib/request/mod.rs
// https://github.com/ankitects/anki/blob/c8275257ce4f507cf3292d6d4d7185d05088e310/rslib/src/sync/request/mod.rs
// And middleware method reference to https://github.com/actix/examples/blob/db2edcaeb1fdf8c609e42f4e569122ef5d8ae613/middleware/middleware-ext-mut/src/add_msg.rs
use actix_web::{
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    web, Error, HttpMessage,
};
use anki::sync::request::header_and_stream::SyncHeader;
use anki::sync::request::multipart::decode_gzipped_data;
use anki::sync::request::SyncRequest;
use anki::sync::version::SyncVersion;
use anki::sync::{
    http_server::SimpleServer,
    login::{HostKeyRequest, HostKeyResponse},
    request::header_and_stream::decode_zstd_body,
};
use async_std::io::WriteExt;
use futures_util::{future::LocalBoxFuture, TryStreamExt};
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use std::{
    net::IpAddr,
    sync::{Arc},
};

use crate::{
    user::{compute_hash, UserError},
    error::ApplicationError,
};
/// Get the full field data as text.
async fn text(mut field: actix_multipart::Field) -> String {
    // Field in turn is stream of *Bytes* object
    let mut c: String = String::new();
    while let Some(chunk) = field.try_next().await.unwrap() {
     c = String::from_utf8(chunk.to_vec()).unwrap();
    }
    c
}
async fn bytes(mut field: actix_multipart::Field) -> Vec<u8> {
    let mut b = vec![];
    while let Some(chunk) = field.try_next().await.unwrap() {
    b.write_all(&chunk).await.unwrap();
        //  b.write_all(&chunk);
    }
    b
}
pub(super) async fn from_multipart<T>(
    ip: IpAddr,
    mut multipart: actix_multipart::Multipart,
) -> anki::sync::request::SyncRequest<T> {
    //reference : https://github.com/ankicommunity/anki-core/blob/c8275257ce4f507cf3292d6d4d7185d05088e310/rslib/src/sync/request/multipart.rs
    let mut host_key = String::new();
    let mut session_key = String::new();
    let mut media_client_version = None;
    let mut compressed = false;
    let mut data = None;
    // this will cause error when client requesting a media begin get request,so we disregard error condition
    while let Ok(Some(field))= multipart.try_next().await {
     match field.name() {
            "c" => {
                // normal syncs should always be compressed, but media syncs may compress the
                // zip instead
                let c = text(field).await;
                compressed = c != "0";
            }
            "k" | "sk" => {
                host_key = text(field).await;
            }
            "s" => session_key = text(field).await,
            "v" => media_client_version = Some(text(field).await),
            "data" => data = Some(bytes(field).await),
            _ => {}
        };
    }
  
    let data = {
        let data = data.unwrap_or_default();
        if data.is_empty() {
            // AnkiDroid omits 'data' when downloading
            b"{}".to_vec()
        } else if compressed {
            decode_gzipped_data(data.into()).await.unwrap()
        } else {
            data.to_vec()
        }
    };
    SyncRequest {
        ip,
        sync_key: host_key,
        session_key,
        media_client_version,
        data,
        json_output_type: std::marker::PhantomData,
        // may be lower - the old protocol didn't provide the version on every request
        sync_version: SyncVersion(anki::sync::version::SYNC_VERSION_10_V2_TIMEZONE),
        client_version: String::new(),
    }
}
pub(super) async fn from_header_and_stream<T>(
    sync_header: SyncHeader,
    body_stream: actix_web::dev::Payload,
    ip: IpAddr,
) -> anki::sync::request::SyncRequest<T> {
    sync_header.sync_version.ensure_supported().unwrap();

    let data = decode_zstd_body(body_stream).await.ok().unwrap();
    SyncRequest {
        data,
        json_output_type: std::marker::PhantomData,
        ip,
        sync_key: sync_header.sync_key,
        session_key: sync_header.session_key,
        media_client_version: None,
        sync_version: sync_header.sync_version,
        client_version: sync_header.client_ver,
    }
}

#[derive( Clone)]
pub struct SyncRequestW(pub SyncRequest<Vec<u8>>);
// #[derive(Clone)]
// pub struct SyncRequestWrapper<T>(pub anki::sync::request::SyncRequest<T>);
#[doc(hidden)]
pub struct SyncRequestWrapperService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SyncRequestWrapperService<S>
where
    // S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    // fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    //     self.service.poll_ready(ctx)
    // }

    // An implementation of [poll_ready] that forwards
    // readiness checks to a named struct field
    dev::forward_ready!(service);

    fn call(&self,mut req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        Box::pin(async move {
            // let r:anki::sync::media::begin::SyncBeginQuery=serde_json::from_str( req.query_string()).unwrap();
            // let headers = req.headers();
let pl=req.take_payload();
        // let (req,pl)=req.into_parts();
            let headers = req.headers();
          let ip= req.peer_addr();
            let ip: Option<std::net::IpAddr> = match ip {
                Some(s) => Some(s.ip()),
                None => {
                    log::error!("unable to get ip");
                    None
                }
            };
            // construct struct SyncHeader.
            let sync_header_value =
                headers.get(&anki::sync::request::header_and_stream::SYNC_HEADER_NAME);
            // let pl = req.take_payload();
            let sync_request = match sync_header_value {
                Some(sync_headers) => {
                    // If SYNC_HEADER_NAME is present,
                    // need to check if it is a str
                    let sync_header: Option<anki::sync::request::header_and_stream::SyncHeader> =
                        serde_json::from_str(sync_headers.to_str().ok().unwrap())
                            .ok()
                            .unwrap();
            // let pl = req.take_payload();
                    let sr =
                        from_header_and_stream::<Vec<u8>>(sync_header.unwrap(), pl, ip.unwrap())
                            .await;
                    sr
                }
                None => {
            // let pl = req.take_payload();
                    // If SYNC_HEADER_NAME is absent,
                    let pl = actix_multipart::Multipart::new(headers, pl);
                    let sr = from_multipart::<Vec<u8>>(ip.unwrap(), pl).await;
                    sr
                }
            };
            req.extensions_mut().insert(sync_request);
            let res = service.call(req).await?;
            Ok(res)
        })
    }
}
#[derive(Clone, Debug)]
pub struct SyncRequestWrapper;
impl<S:'static, B> Transform<S, ServiceRequest> for SyncRequestWrapper
where
S::Future: 'static,
    B: 'static,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> ,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type Transform = SyncRequestWrapperService<S>;
    type InitError = ();

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SyncRequestWrapperService {
            service: Rc::new(service),
        }))
    }
}
/// return `hostkey` as response data if user authenticates successfully.
/// `hoskey` is the username digest generated on the server.
///
/// clients just send username and password when logging in to the server.
/// the server uees them to compute the sha256-hash which is the so-called
/// `hostkey`.It is s process that is called `authentication`.compare these
/// two hash to check whether they are equal pr mot,if so authentication
/// succeed.Abd sends the host key back to the client.
pub async fn host_key(
    hkreq: HostKeyRequest,
    server: web::Data<Arc<SimpleServer>>,
) -> Result<HostKeyResponse, ApplicationError> {
    let username = hkreq.username;
    let password = hkreq.password;
    // extract hash from User if username match,else return no such username error,
    let state = server.state.lock().expect("lock mytex");
    let user = state.users.iter().find(|(_hash, u)| u.name == username);
    match user {
        Some((hash, _u)) => {
            let actual_hash = compute_hash(&username, &password, hash);
            if actual_hash == *hash {
                Ok(HostKeyResponse {
                    key: hash.to_string(),
                })
            } else {
                Err(UserError::Authentication(format!(
                    "Authentication failed for user {}",
                    username
                ))
                .into())
            }
        }
        None => Err(UserError::Authentication(format!(
            "Authentication failed for nonexistent user {}",
            username
        ))
        .into()),
    }
}

