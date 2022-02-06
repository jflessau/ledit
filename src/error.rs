use frankenstein::api_params::SendMessageParamsBuilderError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LeditError {
    #[error("Send message error")]
    SendMessageError,
    #[error("Database error")]
    DatabaseError,
    // #[error("Unknown error")]
    // Unknown,
}

impl From<SendMessageParamsBuilderError> for LeditError {
    fn from(_err: SendMessageParamsBuilderError) -> Self { LeditError::SendMessageError }
}

impl From<sqlx::Error> for LeditError {
    fn from(_err: sqlx::Error) -> Self { LeditError::DatabaseError }
}

impl From<sqlx::migrate::MigrateError> for LeditError {
    fn from(_err: sqlx::migrate::MigrateError) -> Self { LeditError::DatabaseError }
}
