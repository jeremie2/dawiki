use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::Error as SqlxError;
use tera::Error as TeraError;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
    Conflict(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            AppError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            AppError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
            AppError::Conflict(message) => (StatusCode::CONFLICT, message),
        };

        (status, message).into_response()
    }
}

impl From<SqlxError> for AppError {
    fn from(error: SqlxError) -> Self {
        eprintln!("Errore del database: {error}");
        AppError::Internal(
            "Si è verificato un errore durante l'accesso al database.".to_string(),
        )
    }
}

impl From<TeraError> for AppError {
    fn from(error: TeraError) -> Self {
        eprintln!("Errore nel rendering del template: {error}");
        AppError::Internal(
            "Si è verificato un errore durante la generazione della pagina.".to_string(),
        )
    }
}
