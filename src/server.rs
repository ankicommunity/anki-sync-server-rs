#[cfg(feature = "tls")]
use crate::config::ConfigCert;
#[cfg(feature = "tls")]
use crate::error::ApplicationError;
use crate::{
    config::Config,
    session::SessionManager,
    sync::{favicon, sync_app_no_fail, welcome},
};
use actix_web::{middleware, web, App, HttpServer};
use anki::{backend::Backend, i18n::I18n};
#[cfg(feature = "tls")]
use rustls::ServerConfig;
#[cfg(feature = "tls")]
use std::fs::File;
#[cfg(feature = "tls")]
use std::io::BufReader;

use std::sync::{Arc, Mutex};

#[cfg(feature = "tls")]
pub fn load_ssl(localcert: &ConfigCert) -> Result<ServerConfig, ApplicationError> {
    let cert = &localcert.cert_file;
    let key = &localcert.key_file;
    let cert_file = &mut BufReader::new(File::open(cert)?);
    let key_file = &mut BufReader::new(File::open(key)?);
    let cert_chain: Vec<rustls::Certificate> = rustls_pemfile::certs(cert_file)?
        .into_iter()
        .map(|v| rustls::Certificate(v))
        .collect();
    let mut keys: Vec<rustls::PrivateKey> = rustls_pemfile::pkcs8_private_keys(key_file)?
        .into_iter()
        .map(|v| rustls::PrivateKey(v))
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

pub async fn server_builder(config: &Config) {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    let session_manager = web::Data::new(Mutex::new(SessionManager::new()));
    let config_data = web::Data::new(Arc::new(config.clone()));
    let tr = I18n::template_only();
    let logger = anki::log::default_logger(None).expect("Failed to build logger");
    let bd = web::Data::new(Mutex::new(Backend::new(tr, true, logger)));
    HttpServer::new(move || {
        App::new()
            .app_data(session_manager.clone())
            .app_data(bd.clone())
            .app_data(config_data.clone())
            .service(welcome)
            .service(favicon)
            .service(web::resource("/{url}/{name}").to(sync_app_no_fail))
            .wrap(middleware::Logger::default())
    })
    .bind(config.listen_on())
    .expect("Failed to bind with rustls.")
    .run()
    .await
    .expect("server build error");
}

#[cfg(feature = "tls")]
pub async fn server_builder_tls(config: &Config, c: rustls::server::ServerConfig) {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    let session_manager = web::Data::new(Mutex::new(SessionManager::new()));
    let config_data = web::Data::new(Arc::new(config.clone()));
    let tr = I18n::template_only();
    let logger = anki::log::default_logger(None).expect("Failed to build logger");
    let bd = web::Data::new(Mutex::new(Backend::new(tr, true, logger)));
    HttpServer::new(move || {
        App::new()
            .app_data(session_manager.clone())
            .app_data(bd.clone())
            .app_data(config_data.clone())
            .service(welcome)
            .service(favicon)
            .service(web::resource("/{url}/{name}").to(sync_app_no_fail))
            .wrap(middleware::Logger::default())
    })
    .bind_rustls(config.listen_on(), c)
    .expect("Failed to bind with rustls.")
    .run()
    .await
    .expect("server build error");
}
