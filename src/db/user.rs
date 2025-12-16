use crate::controllers::auth::SignUpRequest;
use sqlx::postgres::PgPool;

pub async fn has_with_email(db: &PgPool, email: &str) -> bool {
    sqlx::query!("SELECT 1 FROM users WHERE email = $1", email)
        .fetch_optional(db)
        .await
        .is_some()
}

pub async fn create(db: &PgPool, user: &SignUpRequest) -> bool {
    let hashed_password = bcrypt::hash(&user.password, bcrypt::DEFAULT_COST).unwrap();
    sqlx::query!(
        "INSERT INTO users (email, password, firstname, lastname) VALUES ($1, $2, $3, $4)",
        &user.email,
        &hashed_password,
        &user.firstname,
        &user.lastname
    )
    .execute(db)
    .await
    .is_ok()
}
