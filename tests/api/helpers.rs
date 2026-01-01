use std::sync::LazyLock;
use std::{env, io};

use actix_web::rt;
use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use linkify::{LinkFinder, LinkKind};
use reqwest::{Client, Response, Url};
use secrecy::SecretString;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::{MockServer, Request};

use melierx_backend::configuration::{DatabaseSettings, get_configuration};
use melierx_backend::startup::{Application, get_connection_pool};
use melierx_backend::telemetry::{get_subscriber, init_subscriber};

// Ensure that the tracing stack is only initialized once
static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if env::var("TEST_LOG").is_ok() {
        let subscriber =
            get_subscriber(subscriber_name, default_filter_level, io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber =
            get_subscriber(subscriber_name, default_filter_level, io::sink);
        init_subscriber(subscriber);
    };
});

// Structure representing the test application.
pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
}

impl TestApp {
    /// Send a POST request to the subscriptions endpoint
    pub async fn post_subscriptions(&self, body: String) -> Response {
        Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// Extract the confirmation links from the email request
    pub fn get_confirmation_links(
        &self,
        email_request: &Request,
    ) -> ConfirmationLinks {
        let body: serde_json::Value =
            serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = Url::parse(&raw_link).unwrap();

            // Make sure we don't call random APIs during tests
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> Response {
        Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(
                &self.test_user.username,
                Some(&self.test_user.password),
            )
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

// Structure representing confirmation links extracted from an email.
pub struct ConfirmationLinks {
    pub html: Url,
    pub plain_text: Url,
}

pub struct TestUser {
    user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            r#"
            INSERT INTO users (user_id, username, password_hash)
            VALUES ($1, $2, $3)
            "#,
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

/// Spawns the application and returns its address and a database connection pool.
/// # Returns
/// A `TestApp` instance containing the application address, port, database connection pool,
/// and email server.
pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);

    // Launch a mock email server (PostMark equivalent)
    let email_server = MockServer::start().await;

    // Randomize configuration to avoid conflicts
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        // Use random OS port
        c.application.port = 0;
        // Use the mock email server
        c.email_client.base_url = email_server.uri();
        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    // Launch the application as a background task
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");

    let application_port = application.port();
    let _ = rt::spawn(application.run_until_stopped());

    let test_app = TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&configuration.database)
            .await
            .expect("Failed to connect to the database."),
        email_server,
        test_user: TestUser::generate(),
    };
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
}

/// Configures the database by creating it and running migrations.
/// # Arguments
/// * `config` - A reference to the `DatabaseSettings` containing the database configuration.
/// # Returns
/// A `PgPool` instance connected to the configured database.
pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: SecretString::new(Box::from("postgres")),
        ..config.clone()
    };

    let mut connection =
        PgConnection::connect_with(&maintenance_settings.connection_options())
            .await
            .expect("Failed to connect to Postgres");
    connection
        .execute(
            format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str(),
        )
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.connection_options())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}
