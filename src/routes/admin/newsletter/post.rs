use actix_web::{HttpResponse, web};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::authentication::UserId;
use crate::idempotency::{IdempotencyKey, save_response};
use crate::idempotency::{NextAction, try_processing};
use crate::utils::{e400, e500, see_other};

/// Form data for publishing a newsletter issue.
#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

/// Handle the publishing of a newsletter issue.
/// # Arguments
/// * `pool` - The database connection pool.
/// * `form` - The form data containing the newsletter issue details.
/// * `user_id` - The ID of the authenticated user.
/// # Returns
/// A Result containing an HttpResponse or an actix_web::Error.
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip_all,
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    pool: web::Data<PgPool>,
    form: web::Form<FormData>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey =
        idempotency_key.try_into().map_err(e400)?;

    let mut transaction =
        match try_processing(&pool, &idempotency_key, *user_id)
            .await
            .map_err(e500)?
        {
            NextAction::StartProcessing(t) => t,
            NextAction::ReturnSavedResponse(saved_response) => {
                success_message().send();
                return Ok(saved_response);
            }
        };

    let issue_id = insert_newsletter_issue(
        &mut transaction,
        &title,
        &text_content,
        &html_content,
    )
    .await
    .context("Failed to insert newsletter issue")
    .map_err(e500)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    let response = see_other("/admin/newsletters");
    let response =
        save_response(transaction, &idempotency_key, *user_id, response)
            .await
            .map_err(e500)?;
    success_message().send();
    Ok(response)
}

/// Insert a newsletter issue into the database.
/// # Arguments
/// * `transaction` - The database transaction.
/// * `title` - The title of the newsletter issue.
/// * `text_content` - The plain text content of the newsletter issue.
/// * `html_content` - The HTML content of the newsletter issue.
/// # Returns
/// A Result containing the UUID of the inserted newsletter issue or a sqlx::Error.
#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO issues (issue_id, title, text_content, html_content, published_at)
        VALUES ($1, $2, $3, $4, now())
        "#,
        issue_id,
        title,
        text_content,
        html_content
    )
    .execute(transaction.as_mut())
    .await?;
    Ok(issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (issue_id, subscriber_email)
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        issue_id
    )
    .execute(transaction.as_mut())
    .await?;
    Ok(())
}

/// Create a flash message indicating successful publication of the newsletter issue.
/// # Returns
/// A FlashMessage indicating success.
fn success_message() -> FlashMessage {
    FlashMessage::info(
        "The newsletter issue has been accepted - \
    emails will go out shortly.",
    )
}
