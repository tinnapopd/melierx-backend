use std::collections::HashSet;

use reqwest::header::HeaderValue;

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[actix_web::test]
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act 1 - Try to login with random credentials
    let response = app
        .post_login(&serde_json::json!({
            "username": "random-username",
            "password": "random-password"
        }))
        .await;

    // Assert 1
    assert_is_redirect_to(&response, "/login");

    // Act 2 - Follow the redirect to the login page
    let html_page = app.get_login_html().await;

    // Assert 2
    assert!(html_page.contains(r#"<p><i>Authentication failed.</i></p>"#));

    // Act 3 - Request the login page again
    let html_page = app.get_login_html().await;

    // Assert 3
    assert!(!html_page.contains(r#"<p><i>Authentication failed.</i></p>"#));
}

#[actix_web::test]
async fn redirect_to_admin_dashboard_after_successful_login() {
    // Arrange
    let app = spawn_app().await;

    // Act - Login
    let response = app
        .post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password
        }))
        .await;

    // Assert - Redirect to admin dashboard
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act Follow the redirect to the admin dashboard
    let html_page = app.get_admin_dashboard_html().await;

    // Assert - We see the admin dashboard page
    assert!(
        html_page.contains(&format!("Welcome {}", &app.test_user.username))
    );
}
