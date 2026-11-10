use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;
use tera::{Context, Tera};
use tower_http::services::ServeDir;

const HOME_PAGE_PATH: &str = "Documentazione";

struct AppState {
    database: SqlitePool,
    tera: Tera,
}

#[derive(Debug)]
enum AppError {
    BadRequest(String),
    NotFound(String),
    Conflict(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(message) => {
                (axum::http::StatusCode::BAD_REQUEST, message)
            }
            AppError::NotFound(message) => {
                (axum::http::StatusCode::NOT_FOUND, message)
            }
            AppError::Conflict(message) => {
                (axum::http::StatusCode::CONFLICT, message)
            }
            AppError::Internal(message) => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, message)
            }
        };

        let mut context = Context::new();
        context.insert("message", &message);

        match render_error(&message) {
            Ok(html) => (status, Html(html)).into_response(),
            Err(_) => (status, message).into_response(),
        }
    }
}

type AppResult = Result<Response, AppError>;

#[derive(Deserialize)]
struct SavePagePayload {
    path: String,
    markdown: String,
    html: String,
}

#[derive(Serialize)]
struct SavePageResponse {
    redirect_url: String,
}

#[derive(sqlx::FromRow)]
struct PageRow {
    title: String,
    html: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL non impostata");

    let database = SqlitePool::connect(&database_url)
        .await
        .expect("Impossibile connettersi al database");

    sqlx::migrate!()
        .run(&database)
        .await
        .expect("Errore durante l'esecuzione delle migrazioni");

    let tera = Tera::new("templates/**/*")
        .expect("Errore nel caricamento dei template");

    let state = Arc::new(AppState { database, tera });

    let static_files = ServeDir::new("static");

    let app = Router::new()
        .route("/", get(view_home))
        .route("/~nuova", get(show_create_form))
        .route("/~nuova", post(create_page))
        .route("/~modifica/{*path}", get(show_edit_form))
        .route("/~modifica/{*path}", post(update_page))
        .route("/{*path}", get(view_page))
        .nest_service("/static", static_files)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Impossibile avviare il server sulla porta 3000");

    println!("Server avviato: http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .expect("Errore del server Axum");
}

async fn view_home(State(state): State<Arc<AppState>>) -> AppResult {
    view_page_by_path(&state, HOME_PAGE_PATH).await
}

async fn view_page(
    State(state): State<Arc<AppState>>,
    Path(raw_path): Path<String>,
) -> AppResult {
    let path = normalize_page_path(&raw_path)?;
    view_page_by_path(&state, &path).await
}

async fn view_page_by_path(state: &AppState, path: &str) -> AppResult {
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

async fn show_create_form(State(state): State<Arc<AppState>>) -> AppResult {
    let context = page_editor_context(
        "Crea nuova pagina",
        "create",
        "",
        "",
        "",
    );

    render_template(&state.tera, "page_editor.html", context)
}

async fn create_page(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SavePagePayload>,
) -> AppResult {
    let path = normalize_page_path(&payload.path)?;
    let title = page_title(&path);
    let safe_html = sanitize_html(&payload.html)?;

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

async fn show_edit_form(
    State(state): State<Arc<AppState>>,
    Path(raw_path): Path<String>,
) -> AppResult {
    let path = normalize_page_path(&raw_path)?;

    let markdown = sqlx::query_scalar::<_, String>(
        "SELECT markdown FROM pages WHERE path = ?",
    )
    .bind(&path)
    .fetch_optional(&state.database)
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    let Some(markdown) = markdown else {
        return Err(AppError::NotFound(format!(
            "La pagina /{} non esiste.",
            path
        )));
    };

    let context = page_editor_context(
        &format!("Modifica /{path}"),
        "edit",
        &path,
        &markdown,
        "",
    );

    render_template(&state.tera, "page_editor.html", context)
}

async fn update_page(
    State(state): State<Arc<AppState>>,
    Path(raw_path): Path<String>,
    Json(payload): Json<SavePagePayload>,
) -> AppResult {
    let path = normalize_page_path(&raw_path)?;
    let safe_html = sanitize_html(&payload.html)?;

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

fn normalize_page_path(raw_path: &str) -> Result<String, AppError> {
    let path = raw_path.trim();

    let path = path
        .trim_start_matches('/')
        .trim_end_matches('/');

    let segments: Vec<String> = path
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    if segments.is_empty() {
        return Err(AppError::BadRequest(
            "Il percorso della pagina non può essere vuoto.".to_string(),
        ));
    }

    if segments
        .iter()
        .any(|segment| segment == "." || segment == "..")
    {
        return Err(AppError::BadRequest(
            "Il percorso della pagina contiene segmenti non validi.".to_string(),
        ));
    }

    if segments.iter().any(|segment| {
        segment
            .chars()
            .any(|character| character.is_control() || character == '\\')
    }) {
        return Err(AppError::BadRequest(
            "Il percorso della pagina contiene caratteri non validi.".to_string(),
        ));
    }

    let normalized_path = segments.join("/");

    if normalized_path.len() > 512 {
        return Err(AppError::BadRequest(
            "Il percorso della pagina è troppo lungo.".to_string(),
        ));
    }

    Ok(normalized_path)
}

fn page_title(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

fn sanitize_html(html: &str) -> Result<String, AppError> {
    let mut cleaner = ammonia::Builder::default();

    cleaner
        .add_tags(["details", "summary"])
        .add_generic_attributes(["class", "id", "align", "width", "height"])
        .add_tag_attributes("a", ["target"])
        .add_tag_attributes("img", ["loading", "decoding"]);

    Ok(cleaner.clean(html).to_string())
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

#[derive(Serialize)]
struct BreadcrumbItem {
    label: String,
    href: String,
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
    _unused: &str,
) -> Context {
    let mut context = Context::new();
    context.insert("heading", &heading);
    context.insert("mode", &mode);
    context.insert("path", &path);
    context.insert("markdown", &markdown);

    context
}

fn render_template(
    tera: &Tera,
    template_name: &str,
    context: Context,
) -> AppResult {
    tera.render(template_name, &context)
        .map(Html)
        .map(IntoResponse::into_response)
        .map_err(|error| AppError::Internal(error.to_string()))
}

fn render_error(message: &str) -> Result<String, tera::Error> {
    let mut context = Context::new();
    context.insert("message", &message);

    let tera = Tera::one_off(
        r#"
        <!DOCTYPE html>
        <html lang="it">
        <head>
            <meta charset="UTF-8">
            <title>Errore - DaWiki</title>
        </head>
        <body>
            <h1>Errore</h1>
            <p>{{ message }}</p>
            <p><a href="/">Torna alla home</a></p>
        </body>
        </html>
        "#,
        &context,
        true,
    )?;

    Ok(tera)
}
