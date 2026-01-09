use actix_web::HttpResponse;
use actix_web::http::header::ContentType;

/// Handler for the home page
/// Returns the contents of the home.html file as an HTTP response.
pub async fn home() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("home.html"))
}
