mod app_state;
mod error;
mod html_sanitizer;
mod page_path;
mod routes;

use crate::app_state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;
use tera::Tera;
use tower_http::services::ServeDir;

const HOME_PAGE_PATH: &str = "Documentazione";

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
        .route("/", get(routes::pages::view_home))
        .route("/~nuova", get(routes::pages::show_create_form))
        .route("/~nuova", post(routes::pages::create_page))
        .route("/~modifica/{*path}", get(routes::pages::show_edit_form))
        .route("/~modifica/{*path}", post(routes::pages::update_page))
        .route("/{*path}", get(routes::pages::view_page))
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
