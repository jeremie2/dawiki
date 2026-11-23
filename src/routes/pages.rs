use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
    Json,
    http::{HeaderMap, StatusCode},
    response::Redirect,
};
use serde::{Deserialize, Serialize};
use tera::Context;

use crate::app_state::AppState;
use crate::routes::auth::{current_user, UserRole};
use crate::error::AppError;
use crate::html_sanitizer::sanitize_page_html;
use crate::page_path::{normalize_page_path, page_title_from_path};

pub const HOME_PAGE_PATH: &str = "Documentazione";

pub type AppResult = Result<Response, AppError>;

/// JSON submitted by the editor. Page protection is server-controlled.
#[derive(Deserialize)]
pub struct SavePagePayload {
    pub path: String,
    pub markdown: String,
    pub html: String,
    pub toc_html: String,
}

#[derive(Serialize)]
pub struct SavePageResponse {
    pub redirect_url: String,
}

/// Data used to render a wiki page and its permissions.
#[derive(sqlx::FromRow)]
pub struct PageRow {
    pub title: String,
    pub html: String,
    pub toc_html: String,
    pub is_protected: i64,
}

#[derive(Serialize)]
pub struct BreadcrumbItem {
    pub label: String,
    pub href: String,
}

/// Renders the wiki home page.
pub async fn view_home(
    State(state): State<std::sync::Arc<AppState>>,
    headers: HeaderMap,
) -> AppResult {
    view_page_by_path(&state, HOME_PAGE_PATH, &headers).await
}

/// Resolves and renders a wiki page by path.
pub async fn view_page(
    State(state): State<std::sync::Arc<AppState>>,
    Path(raw_path): Path<String>,
    headers: HeaderMap,
) -> AppResult {
    let path = normalize_page_path(&raw_path)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    view_page_by_path(&state, &path, &headers).await
}

