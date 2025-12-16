use crate::AppState;
use crate::db::user::{create, has_with_email};
use actix_web::{HttpResponse, Responder, post, web};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
pub struct SignUpRequest {
    pub email: String,
    pub password: String,
    pub firstname: String,
    pub lastname: String,
}

#[post("/auth/sign-up")]
pub async fn sign_up(state: web::Data<AppState>, data: web::Json<SignUpRequest>) -> impl Responder {
    let db = state.db.lock().unwrap();
    if has_with_email(&db, &data.email).await {
        return HttpResponse::UnprocessableEntity()
            .json(json!({ "status": "error", "message": "Email already exists" }).to_string());
    }
    create(&db, &data).await;

    HttpResponse::Ok()
        .json(json!({ "status": "success", "message": "Account created successfully" }).to_string())
}

#[post("/auth/sign-in")]
pub async fn sign_in() -> impl Responder {
    "Sign In"
}
