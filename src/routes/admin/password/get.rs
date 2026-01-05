use actix_web::HttpResponse;
use actix_web::http::header::ContentType;

pub async fn change_password_form() -> Result<HttpResponse, actix_web::Error> {
    let html_content = r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta http-equiv="content-type" content="text/html;">
            <title>Change Password</title>
        </head>
        <body>
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
    "#;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_content))
}
