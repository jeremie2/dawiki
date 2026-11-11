use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tera::Context;

use crate::app_state::AppState;
use crate::error::AppError;
use crate::html_sanitizer::sanitize_page_html;
use crate::page_path::{normalize_page_path, page_title_from_path};

pub const HOME_PAGE_PATH: &str = "Documentazione";

pub type AppResult = Result<Response, AppError>;

#[derive(Deserialize)]
pub struct SavePagePayload {
    pub path: String,
    pub markdown: String,
    pub html: String,
}

#[derive(Serialize)]
pub struct SavePageResponse {
    pub redirect_url: String,
}

#[derive(sqlx::FromRow)]
pub struct PageRow {
    pub title: String,
    pub html: String,
}

#[derive(Serialize)]
pub struct BreadcrumbItem {
    pub label: String,
    pub href: String,
}

pub async fn view_home(
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult {
    view_page_by_path(&state, HOME_PAGE_PATH).await
}

pub async fn view_page(
    State(state): State<std::sync::Arc<AppState>>,
    Path(raw_path): Path<String>,
) -> AppResult {
    let path = normalize_page_path(&raw_path)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    view_page_by_path(&state, &path).await
}

async fn view_page_by_path(
    state: &AppState,
    path: &str,
) -> AppResult {
    let page = sqlx::query_as::<_, PageRow>(
        "SELECT title, html FROM pages WHERE path = ?",
    )
    .bind(path)
    .fetch_optional(&state.database)
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    let Some(page) = page else {
        return Err(AppError::NotFound(format!(
            "La pagina /{} non esiste.",
            path
        )));
    };

    let context = page_view_context(path, &page.title, &page.html);

    render_template(&state.tera, "page_view.html", context)
}

pub async fn show_create_form(
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult {
    let context = page_editor_context(
        "Crea nuova pagina",
        "create",
        "",
        "",
    );

    render_template(&state.tera, "page_editor.html", context)
}

pub async fn create_page(
    State(state): State<std::sync::Arc<AppState>>,
    Json(payload): Json<SavePagePayload>,
) -> AppResult {
    let path = normalize_page_path(&payload.path)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    let title = page_title_from_path(&path);
    let safe_html = sanitize_page_html(&payload.html)?;

    let result = sqlx::query(
        "INSERT INTO pages (path, title, markdown, html) VALUES (?, ?, ?, ?)",
    )
    .bind(&path)
    .bind(&title)
    .bind(&payload.markdown)
    .bind(&safe_html)
    .execute(&state.database)
    .await;

    match result {
        Ok(_) => Ok(Json(SavePageResponse {
            redirect_url: format!("/{path}"),
        })
        .into_response()),
        Err(error) => {
            if is_unique_constraint_error(&error) {
                return Err(AppError::Conflict(format!(
                    "La pagina /{} esiste già.",
                    path
                )));
            }

            Err(AppError::Internal(error.to_string()))
        }
    }
}

pub async fn show_edit_form(
    State(state): State<std::sync::Arc<AppState>>,
    Path(raw_path): Path<String>,
) -> AppResult {
    let path = normalize_page_path(&raw_path)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    let markdown_option = sqlx::query_scalar::<_, String>(
        "SELECT markdown FROM pages WHERE path = ?",
    )
    .bind(&path)
    .fetch_optional(&state.database)
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    let Some(markdown_content) = markdown_option else {
        return Err(AppError::NotFound(format!(
            "La pagina /{} non esiste.",
            path
        )));
    };

    let context = page_editor_context(
        &format!("Modifica /{path}"),
        "edit",
        &path,
        &markdown_content,
    );

    render_template(&state.tera, "page_editor.html", context)
}

pub async fn update_page(
    State(state): State<std::sync::Arc<AppState>>,
    Path(raw_path): Path<String>,
    Json(payload): Json<SavePagePayload>,
) -> AppResult {
    let path = normalize_page_path(&raw_path)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    let safe_html = sanitize_page_html(&payload.html)?;

    let result = sqlx::query(
        r#"
        UPDATE pages
        SET markdown = ?, html = ?, updated_at = CURRENT_TIMESTAMP
        WHERE path = ?
        "#,
    )
    .bind(&payload.markdown)
    .bind(&safe_html)
    .bind(&path)
    .execute(&state.database)
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "La pagina /{} non esiste.",
            path
        )));
    }

    Ok(Json(SavePageResponse {
        redirect_url: format!("/{path}"),
    })
    .into_response())
}

fn is_unique_constraint_error(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .map(|database_error| {
            database_error
                .message()
                .contains("UNIQUE constraint failed")
        })
        .unwrap_or(false)
}

fn page_view_context(
    path: &str,
    title: &str,
    content_html: &str,
) -> Context {
    let segments: Vec<&str> = path.split('/').collect();

    let breadcrumb = segments
        .iter()
        .take(segments.len().saturating_sub(1))
        .scan(String::new(), |accumulator, segment| {
            if accumulator.is_empty() {
                accumulator.push_str(segment);
            } else {
                accumulator.push('/');
                accumulator.push_str(segment);
            }

            Some(BreadcrumbItem {
                label: segment.to_string(),
                href: format!("/{accumulator}"),
            })
        })
        .collect::<Vec<_>>();

    let mut context = Context::new();
    context.insert("title", &title);
    context.insert("path", &path);
    context.insert("breadcrumb", &breadcrumb);
    context.insert("content_html", &content_html);
    context.insert("edit_url", &format!("/~modifica/{path}"));

    context
}

fn page_editor_context(
    heading: &str,
    mode: &str,
    path: &str,
    markdown: &str,
) -> Context {
    let mut context = Context::new();
    context.insert("heading", &heading);
    context.insert("mode", &mode);
    context.insert("path", &path);
    context.insert("markdown", &markdown);

    context
}

fn render_template(
    tera: &tera::Tera,
    template_name: &str,
    context: Context,
) -> AppResult {
    tera.render(template_name, &context)
        .map(Html)
        .map(IntoResponse::into_response)
        .map_err(|error| AppError::Internal(error.to_string()))
}
