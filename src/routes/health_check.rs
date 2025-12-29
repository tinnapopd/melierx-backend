use actix_web::{HttpResponse, Responder};

/// A simple health check endpoint that returns HTTP 200 OK.
/// This can be used by monitoring systems to verify that the application is running.
/// # Returns
/// An HTTP 200 OK response.
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}
