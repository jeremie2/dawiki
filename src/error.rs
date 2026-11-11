use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Serialize;
use tera::{Context, Tera};

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    NotFound(String),
    Conflict(String),
    Internal(String),
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "Richiesta non valida",
            AppError::NotFound(_) => "Pagina non trovata",
            AppError::Conflict(_) => "Pagina già esistente",
            AppError::Internal(_) => "Errore interno",
        }
    }

    fn message(&self) -> &str {
        match self {
            AppError::BadRequest(message)
            | AppError::NotFound(message)
            | AppError::Conflict(message)
            | AppError::Internal(message) => message,
        }
    }
}

#[derive(Serialize)]
struct ErrorTemplateContext<'a> {
    title: &'a str,
    message: &'a str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let title = self.title();
        let message = self.message();

        let context = ErrorTemplateContext {
            title,
            message,
        };

        let rendered = Tera::one_off(
            include_str!("../templates/error.html"),
            &Context::from_serialize(&context).unwrap_or_default(),
            true,
        );

        match rendered {
            Ok(html) => (status, Html(html)).into_response(),
            Err(error) => {
                eprintln!(
                    "Errore durante il rendering della pagina di errore: {error}"
                );

                let fallback = format!(
                    "<h1>{}</h1><p>{}</p>",
                    title,
                    message
                );

                (status, Html(fallback)).into_response()
            }
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        eprintln!("Errore del database: {error}");

        AppError::Internal(
            "Si è verificato un errore durante l'accesso al database."
                .to_string(),
        )
    }
}

impl From<tera::Error> for AppError {
    fn from(error: tera::Error) -> Self {
        eprintln!("Errore nel rendering del template: {error}");

        AppError::Internal(
            "Si è verificato un errore durante la generazione della pagina."
                .to_string(),
        )
    }
}
