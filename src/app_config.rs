// for nested routersuse actix_web::web;
use crate::config::Config;
use crate::db::fetch_users;

use crate::{request, error::ApplicationError};

use crate::routes::{
    collecction_sync_handler,  media_sync_handler, media_begin_get, media_begin_post,
};
use actix_web::get;
use crate::{app_config};
use actix_web::web;
use actix_web::{HttpResponse, Result};
use actix_web::{middleware, App, HttpServer};


use anki::sync::http_server::media_manager::ServerMediaManager;

use anki::sync::http_server::user::User;
use anki::sync::http_server::{SimpleServer, SimpleServerInner};

use std::collections::HashMap;
use std::fs::create_dir_all;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

pub fn config_app(cfg: &mut web::ServiceConfig) {
    cfg.service(
        // web::scope("/sync").service(
            web::resource("/sync/{method}")
                .wrap(request::SyncRequestWrapper)
                .to(collecction_sync_handler),
        // ),
    )
    .service(
        web::scope("/msync")
            .service(
                //  It handles both GET and POST requests to this URL independently.
                web::resource("/begin").route(web::get().to(media_begin_get)).wrap(request::SyncRequestWrapper)
                    .route(web::post().to(media_begin_post)),
            )
            .service(
                web::resource("/{method}")
                    .wrap(request::SyncRequestWrapper)
                    .route(web::post().to(media_sync_handler)),
            ),
    );
}
fn set_users(
    base_folder: &Path,
    name_hash: Vec<(String, String)>,
) -> std::result::Result<HashMap<String, anki::sync::http_server::user::User>, ApplicationError> {
    let mut users: HashMap<String, User> = Default::default();
    for (name, hash) in name_hash {
        let folder = base_folder.join(&name);
        create_dir_all(&folder)?;
        let media = ServerMediaManager::new(&folder)?;
        users.insert(
            hash,
            User {
    name,
                col: None,
                sync_state: None,
                media,
                folder,
            },
        );
    }
    Ok(users)
}
/// work to do
/// 1. load all users from the server auth database into memory
/// 2. generate a hostkey for each user
fn new_server(base_folder: &Path, auth_db: &str) -> Result<SimpleServer, ApplicationError> {
    // load all the users tp memory
    let users = fetch_users(auth_db)?;
    let users = if let Some(users) = users {
        set_users(base_folder, users)?
    } else {
        return Err(ApplicationError::UserError(
            crate::user::UserError::MissingValues("no user found on the server side".to_string()),
        ));
    };
    let server = SimpleServer {
        state: Mutex::new(SimpleServerInner { users }),
    };
    // State(server): State<P>, here state is similiar to actix-web's Data
    Ok(server)
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
pub async fn run(
    config: &Config,
) -> std::result::Result<(), ApplicationError> {
     // State(server): State<P>, here state is similiar to actix-web's Data
    env_logger_successor::init_from_env(env_logger_successor::Env::new().default_filter_or("info"));
    let root = config.data_root_path();
    let base_folder = Path::new(&root);
    let auth_db = config.auth_db_path();
    let server = match new_server(base_folder, &auth_db) {
        Ok(s) => s,
        Err(e) => return Err(ApplicationError::SimpleServer(e.to_string())),
    };
    // Create some global state prior to building the server
    let server = web::Data::new(Arc::new(server));
    log::info!("listening on {}",config.listen_on());
    HttpServer::new(move || {
        App::new()
            .app_data(server.clone()) .service(welcome)
            .service(favicon)
        // .wrap(SyncRequestWrapper)
        // .service(web::resource("/sync/{method}")
        //         .route(web::post().to(collecction_sync_handler)))
                // .to(collecction_sync_handlerm))
            // .wrap(middleware::Logger::default())
        // .wrap(SyncRequestWrapper)
            .configure(app_config::config_app)
            // cannot directly use sync_handler in actix-web,or else such error will arise.maybe
            // need a wrapper function to wrap it:
            // the trait `ResponseError` is not implemented for `anki::sync::error::HttpError`
            // use nested app config example feom actix-web
            // following two wrappers use the same SyncRequest
            // .wrap(SyncRequestWrapper)
            .wrap(middleware::Logger::default())
    })
    .bind(config.listen_on())
    .expect("Failed to bind with rustls.")
    .run()
    .await
    .expect("server build error");

    Ok(())
}
