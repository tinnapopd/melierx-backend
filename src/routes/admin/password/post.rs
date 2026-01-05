use actix_web::{HttpResponse, web};
use secrecy::SecretString;

#[derive(serde::Deserialize)]
pub struct FormData {
    pub current_password: SecretString,
    pub new_password: SecretString,
    pub new_password_check: SecretString,
}

pub async fn change_password(
    _orm: web::Form<FormData>,
) -> Result<HttpResponse, actix_web::Error> {
    todo!()
}
