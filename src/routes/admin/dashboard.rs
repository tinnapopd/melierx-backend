use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, web};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::authentication::UserId;
use crate::utils::e500;

pub async fn admin_dashboard(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(&pool, *user_id).await.map_err(e500)?;

    let html_content = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta http-equiv="Content-Type" content="text/html; charset=UTF-8">
            <title>Admin Dashboard</title>
        </head>
        <body>
            <h1>Welcome {username}!</h1>
            <p>Available actions:</p>
            <ol>
                <li><a href="/admin/password">Change password</a></li>
                <li>
                    <form name="logoutForm" action="/admin/logout" method="post">
                        <input type="submit" value="Logout">
                    </form>
                </li>
            </ol>
        </body>
        </html>
    "#
    );

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_content))
}

#[tracing::instrument(name = "Get username from user_id", skip(pool))]
pub async fn get_username(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to get username.")?;

    Ok(row.username)
}
