mod collecction;
pub mod config;
mod db;
mod error;
mod media;
pub mod parse_args;
pub mod server;
pub mod session;
pub mod sync;
pub mod user;
#[cfg(feature = "tls")]
use self::server::{load_ssl, server_builder_tls};
use self::{config::Config, server::server_builder, user::create_auth_db};
use clap::Parser;
use lazy_static::lazy_static;
use std::env;
use user::add_user;

lazy_static! {
    static ref USERNAME: String = env::var("ANKISYNCD_USERNAME").unwrap_or_else(|_| "".to_string());
    static ref PASSWORD: String = env::var("ANKISYNCD_PASSWORD").unwrap_or_else(|_| "".to_string());
}

#[actix_web::main]
async fn main() -> Result<(), ()> {
    let matches = parse_args::Arg::parse();
    // Display config
    if matches.default {
        let default_yaml = Config::default().to_string().expect("Failed to serialize.");
        println!("{}", default_yaml);
        return Ok(());
    }
    // read config file if needed
    let conf = match parse_args::config_from_arguments(&matches) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error while getting configuration: {}", e);
            return Err(());
        }
    };
    // create db if not exist
    let auth_path = conf.auth_db_path();
    create_auth_db(&auth_path).expect("Failed to create auth database.");

    // Manage account if needed, exit if this is the case
    if !USERNAME.is_empty() && !PASSWORD.is_empty() {
        add_user(&[USERNAME.to_string(), PASSWORD.to_string()], &auth_path)
            .expect("adding user from env vars fail");
    }
    if let Some(cmd) = matches.cmd.as_ref() {
        parse_args::manage_user(cmd, &auth_path);
        return Ok(());
    }
    #[cfg(feature = "tls")]
    if cfg!(feature = "tls") {
        if conf.encryption_enabled() {
            let tls_conf = match load_ssl(conf.encryption_config().unwrap()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error while setting up ssl: {}", e);
                    return Err(());
                }
            };
            server_builder_tls(&conf, tls_conf).await;
            return Ok(());
        }
    } else {
        if conf.encryption_enabled() {
            eprintln!("TLS encryption is enabled but will be ignored as encryption support was not built in the binary.");
        }
    }
    server_builder(&conf).await;
    Ok(())
}