/// Loads the page and computes its available actions.
async fn view_page_by_path(
    state: &AppState,
    path: &str,
    headers: &HeaderMap,
) -> AppResult {
    let page = sqlx::query_as::<_, PageRow>(
        "SELECT title, html, toc_html, is_protected FROM pages WHERE path = ?",
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

    let mut context = page_view_context(
        path,
        &page.title,
        &page.html,
        &page.toc_html,
    );
    let user = current_user(state, headers).await?;
    insert_navigation_context(&mut context, user.as_ref());
    let allowed = user.as_ref().is_some_and(|u| page.is_protected == 0 || can_edit_protected(u.role));
    context.insert("page_actions_visible", &true);
    context.insert("can_edit_page", &allowed);
    context.insert("is_protected", &(page.is_protected != 0));

    render_template(&state.tera, "page_view.html", context)
}

/// Shows the creation editor to authenticated users.
pub async fn show_create_form(
    State(state): State<std::sync::Arc<AppState>>,
    headers: HeaderMap,
) -> AppResult {
    if current_user(&state, &headers).await?.is_none() {
        return Ok(Redirect::to("/~login").into_response());
    }
    let mut context = page_editor_context(
        "Crea nuova pagina",
        "create",
        "",
        "",
    );
    let user = current_user(&state, &headers).await?;
    insert_navigation_context(&mut context, user.as_ref());

    render_template(&state.tera, "page_editor.html", context)
}

/// Creates a page after checking authentication.
pub async fn create_page(
    State(state): State<std::sync::Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<SavePagePayload>,
) -> AppResult {
    if current_user(&state, &headers).await?.is_none() {
        return Ok((StatusCode::UNAUTHORIZED, "Devi effettuare il login per creare pagine.").into_response());
    }
    let path = normalize_page_path(&payload.path)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    let title = page_title_from_path(&path);
    let safe_html = sanitize_page_html(&payload.html)?;

    let result = sqlx::query(
    "INSERT INTO pages (path, title, markdown, html, toc_html) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&path)
    .bind(&title)
    .bind(&payload.markdown)
    .bind(&safe_html)
    .bind(&payload.toc_html)
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

/// Opens the editor after checking page permissions.
pub async fn show_edit_form(
    State(state): State<std::sync::Arc<AppState>>,
    Path(raw_path): Path<String>,
    headers: HeaderMap,
) -> AppResult {
    if current_user(&state, &headers).await?.is_none() {
        return Ok(Redirect::to("/~login").into_response());
    }
    let path = normalize_page_path(&raw_path)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    let page_option = sqlx::query_as::<_, (String, i64)>(
        "SELECT markdown, is_protected FROM pages WHERE path = ?",
    )
    .bind(&path)
    .fetch_optional(&state.database)
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    let Some((markdown_content, is_protected)) = page_option else {
        return Err(AppError::NotFound(format!(
            "La pagina /{} non esiste.",
            path
        )));
    };

    // Protected pages are editable only by staff and administrators.
    let user = current_user(&state, &headers).await?.expect("Authentication checked above");
    if is_protected != 0 && !can_edit_protected(user.role) {
        return Ok((StatusCode::FORBIDDEN, "Non hai i permessi per modificare questa pagina.").into_response());
    }

    let mut context = page_editor_context(
        &format!("Modifica /{path}"),
        "edit",
        &path,
        &markdown_content,
    );
    insert_navigation_context(&mut context, Some(&user));

    render_template(&state.tera, "page_editor.html", context)
}

/// Enforces permissions again when saving changes.
pub async fn update_page(
    State(state): State<std::sync::Arc<AppState>>,
    Path(raw_path): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<SavePagePayload>,
) -> AppResult {
    if current_user(&state, &headers).await?.is_none() {
        return Ok((StatusCode::UNAUTHORIZED, "Devi effettuare il login per modificare pagine.").into_response());
    }
    let path = normalize_page_path(&raw_path)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    let user = current_user(&state, &headers).await?.expect("Authentication checked above");
    let protected_editor = can_edit_protected(user.role);
    let page_protection = sqlx::query_scalar::<_, i64>(
        "SELECT is_protected FROM pages WHERE path = ?",
    )
    .bind(&path)
    .fetch_optional(&state.database)
    .await?;

    let Some(is_protected) = page_protection else {
        return Err(AppError::NotFound(format!("La pagina /{} non esiste.", path)));
    };
    if is_protected != 0 && !protected_editor {
        return Ok((StatusCode::FORBIDDEN, "Non hai i permessi per modificare questa pagina.").into_response());
    }

    let safe_html = sanitize_page_html(&payload.html)?;

    let result = sqlx::query(
    r#"
    UPDATE pages
    SET markdown = ?, html = ?, toc_html = ?, updated_at = CURRENT_TIMESTAMP
    WHERE path = ? AND (is_protected = 0 OR ? = 1)
    "#,
    )
    .bind(&payload.markdown)
    .bind(&safe_html)
    .bind(&payload.toc_html)
    .bind(&path)
    .bind(protected_editor)
    .execute(&state.database)
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    if result.rows_affected() == 0 {
        // The page might have been protected after the initial permission check.
        let still_exists = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM pages WHERE path = ?",
        )
        .bind(&path)
        .fetch_optional(&state.database)
        .await?;
        if still_exists.is_some() {
            return Ok((StatusCode::FORBIDDEN, "Non hai i permessi per modificare questa pagina.").into_response());
        }
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

/// Detects duplicate page paths reported by SQLite.
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

/// Builds template data including breadcrumbs.
fn page_view_context(
    path: &str,
    title: &str,
    content_html: &str,
    toc_html: &str,
) -> Context {
    let segments: Vec<&str> = path.split('/').collect();

    let breadcrumb = segments
        .iter()
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
    context.insert("toc_html", &toc_html);
    context.insert("edit_url", &format!("/~modifica/{path}"));

    context
}

/// Builds the Markdown editor context.
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

/// Renders a Tera template as an HTTP response.
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

/// Staff and administrators may edit pages marked as protected.
fn can_edit_protected(role: UserRole) -> bool {
    matches!(role, UserRole::Staff | UserRole::Admin)
}

#[cfg(test)]
mod permission_tests {
    use super::*;

    #[test]
    fn only_staff_and_admin_can_edit_protected_pages() {
        assert!(!can_edit_protected(UserRole::User));
        assert!(can_edit_protected(UserRole::Staff));
        assert!(can_edit_protected(UserRole::Admin));
    }
}

/// Adds session-dependent navigation data to a Tera context.
pub fn insert_navigation_context(context: &mut Context, user: Option<&crate::routes::auth::CurrentUser>) {
    context.insert("authenticated", &user.is_some());
    if let Some(user) = user {
        context.insert("current_nickname", &user.nickname);
        context.insert("personal_page_url", &format!("/{}", user.nickname));
    }
}
