use actix_web::{HttpResponse, Responder};

// Public Functions
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}
