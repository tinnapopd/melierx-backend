use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};

/// Handle user logout by clearing the session and redirecting to the login page.
/// Sends a flash message confirming the logout.
/// # Arguments
/// * `session` - The current user session.
/// # Returns
/// * `HttpResponse` - A redirection response to the login page.
pub async fn log_out(
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        Ok(see_other("/login"))
    } else {
        session.log_out();
        FlashMessage::info("You have successfully logged out.").send();
        Ok(see_other("/login"))
    }
}
