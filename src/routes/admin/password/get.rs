use std::fmt::Write;

use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use actix_web_flash_messages::IncomingFlashMessages;

use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};

pub async fn change_password_form(
    session: TypedSession,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        return Ok(see_other("/login"));
    }

    let mut msg_html = String::new();
    for msg in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", msg.content()).unwrap();
    }

    let html_content = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta http-equiv="content-type" content="text/html;">
            <title>Change Password</title>
        </head>
        <body>
            {msg_html}
            <form action="/admin/password" method="post">
                <label for="current_password">Current Password
                <input type="password" placeholder="Current Password" name="current_password">
                </label>
                
                <label for="new_password">New Password
                <input type="password" placeholder="New Password" name="new_password">
                </label>
                
                <label for="confirm_password">Confirm New Password
                <input type="password" placeholder="Confirm New Password" name="confirm_password">
                </label>
                <br>
                <button type="submit">Change Password</button>
            </form>
            <p><a href="/admin/dashboard">Back</a></p>
        </body>
        </html>
    "#
    );

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_content))
}
