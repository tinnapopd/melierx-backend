use std::fmt;

use actix_web::HttpResponse;
use actix_web::http::header::LOCATION;

/// Convert any error into an Internal Server Error actix_web::Error.
/// # Arguments
/// * `e` - The error to convert.
/// # Returns
/// An actix_web::Error representing an Internal Server Error.
pub fn e500<T>(e: T) -> actix_web::Error
where
    T: fmt::Debug + fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

/// Convert any error into a Bad Request actix_web::Error.
/// # Arguments
/// * `e` - The error to convert.
/// # Returns
/// An actix_web::Error representing a Bad Request.
pub fn e400<T: fmt::Debug + fmt::Display>(e: T) -> actix_web::Error
where
    T: fmt::Debug + fmt::Display + 'static,
{
    actix_web::error::ErrorBadRequest(e)
}

/// Create a See Other HttpResponse redirecting to the specified location.
/// # Arguments
/// * `location` - The URL to redirect to.
/// # Returns
/// An HttpResponse with status code 303 See Other.
pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}
