use crate::helpers::{assert_is_redirect_to, spawn_app};
use uuid::Uuid;

#[actix_web::test]
async fn you_must_be_logged_in_to_change_password() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_change_password().await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[actix_web::test]
async fn you_must_be_logged_in_to_change_your_password() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[actix_web::test]
async fn new_password_fields_must_match() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    // Act - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Act - Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &another_new_password,
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/password");

    // Act - 3 Follow the redirect
    let html_page = app.get_change_password_html().await;

    // Assert - We see an error message
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
        the fields values must match.</i></p>"
    ));
}

#[actix_web::test]
async fn current_password_must_be_valid() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    // Act - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Act - Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &wrong_password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/password");

    // Act - Follow the redirect
    let html_page = app.get_change_password_html().await;

    // Assert
    assert!(
        html_page.contains("<p><i>The current password is incorrect.</i></p>")
    );
}

#[actix_web::test]
async fn changing_password_works() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let response = app.post_login(&login_body).await;

    // Assert Login worked
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act - Change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert Change password worked
    assert_is_redirect_to(&response, "/admin/password");

    // Act - Follow the redirect
    let html_page = app.get_change_password_html().await;

    // Assert Flash message is displayed
    assert!(
        html_page.contains("<p><i>Your password has been changed.</i></p>")
    );

    // Act - Logout
    let response = app.post_logout().await;

    // Assert Logout worked
    assert_is_redirect_to(&response, "/login");

    // Act - Follow the redirect
    let html_page = app.get_login_html().await;
    assert!(
        html_page.contains("<p><i>You have successfully logged out.</i></p>")
    );

    // Act - Login with new password
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &new_password,
    });
    let response = app.post_login(&login_body).await;

    // Assert Login with new password worked
    assert_is_redirect_to(&response, "/admin/dashboard");
}
