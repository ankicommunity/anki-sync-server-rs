#![forbid(unsafe_code)]
mod db;
pub mod envconfig;
mod media;
pub mod session;
pub mod sync;
pub mod user;
use self::{
    session::SessionManager,
    sync::{favicon, sync_app, welcome},
    user::{add_user, create_auth_db, user_list, user_manage},
};
use crate::envconfig::env_variables;
use actix_web::{middleware, web, App, HttpServer};
use anki::sync::server::LocalServer;
use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::{NoClientAuth, ServerConfig};
use std::cell::Cell;
use std::fs::File;
use std::io::BufReader;
use std::{env, sync::Mutex};
use std::{fs, path::Path};
/// generate Setting.toml if not exist
fn setting_exist() {
    let p = Path::new("Settings.toml");
    let content = r#"
host="0.0.0.0"
port = "27701"
data_root = "./collections"
base_url = "/sync/"
base_media_url = "/msync/"
auth_db_path = "./auth.db"
session_db_path = "./session.db"
# following fields will be added 
#into auth.db if not empty,and two fields must not be empty
username=""
userpassword=""
# embeded encrypted http /https credential if in Intranet
# true to enable ssl or false
ssl_enable="false"
cert_file=""
key_file=""

    "#;
    if !p.exists() {
        fs::write(&p, content).unwrap();
    }
}
/// "cert.pem" "key.pem"
fn load_ssl() -> Option<ServerConfig> {
    // load ssl keys
    let status = &env_variables()["ssl_enable"];
    if status == "true" {
        let cert = &env_variables()["cert_file"];
        let key = &env_variables()["key_file"];

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
    setting_exist();
    // create db if not exist
    create_auth_db().unwrap();
    // env vars parse
    // enter into user management if user flag is enabled
    let v_args = env::args().into_iter().collect::<Vec<_>>();
    let len = v_args.len() as u8;
    if len == 1 {
        // insert record into db if username is not empty,
        let name = env_variables().remove("username").unwrap();
        let pass = env_variables().remove("userpassword").unwrap();
        // insert record into db if user if not empty,
        // else start server
        if !name.is_empty() {
            if !pass.is_empty() {
                // look up in db to check if user exist
                let user_list = user_list().unwrap();
                // if not insert into db
                if user_list.is_none() {
                    add_user(&[name, pass]).unwrap();
                }
            } else {
                panic!("user fields are not allowed for empty")
            }
        }
        let config = load_ssl();

        std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
        env_logger::init();
        //reference py ver open col
        let session_manager = web::Data::new(Mutex::new(SessionManager::new()));
        let server: web::Data<Mutex<Option<LocalServer>>> = web::Data::new(Mutex::new(None));
        let s = HttpServer::new(move || {
            App::new()
                .app_data(session_manager.clone())
                .app_data(server.clone())
                .service(welcome)
                .service(favicon)
                .service(web::resource("/{url}/{name}").to(sync_app))
                .wrap(middleware::Logger::default())
        });
        if let Some(c) = config {
            s.bind_rustls(envconfig::addr(), c)?.run().await
        } else {
            s.bind(envconfig::addr())?.run().await
        }
    } else if len == 2 {
        let var = v_args.get(1).unwrap();
        if var == "U" {
            // into diff account ops according to diff
            // choice ...
            user_manage();
        } else {
            println!("incorrect flag,use capital U as flag");
        }
        Ok(())
    } else {
        println!("incorrect flag,use capital U as flag");
        Ok(())
    }
}
