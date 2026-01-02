use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

// Error type for authentication failures.
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

// Basic authentication credentials structure.
pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

/// Validate the provided credentials against the database.
/// Returns the user ID if authentication is successful.
/// # Arguments
/// * `pool` - A reference to the PostgreSQL connection pool.
/// * `credentials` - The credentials to validate.
/// # Returns
/// A Result containing the user ID if successful, or an AuthError otherwise.
#[tracing::instrument(name = "Validate credentials", skip(pool, credentials))]
pub async fn validate_credentials(
    pool: &PgPool,
    credentials: Credentials,
) -> Result<Uuid, AuthError> {
    let mut user_id: Option<Uuid> = None;
    let mut expected_password_hash = SecretString::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string()
            .into_boxed_str(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(pool, &credentials.username).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username."))
        .map_err(AuthError::InvalidCredentials)
}

/// Verify the provided password against the expected password hash.
/// # Arguments
/// * `expected_password_hash` - The expected password hash.
/// * `password_candidate` - The password candidate to verify.
/// # Returns
/// A Result indicating whether the verification was successful or an AuthError otherwise.
#[tracing::instrument(
    name = "Validate credentials",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: SecretString,
    password_candidate: SecretString,
) -> Result<(), AuthError> {
    let expected_password_hash =
        PasswordHash::new(expected_password_hash.expose_secret())
            .context("Failed to parse hash in PHC string format.")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

/// Retrieve stored credentials for a given username from the database.
/// # Arguments
/// * `pool` - A reference to the PostgreSQL connection pool.
/// * `username` - The username whose credentials are to be retrieved.
/// # Returns
/// A Result containing an Option with the user ID and password hash if found, or an anyhow::Error otherwise.
#[tracing::instrument(name = "Get stored credentials", skip(pool, username))]
async fn get_stored_credentials(
    pool: &PgPool,
    username: &str,
) -> Result<Option<(Uuid, SecretString)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| {
        (
            row.user_id,
            SecretString::new(row.password_hash.into_boxed_str()),
        )
    });

    Ok(row)
}
