use std::fmt::Write;

use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use actix_web_flash_messages::IncomingFlashMessages;

/// Handler to serve the login form.
/// Arguments:
/// - `flash_messages`: Incoming flash messages to be displayed on the login page.
/// Returns:
/// - `HttpResponse`: The HTTP response containing the login form HTML.
pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut message_html = String::new();
    for message in flash_messages.iter() {
        writeln!(message_html, "<p><i>{}</i></p>", message.content()).unwrap();
    }

    let html_content = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8">
            <title>Login</title>
        </head>
        <body>
            {message_html}
            <form action="/login" method="post">
                <label>Username
                    <input
                        type="text"
                        placeholder="Enter Username"
                        name="username"
                    >
                </label>
                <label>Password
                    <input
                        type="password"
                        placeholder="Enter Password"
                        name="password"
                    >
                </label>
                <button type="submit">Login</button>
            </form>
        </body>
        </html>
    "#,
    );
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_content)
}
