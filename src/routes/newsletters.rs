use std::fmt;

use actix_web::http::header::{HeaderMap, HeaderValue};
use actix_web::http::{StatusCode, header};
use actix_web::{HttpRequest, HttpResponse, ResponseError, web};
use anyhow::Context;
use base64::Engine;
use base64::engine::general_purpose;
use secrecy::SecretString;
use sqlx::PgPool;

use crate::authentication::{AuthError, Credentials, validate_credentials};
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl fmt::Debug for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value =
                    HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(
    name = "Publish newsletter",
    skip(pool, email_client, body, request)
    fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub async fn publish_newsletter(
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    body: web::Json<BodyData>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credential = basic_authentication(request.headers())
        .map_err(PublishError::AuthError)?;
    tracing::Span::current()
        .record("username", &tracing::field::display(&credential.username));

    let user_id =
        validate_credentials(&pool, credential)
            .await
            .map_err(|e| match e {
                AuthError::InvalidCredentials(_) => {
                    PublishError::AuthError(e.into())
                }
                AuthError::UnexpectedError(_) => {
                    PublishError::UnexpectedError(e.into())
                }
            })?;

    tracing::Span::current()
        .record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to send newsletter to {}",
                            &subscriber.email
                        )
                    })?;
            }
            Err(e) => {
                tracing::warn!(error.cause_chain = ?e, "Skipping a confirmed subscriber due to invalid email");
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(e) => Err(anyhow::anyhow!(e)),
    })
    .collect();

    Ok(confirmed_subscribers)
}

fn basic_authentication(
    headers: &HeaderMap,
) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header is missing")?
        .to_str()
        .context("The 'Authorization' header is not a valid UTF-8 string.")?;

    let base64_encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The 'Authorization' header is not a Basic authentication.")?;

    let decoded_bytes = general_purpose::STANDARD
        .decode(base64_encoded_segment)
        .context("Failed to decode base64 credentials.")?;

    let decoded_credentials = String::from_utf8(decoded_bytes).context(
        "The decoded 'Authorization' header is not a valid UTF-8 string.",
    )?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "A username must be provided in 'Authorization' header."
            )
        })?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "A password must be provided in 'Authorization' header."
            )
        })?
        .to_string();

    Ok(Credentials {
        username,
        password: SecretString::new(password.into_boxed_str()),
    })
}
