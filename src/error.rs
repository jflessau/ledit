use frankenstein::api_params::SendMessageParamsBuilderError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LeditError {
    #[error("Send message error")]
    SendMessage,
    #[error("Database error")]
    Database,
    #[error("Failed to find random user")]
    RndUser,
    // #[error("Unknown error")]
    // Unknown,
}

impl From<SendMessageParamsBuilderError> for LeditError {
    fn from(_err: SendMessageParamsBuilderError) -> Self { LeditError::SendMessage }
}

impl From<sqlx::Error> for LeditError {
    fn from(_err: sqlx::Error) -> Self { LeditError::Database }
}

impl From<sqlx::migrate::MigrateError> for LeditError {
    fn from(_err: sqlx::migrate::MigrateError) -> Self { LeditError::Database }
}
