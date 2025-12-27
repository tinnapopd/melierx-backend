use actix_web::{HttpResponse, web};

// Public Structs
#[derive(serde::Deserialize)]
pub struct Parameters {
    pub subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(_parameter))]
pub async fn confirm(_parameter: web::Query<Parameters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
