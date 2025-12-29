use std::io;

use melierx_backend::configuration::get_configuration;
use melierx_backend::startup::Application;
use melierx_backend::telemetry::{get_subscriber, init_subscriber};

#[actix_web::main]
async fn main() -> io::Result<()> {
    let subscriber = get_subscriber("melierx_backend".into(), "info".into(), || io::stdout());
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
