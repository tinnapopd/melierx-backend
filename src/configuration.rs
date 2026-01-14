use std::convert::{TryFrom, TryInto};
use std::env;
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;

/// Environment enum to distinguish between local and production settings.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either 'local' or 'production'.",
                other
            )),
        }
    }
}

// Application settings structure.
#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub base_url: String,
    pub hmac_secret: SecretString,
}

/// Database settings structure.
#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub database_name: String,
    pub username: String,
    pub password: SecretString,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .database(&self.database_name)
            .password(self.password.expose_secret())
            .ssl_mode(ssl_mode)
    }
}

/// Email client settings structure.
#[derive(serde::Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: SecretString,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_milliseconds)
    }

    pub fn client(self) -> EmailClient {
        let sender_email =
            self.sender().expect("Invalid sender email address.");
        let timeout = self.timeout();
        let base_url = self
            .base_url
            .parse()
            .expect("Invalid email client base URL");
        EmailClient::new(
            base_url,
            sender_email,
            self.authorization_token,
            timeout,
        )
    }
}

/// Facade settings structure.
#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
    pub redis_uri: SecretString,
}

/// Load the configuration settings from files and environment variables
/// # Returns
/// A Result containing the Settings struct or a ConfigError
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path =
        env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");

    // Detect the running environment, default to 'local'
    let environment: Environment = env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");

    let settings = config::Config::builder()
        // Base config: configuration/base.*
        .add_source(
            config::File::from(configuration_directory.join("base"))
                .required(true),
        )
        // Env-specific override
        .add_source(
            config::File::from(
                configuration_directory.join(environment.as_str()),
            )
            .required(true),
        )
        .add_source(config::Environment::with_prefix("APP").separator("__"))
        // Add in settings from environment variables (with prefix APP and '__' as separator)
        // E.g., `APP_DATABASE__USERNAME` would set `database.username`
        .build()?;

    settings.try_deserialize::<Settings>()
}
