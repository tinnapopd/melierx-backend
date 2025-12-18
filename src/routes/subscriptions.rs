use actix_web::{HttpResponse, web};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>) -> HttpResponse {
    println!(
        "Received subscription: name={}, email={}",
        form.name, form.email
    );
    HttpResponse::Ok().finish()
}
