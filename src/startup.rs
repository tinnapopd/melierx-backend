use std::io;
use std::net::TcpListener;
use std::time::Duration;

use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{confirm, health_check, publish_newsletter, subscribe};

// Application struct representing the running application.
pub struct Application {
    pub port: u16,
    pub server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let timeout = configuration.email_client.timeout();
        let base_url = configuration
            .email_client
            .base_url
            .parse()
            .expect("Invalid email client base URL");
        let email_client = EmailClient::new(
            base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url.clone(),
        )?;

        Ok(Self { port, server })
    }

    /// Get the port that the application is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Run the application until stopped.
    pub async fn run_until_stopped(self) -> Result<(), io::Error> {
        self.server.await
    }
}

// Newtype for application base URL.
pub struct ApplicationBaseUrl(pub String);

/// Build and run the HTTP server.
/// # Arguments
/// * `configuration` - A reference to the application settings.
/// # Returns
/// A Result containing the Server or an io::Error.
pub async fn build(configuration: &Settings) -> Result<Server, io::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let timeout = configuration.email_client.timeout();
    let base_url = configuration
        .email_client
        .base_url
        .parse()
        .expect("Invalid email client base URL");
    let email_client = EmailClient::new(
        base_url,
        sender_email,
        configuration.email_client.authorization_token.clone(),
        timeout,
    );

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;
    run(
        listener,
        connection_pool,
        email_client,
        configuration.application.base_url.clone(),
    )
}

/// Run the HTTP server.
/// # Arguments
/// * `listener` - A TcpListener for incoming connections.
/// * `db_pool` - A PgPool for database connections.
/// * `email_client` - An EmailClient for sending emails.
/// * `base_url` - The base URL of the application.
/// # Returns
/// A Result containing the Server or an io::Error.
pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move || {
        App::new()
            // Middleware logger
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            // Get a pointer copy and attach it to the application state
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

/// Get a connection pool to the database.
/// # Arguments
/// * `configuration` - A reference to the database settings.
/// # Returns
/// A `PgPool` instance.
pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}
