pub mod app_config;
pub mod config;
mod db;
mod error;
pub mod parse_args;
pub mod response;
pub mod routes;
pub mod user;
#[cfg(feature = "account")]
use clap::Parser;
pub mod request;
#[cfg(feature = "account")]
use crate::app_config::run;
pub use crate::config::Config;
pub use crate::error::ApplicationError;
#[cfg(feature = "account")]
use crate::user::create_auth_db;
/// It allow account section to exist in config file ,so the feature `account` need be enabled.
///
/// If config argument is absent in arg parsing ,then ./ankisyncd.toml will be used.
#[cfg(feature = "account")]
pub async fn server_run_account() -> Result<(), ApplicationError> {
    use std::path::Path;

    use user::create_user_from_conf;

    let matches = parse_args::Arg::parse();
    // Display config
    if matches.default {
        let default_yaml = Config::default().to_string().expect("Failed to serialize.");
        println!("{}", default_yaml);
        return Ok(());
    }
    // read config file if needed
    // use the conf file passed by argument,else use one which is located in .
    let conf = if matches.config.as_ref().is_some() {
        match parse_args::config_from_arguments(&matches) {
            Ok(c) => c,
            Err(_) => {
                return Err(ApplicationError::ParseConfig(
                    "Error while getting configuration".into(),
                ));
            }
        }
    } else {
        let p = Path::new("./ankisyncd.toml");
        if p.exists() {
            match Config::from_file(p) {
                Ok(c) => c,
                Err(_) => {
                    return Err(ApplicationError::ParseConfig(
                        "Error while getting configuration".into(),
                    ));
                }
            }
        } else {
            return Err(ApplicationError::ParseConfig(format!(
                "file {} not found indicated in its path",
                p.display()
            )));
        }
    };
    // create db if not exist。
    // add to db if account is not empty
    let auth_path = conf.auth_db_path();
    create_auth_db(&auth_path).expect("Failed to create auth database.");
    #[cfg(feature = "account")]
    if let Some(acnt) = conf.clone().account {
        create_user_from_conf(acnt, &auth_path);
    }
    // Manage account if needed, exit if this is the case
    if let Some(cmd) = matches.cmd.as_ref() {
        parse_args::manage_user(&cmd, &auth_path);
        return Ok(());
    }
    run(&conf).await;
    Ok(())
}
