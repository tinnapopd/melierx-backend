use std::io;
use std::net::TcpListener;

use actix_session::SessionMiddleware;
use actix_session::storage::RedisSessionStore;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_flash_messages::storage::CookieMessageStore;
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{admin_dashboard, publish_newsletter, subscribe};
use crate::routes::{change_password, change_password_form};
use crate::routes::{confirm, health_check, home, log_out, login, login_form};

// Application struct representing the running application.
pub struct Application {
    pub port: u16,
    pub server: Server,
}

impl Application {
    /// Build and configure the application.
    /// # Arguments
    /// * `configuration` - The application settings.
    /// # Returns
    /// A Result containing the Application or an io::Error.
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database)
            .await
            .expect("Failed to create database connection pool.");

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
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
        )
        .await?;

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

// Newtype for HMAC secret.
#[derive(Clone)]
pub struct HmacSecret(pub SecretString);

/// Run the HTTP server.
/// # Arguments
/// * `listener` - A TcpListener for incoming connections.
/// * `db_pool` - A PgPool for database connections.
/// * `email_client` - An EmailClient for sending emails.
/// * `base_url` - The base URL of the application.
/// # Returns
/// A Result containing the Server or an io::Error.
async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: SecretString,
    redis_uri: SecretString,
) -> Result<Server, anyhow::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url: web::Data<ApplicationBaseUrl> =
        web::Data::new(ApplicationBaseUrl(base_url));
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(Key::from(
        hmac_secret.expose_secret().as_bytes(),
    ))
    .build();
    let message_framework =
        FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/admin/dashboard", web::get().to(admin_dashboard))
            .route("/admin/password", web::get().to(change_password_form))
            .route("/admin/password", web::post().to(change_password))
            .route("/admin/logout", web::post().to(log_out))
            // Get a pointer copy and attach it to the application state
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(web::Data::new(HmacSecret(hmac_secret.clone())))
    })
    .listen(listener)?
    .run();

    Ok(server)
}

/// Get a connection pool to the database.
/// # Arguments
/// * `configuration` - A reference to the database settings.
/// # Returns
/// A Result containing the PgPool or a sqlx::Error.
pub async fn get_connection_pool(
    configuration: &DatabaseSettings,
) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .connect_with(configuration.connect_options())
        .await
}
