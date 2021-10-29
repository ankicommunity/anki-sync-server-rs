use actix_web::{middleware, web, App, HttpServer};
use ankisyncd::{
    envconfig,
    session::SessionManager,
    sync::{favicon, sync_app, welcome},
};
use env_logger;
use std::sync::Mutex;
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
# following fields will be added into auth.db if not empty
username=""
password=""
    "#;
    if !p.exists() {
        fs::write(&p, content).unwrap();
    }
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    setting_exist();
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    //reference py ver open col
    let session_manager = web::Data::new(Mutex::new(SessionManager::new()));
    HttpServer::new(move || {
        App::new()
            .app_data(session_manager.clone())
            .service(welcome)
            .service(favicon)
            .service(web::resource("/{url}/{name}").to(sync_app))
            .wrap(middleware::Logger::default())
    })
    .bind(envconfig::addr())?
    .run()
    .await
}
