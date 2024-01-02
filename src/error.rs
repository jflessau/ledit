use frankenstein::api_params::SendMessageParamsBuilderError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LeditError {
    #[error("Frankenstein Error: {0}")]
    SendMessageParamsBuilder(#[from] SendMessageParamsBuilderError),

    #[error("Failed to find random user")]
    Frankenstein(String),

    #[error("Sqlx Error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("SQLx Migrate Error: {0}")]
    SqlxMigrate(#[from] sqlx::migrate::MigrateError),

    #[error("Failed to find random user")]
    RndUser,
}

impl From<frankenstein::Error> for LeditError {
    fn from(err: frankenstein::Error) -> Self {
        LeditError::Frankenstein(format!("{:?}", err))
    }
}
