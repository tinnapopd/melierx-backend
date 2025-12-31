use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use uuid::Uuid;

// Query parameters structure for subscription confirmation.
#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

/// Handles the confirmation of a pending subscription.
/// # Arguments
/// * `pool` - A reference to the PostgreSQL connection pool.
/// * `parameters` - The query parameters containing the subscription token.
/// # Returns
/// An HTTP response indicating the result of the confirmation process.
#[tracing::instrument(name = "Confirm a pending subscription", skip(pool, parameters))]
pub async fn confirm(pool: web::Data<PgPool>, parameters: web::Query<Parameters>) -> HttpResponse {
    let id = match get_subscriber_id_from_token(&pool, &parameters.subscription_token).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match id {
        None => HttpResponse::Unauthorized().finish(),
        Some(subscriber_id) => {
            if confirm_subscriber(&pool, subscriber_id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
    }
}

/// Retrieves the subscriber ID associated with the given subscription token.
/// # Arguments
/// * `pool` - A reference to the PostgreSQL connection pool.
/// * `subscription_token` - The subscription token string.
/// # Returns
/// An Option containing the subscriber UUID if found, or None if not found.
#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}

/// Marks the subscriber as confirmed in the database.
/// # Arguments
/// * `pool` - A reference to the PostgreSQL connection pool.
/// * `subscriber_id` - The UUID of the subscriber to be confirmed.
/// # Returns
/// A Result indicating success or failure of the operation.
#[tracing::instrument(name = "Marking subscription as confirmed", skip(subscriber_id, pool))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'confirmed'
        WHERE id = $1
        "#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
