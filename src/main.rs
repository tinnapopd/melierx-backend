use actix_web::{App, HttpServer, web};
use std::sync::Mutex;
mod controllers;
mod db;

pub struct AppState {
    db: Mutex<sqlx::postgres::PgPool>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let state = web::Data::new(AppState {
        db: Mutex::new(
            sqlx::postgres::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
                .await
                .unwrap(),
        ),
    });
    HttpServer::new(move || {
        App::new()
            .service(controllers::auth::sign_up)
            .service(controllers::auth::sign_in)
            .service(controllers::me::get_profile)
            .service(controllers::me::update_profile)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
