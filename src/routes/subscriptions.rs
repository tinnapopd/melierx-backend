use std::error;
use std::fmt;
use std::iter;

use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use anyhow::Context;
use chrono::Utc;
use rand::{Rng, distr::Alphanumeric};
use sqlx::Executor;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;

/// Form data structure for new subscriber.
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

// Error type for subscription process.
#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("0")]
    ValidationError(String),
    #[error("transparent")]
    UnexpectedError(#[from] anyhow::Error),
}

impl fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// Error type for storing subscription token.
pub struct StoreTokenError(sqlx::Error);

impl error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.0)
    }
}

impl fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "A database error was encountered while trying to store a subscription token."
        )
    }
}

/// Handles the subscription of a new user.
/// # Arguments
/// * `form` - The form data containing subscriber details.
/// * `pool` - A reference to the PostgreSQL connection pool.
/// * `email_client` - A reference to the EmailClient for sending emails.
/// * `base_url` - The base URL of the application for constructing confirmation links.
/// # Returns
/// An HTTP response indicating the result of the subscription process.
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to start a new database transaction")?;
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store subscription token in the database")?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction.")?;
    send_confirmation_email(
        &email_client,
        &new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;
    Ok(HttpResponse::Ok().finish())
}

/// Saves the new subscriber details in the database.
/// # Arguments
/// * `transaction` - A mutable reference to the database transaction.
/// * `new_subscriber` - A reference to the NewSubscriber struct containing subscriber details.
/// # Returns
/// The UUID of the newly created subscriber.
#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now().naive_utc()
    );
    transaction.execute(query).await?;
    Ok(subscriber_id)
}

/// Sends a confirmation email to the new subscriber.
/// # Arguments
/// * `email_client` - A reference to the EmailClient for sending emails.
/// * `new_subscriber` - A reference to the NewSubscriber struct containing subscriber details.
/// * `base_url` - The base URL of the application for constructing the confirmation link.
/// * `subscription_token` - The subscription token to include in the confirmation link.
/// # Returns
/// A Result indicating success or failure of the email sending operation.
#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let plain_body = format!(
        "Welcome to our melierx website!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our melierx website!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

/// Stores the subscription token in the database associated with the subscriber ID.
/// # Arguments
/// * `transaction` - A mutable reference to the database transaction.
/// * `subscriber_id` - The UUID of the subscriber.
/// * `subscription_token` - The subscription token string to store.
/// # Returns
/// A Result indicating success or failure of the operation.
#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(transaction, subscription_token)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query).await.map_err(StoreTokenError)?;
    Ok(())
}

/// Generates a random subscription token.
/// # Returns
/// A randomly generated subscription token string.
fn generate_subscription_token() -> String {
    let mut rng = rand::rng();
    iter::repeat_with(|| rng.sample(Alphanumeric))
        .take(25)
        .map(char::from)
        .collect()
}

/// Formats the error chain for debugging purposes.
/// # Arguments
/// * `e` - A reference to the error to format.
/// * `f` - A mutable reference to the formatter.
/// # Returns
/// A fmt::Result indicating success or failure of the formatting operation.
fn error_chain_fmt(e: &impl error::Error, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        write!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
