use actix_web::HttpResponse;

/// A simple health check endpoint that returns HTTP 200 OK.
/// This can be used by monitoring systems to verify that the application is running.
/// # Returns
/// An HTTP response with status 200 OK.
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
