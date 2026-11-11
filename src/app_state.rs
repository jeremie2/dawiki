use sqlx::SqlitePool;
use tera::Tera;

#[derive(Clone)]
pub struct AppState {
    pub database: SqlitePool,
    pub tera: Tera,
}