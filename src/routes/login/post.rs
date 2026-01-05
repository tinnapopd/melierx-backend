use std::fmt;

use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::{HttpResponse, web};
use actix_web_flash_messages::FlashMessage;
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;

use crate::authentication::AuthError;
use crate::authentication::{Credentials, validate_credentials};
use crate::routes::error_chain_fmt;
use crate::session_state::TypedSession;
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
    skip(pool, session, form)
    fields(
        username = tracing::field::Empty,
        user_id = tracing::field::Empty,
    )
)]
pub async fn login(
    pool: web::Data<PgPool>,
    session: TypedSession,
    form: web::Form<FormData>,
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
            session.renew();
            session.insert_user_id(user_id).map_err(|e| {
                login_redirect(LoginError::UnexpectedError(e.into()))
            })?;
            let result = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/admin/dashboard"))
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
            Err(login_redirect(e))
        }
    }
}

fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();
    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();

    InternalError::from_response(e, response)
}
