use std::fmt;

use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::{HttpResponse, web};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;

use crate::authentication::AuthError;
use crate::authentication::{Credentials, validate_credentials};
use crate::routes::error_chain_fmt;
use crate::startup::HmacSecret;

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong.")]
    UnexpectedError(#[from] anyhow::Error),
}

impl fmt::Debug for LoginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(serde::Deserialize)]
pub struct FormData {
    pub username: String,
    pub password: SecretString,
}

#[tracing::instrument(
    skip(pool, form, secret)
    fields(
        username = tracing::field::Empty,
        user_id = tracing::field::Empty,
    )
)]
pub async fn login(
    pool: web::Data<PgPool>,
    form: web::Form<FormData>,
    secret: web::Data<HmacSecret>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current()
        .record("username", tracing::field::display(&credentials.username));
    match validate_credentials(&pool, credentials).await {
        Ok(user_id) => {
            tracing::Span::current()
                .record("user_id", tracing::field::display(&user_id));
            let result = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish();
            Ok(result)
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => {
                    LoginError::AuthError(e.into())
                }
                AuthError::UnexpectedError(_) => {
                    LoginError::UnexpectedError(e.into())
                }
            };
            let query_string =
                format!("error={}", urlencoding::Encoded::new(e.to_string()));
            let hmac_tag = {
                let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
                    secret.0.expose_secret().as_bytes(),
                )
                .unwrap();
                mac.update(query_string.as_bytes());
                mac.finalize().into_bytes()
            };
            let response = HttpResponse::SeeOther()
                .insert_header((
                    LOCATION,
                    format!("/login?{}&tag={:x}", query_string, hmac_tag),
                ))
                .finish();

            Err(InternalError::from_response(e, response))
        }
    }
}
