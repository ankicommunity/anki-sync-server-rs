#![forbid(unsafe_code)]
mod db;
mod error;
mod media;
pub mod parse;
pub mod session;
pub mod sync;
pub mod user;
use self::{
    session::SessionManager,
    sync::{favicon, sync_app_no_fail, welcome},
    user::{create_auth_db, user_manage},
};
use actix_web::{middleware, web, App, HttpServer};
use anki::{backend::Backend, i18n::I18n};
#[cfg(feature = "rustls")]
use parse::conf::LocalCert;
use parse::{
    conf::{create_conf, Settings},
    parse,
};
#[cfg(feature = "tls")]
use rustls::ServerConfig;
#[cfg(feature = "tls")]
use std::fs::File;
#[cfg(feature = "tls")]
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex;
use user::create_account;
/// "cert.pem" "key.pem"
#[cfg(feature = "tls")]
fn load_ssl(localcert: LocalCert) -> Result<Option<ServerConfig>, error::ApplicationError> {
    // load ssl keys
    if localcert.ssl_enable {
        let cert = localcert.cert_file;
        let key = localcert.key_file;
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
        Ok(Some(config))
    } else {
        Ok(None)
    }
}
async fn server_builder(addr: String) -> Result<(), ()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    let session_manager = web::Data::new(Mutex::new(SessionManager::new()));
    let tr = I18n::template_only();
    let bd = web::Data::new(Mutex::new(Backend::new(tr, true)));
    HttpServer::new(move || {
        App::new()
            .app_data(session_manager.clone())
            .app_data(bd.clone())
            .service(welcome)
            .service(favicon)
            .service(web::resource("/{url}/{name}").to(sync_app_no_fail))
            .wrap(middleware::Logger::default())
    })
    .bind(addr)
    .expect("Failed to bind with rustls.")
    .run()
    .await
    .expect("server build error");
    Ok(())
}
#[cfg(feature = "tls")]
async fn server_builder_tls(addr: String, c: rustls::server::ServerConfig) -> Result<(), ()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    let session_manager = web::Data::new(Mutex::new(SessionManager::new()));
    let tr = I18n::template_only();
    let bd = web::Data::new(Mutex::new(Backend::new(tr, true)));
    HttpServer::new(move || {
        App::new()
            .app_data(session_manager.clone())
            .app_data(bd.clone())
            .service(welcome)
            .service(favicon)
            .service(web::resource("/{url}/{name}").to(sync_app_no_fail))
            .wrap(middleware::Logger::default())
    })
    .bind_rustls(addr, c)
    .expect("Failed to bind with rustls.")
    .run()
    .await
    .expect("server build error");
    Ok(())
}
#[actix_web::main]
async fn main() -> Result<(), ()> {
    //cli argument  parse
    let matches = parse();
    // set config path if parsed and write conf settings
    // to path
    let conf_path = Path::new(
        matches
            .value_of("config")
            .expect("Could not parse config from args"),
    );
    create_conf(conf_path);
    // read config file
    let conf = Settings::new().expect("Failed to populate settings from file.");

    // create db if not exist
    let auth_path = conf.paths.auth_db_path;
    create_auth_db(&auth_path).expect("Failed to create auth database.");
    // enter into account manage if subcommand exists,else run server
    if matches.subcommand_name().is_some() {
        if let Err(e) = user_manage(matches, auth_path) {
            eprintln!("Error managing users: {}", e);
            return Err(());
        };
        Ok(())
    } else {
        //    run ankisyncd without any sub-command

        if let Err(e) = create_account(conf.account, auth_path) {
            eprintln!("Error creating account: {}", e);
            return Err(());
        }
        let addr = format!("{}:{}", conf.address.host, conf.address.port);
        #[cfg(feature = "rustls")]
        let lc = conf.localcert;
        #[cfg(feature = "rustls")]
        let enable = lc.ssl_enable;
        #[cfg(feature = "rustls")]
        let tls_conf = match load_ssl(lc) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error while setting up ssl: {}", e);
                return Err(());
            }
        };
        if cfg!(feature = "rustls") {
            #[cfg(feature = "rustls")]
            if enable {
                #[cfg(feature = "rustls")]
                server_builder_tls(addr, tls_conf.unwrap()).await?;
            } else {
                server_builder(addr.clone()).await?;
            }
            return Ok(());
        }
        server_builder(addr).await?;
        Ok(())
    }
}
