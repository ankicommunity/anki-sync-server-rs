#![forbid(unsafe_code)]
mod db;
mod media;
pub mod parse;
pub mod session;
pub mod sync;
pub mod user;
use self::{
    session::SessionManager,
    sync::{favicon, sync_app, welcome},
    user::{create_auth_db, user_manage},
};
use anki::{backend::Backend,i18n::I18n};
use actix_web::{middleware, web, App, HttpServer};
use config::Config;
use lazy_static::lazy_static;
use parse::{conf::write_conf, parse};
use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::{NoClientAuth, ServerConfig};
use std::io::BufReader;
use std::{env, sync::Mutex};
use std::{fs::File, path::PathBuf};
use std::{path::Path, sync::RwLock};
use user::create_account;
lazy_static! {
    pub static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
}
lazy_static! {
    /// get env ANKISYNCD_ROOT if set as working path where
    /// server data(collections folder) and database(auth.db) reside in
 pub static ref ROOT:PathBuf=match env::var("ANKISYNCD_ROOT") {
        Ok(r)=>Path::new(&r).to_owned(),
        Err(_)=>env::current_dir().unwrap()
    };
}
/// "cert.pem" "key.pem"
fn load_ssl() -> Option<ServerConfig> {
    // load ssl keys
    let settings = SETTINGS.read().unwrap();
    let status = settings.get_bool("localcert.ssl_enable").unwrap();
    if status {
        let cert = settings.get_str("localcert.cert_file").unwrap();
        let key = settings.get_str("localcert.key_file").unwrap();

        let mut config = ServerConfig::new(NoClientAuth::new());
        let cert_file = &mut BufReader::new(File::open(cert).unwrap());
        let key_file = &mut BufReader::new(File::open(key).unwrap());
        let cert_chain = certs(cert_file).unwrap();
        let mut keys = pkcs8_private_keys(key_file).unwrap();
        if keys.is_empty() {
            eprintln!("Could not locate PKCS 8 private keys.");
            std::process::exit(1);
        }
        config.set_single_cert(cert_chain, keys.remove(0)).unwrap();
        Some(config)
    } else {
        None
    }
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    //cli argument  parse
    let matches = parse();
    // set config path if parsed and write conf settings
    // to path
    let conf_path = if let Some(v) = matches.value_of("config") {
        ROOT.join(v)
    } else {
        ROOT.join("Settings.toml")
    };
    write_conf(&conf_path);
    // merge config
    SETTINGS
        .write()
        .unwrap()
        .merge(config::File::from(conf_path))
        .unwrap();

    // create db if not exist
    let settings = SETTINGS.read().unwrap();
    let auth_path = settings.get_str("path.auth_db_path").unwrap();
    create_auth_db(ROOT.join(Path::new(&auth_path).file_name().unwrap())).unwrap();
    // enter into account manage if subcommand exists,else run server
    if matches.subcommand_name().is_some() {
        user_manage(matches);
        Ok(())
    } else {
        //    run ankisyncd without any sub-command

        create_account(&settings);
        let config = load_ssl();
        // parse ip address
        let host = settings.get_str("address.host").unwrap();
        let port = settings.get_str("address.port").unwrap();
        let addr = format!("{}:{}", host, port);

        std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
        env_logger::init();
        let session_manager = web::Data::new(Mutex::new(SessionManager::new()));
        let tr = I18n::template_only();
        let bd = web::Data::new(Mutex::new(Backend::new(tr, true)));
        let s = HttpServer::new(move || {
            App::new()
                .app_data(session_manager.clone())
                .app_data(bd.clone())
                .service(welcome)
                .service(favicon)
                .service(web::resource("/{url}/{name}").to(sync_app))
                .wrap(middleware::Logger::default())
        });
        if let Some(c) = config {
            s.bind_rustls(addr, c)?.run().await
        } else {
            s.bind(addr)?.run().await
        }
    }
}

#[test]
fn test_var() {
    let root = env::var("ANKISYNCD_ROOT");
    println!("{:?}", root);
}
