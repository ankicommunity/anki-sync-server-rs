use actix_web::{middleware, web, App, HttpServer};
use ankisyncd::{
    envconfig,
    session::SessionManager,
    sync::{favicon, sync_app, welcome},
};

use env_logger;
use std::sync::Mutex;
#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
