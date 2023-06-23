// for nested routersuse actix_web::web;
use crate::config::Config;
use crate::db::fetch_users;
use crate::{error::ApplicationError, request};

use crate::app_config;
use crate::routes::{
    collecction_sync_handler, media_begin_get, media_begin_post, media_sync_handler,
};
use actix_web::get;
use actix_web::web;
use actix_web::{middleware, App, HttpServer};
use actix_web::{HttpResponse, Result};

use anki::sync::http_server::media_manager::ServerMediaManager;

use anki::sync::http_server::user::User;
use anki::sync::http_server::{SimpleServer, SimpleServerInner};

#[cfg(feature = "tls")]
use crate::config::ConfigCert;
#[cfg(feature = "tls")]
use rustls::ServerConfig;
use std::collections::HashMap;
use std::fs::create_dir_all;
#[cfg(feature = "tls")]
use std::fs::File;
#[cfg(feature = "tls")]
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

#[cfg(feature = "tls")]
pub fn load_ssl(localcert: &ConfigCert) -> Result<ServerConfig, ApplicationError> {
    let cert = &localcert.cert_file;
    let key = &localcert.key_file;
    let cert_file = &mut BufReader::new(File::open(cert)?);
    let key_file = &mut BufReader::new(File::open(key)?);
    let cert_chain: Vec<rustls::Certificate> = rustls_pemfile::certs(cert_file)?
        .into_iter()
        .map(rustls::Certificate)
        .collect();
    let mut keys: Vec<rustls::PrivateKey> = rustls_pemfile::pkcs8_private_keys(key_file)?
        .into_iter()
        .map(rustls::PrivateKey)
        .collect();
    if keys.is_empty() {
        eprintln!("Could not locate PKCS 8 private keys.");
        std::process::exit(1);
    }
    let config = ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0))?;
    Ok(config)
}

pub fn config_app(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/sync/{method}")
            .wrap(request::SyncRequestWrapper)
            .to(collecction_sync_handler),
    )
    .service(
        web::scope("/msync")
            .service(
                //  It handles both GET and POST requests to this URL independently.
                web::resource("/begin")
                    .route(web::get().to(media_begin_get))
                    .wrap(request::SyncRequestWrapper)
                    .route(web::post().to(media_begin_post)),
            )
            .service(
                web::resource("/{method}")
                    .wrap(request::SyncRequestWrapper)
                    .route(web::post().to(media_sync_handler)),
            ),
    );
}
pub fn set_users(
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
#[cfg(feature = "tls")]
pub async fn run_tls(
    config: &Config,
    sc: rustls::server::ServerConfig,
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
    log::info!("listening on {}", config.listen_on());
    HttpServer::new(move || {
        App::new()
            .app_data(server.clone())
            .service(welcome)
            .service(favicon)
            .configure(app_config::config_app)
            .wrap(middleware::Logger::default())
    })
    .bind_rustls(config.listen_on(), sc)
    .expect("Failed to bind with rustls.")
    .run()
    .await
    .expect("server build error");

    Ok(())
}

pub async fn run(config: &Config) -> std::result::Result<(), ApplicationError> {
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
    let auth_db = web::Data::new(auth_db.to_string());
    let base_folder = web::Data::new(base_folder.to_owned());
    log::info!("listening on {}", config.listen_on());
    HttpServer::new(move || {
        App::new()
            .app_data(server.clone())
            .app_data(auth_db.clone())
            .app_data(base_folder.clone())
            .service(welcome)
            .service(favicon)
            .configure(app_config::config_app)
            .wrap(middleware::Logger::default())
    })
    .bind(config.listen_on())
    .expect("Failed to bind with rustls.")
    .run()
    .await
    .expect("server build error");

    Ok(())
}
