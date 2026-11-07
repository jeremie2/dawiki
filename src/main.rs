use axum::{
    extract::{Form, State, Path},
    response::{Html, IntoResponse, Response, Redirect},
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;
use serde::Deserialize;
use tera::Tera;
use std::sync::Arc;

struct AppState {
    db: SqlitePool,
    tera: Tera,
}

#[derive(Deserialize)]
struct DatiPagina {
    slug: String,
    titolo: String,
    markdown: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL non impostato");
    let pool = SqlitePool::connect(&db_url).await.expect("Impossibile connettersi al DB");

    let mut tera = Tera::new("templates/**/*").expect("Errore nella configurazione dei template");
    tera.full_reload().unwrap(); 

    let stato_condiviso = Arc::new(AppState { db: pool, tera });

    let app = Router::new()
        .route("/pagine/nuova", get(mostra_editor))
        .route("/pagine/salva", post(salva_pagina))
        .route("/wiki/*slug", get(visualizza_pagina))
        .with_state(stato_condiviso);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Server avviato! Vai su: http://127.0.0");
    axum::serve(listener, app).await.unwrap();
}

// Get the eteditor from `templates/editor.html` and show it.
async fn mostra_editor(State(stato): State<Arc<AppState>>) -> impl IntoResponse {
    let contesto = tera::Context::new();
    match stato.tera.render("editor.html", &contesto) {
        Ok(html_renderizzato) => Html(html_renderizzato).into_response(),
        Err(e) => {
            eprintln!("Errore Tera: {}", e);
            Response::builder().status(500).body("Errore Interno di Tera".into()).unwrap()
        }
    }
}

// Receive the form, elaborate the md and write into DB
async fn salva_pagina(
    State(stato): State<Arc<AppState>>,
    Form(input): Form<DatiPagina>, 
) -> impl IntoResponse {
    
    let mut opzioni = pulldown_cmark::Options::empty();
    opzioni.insert(pulldown_cmark::Options::ENABLE_TABLES);
    
    let parser = pulldown_cmark::Parser::new_ext(&input.markdown, opzioni);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    // Slug should start with '/'.
    let slug_pulito = if input.slug.starts_with('/') { input.slug.clone() } else { format!("/{}", input.slug) };

    let risultato = sqlx::query!(
        r#"
        INSERT OR REPLACE INTO pagine_wiki (slug, titolo, contenuto_markdown, contenuto_html)
        VALUES (?, ?, ?, ?)
        "#,
        slug_pulito,
        input.titolo,
        input.markdown,
        html_output
    )
    .execute(&stato.db)
    .await;

    match risultato {
        Ok(_) => {
            println!("Pagina '{}' salvata nel DB con slug: {}", input.titolo, slug_pulito);
            Redirect::to(&format!("/wiki/{}", slug_pulito.trim_start_matches('/'))).into_response()
        }
        Err(e) => {
            eprintln!("Errore durante il salvataggio nel DB: {}", e);
            Response::builder().status(500).body(format!("Errore DB: {}", e).into()).unwrap()
        }
    }
}

// Read from DB
async fn visualizza_pagina(
    State(stato): State<Arc<AppState>>,
    Path(slug): Path<String>, 
) -> impl IntoResponse {
    let slug_cercato = if slug.starts_with('/') { slug } else { format!("/{}", slug) };

    println!("Ricerca nel DB dello slug virtuale: {}", slug_cercato);

    let risultato = sqlx::query!(
        r#"SELECT titolo as "titolo!", contenuto_html as "contenuto_html!" FROM pagine_wiki WHERE slug = ?"#,
        slug_cercato
    )
    .fetch_optional(&stato.db)
    .await;

    match risultato {
        Ok(Some(pagina)) => {
            Html(format!(
                r#"<!DOCTYPE html>
                <html>
                <head>
                    <meta charset="UTF-8">
                    <title>{} - DaWiki</title>
                    <style>body {{ font-family: sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; line-height: 1.6; }}</style>
                </head>
                <body>
                    <h1>{}</h1>
                    <hr>
                    <div>{}</div>
                </body>
                </html>"#,
                pagina.titolo, pagina.titolo, pagina.contenuto_html
            )).into_response()
        }
        Ok(None) => {
            Response::builder()
                .status(404)
                .body(format!("Errore 404: Lo slug '{}' non esiste nel database!", slug_cercato).into())
                .unwrap()
        }
        Err(e) => {
            eprintln!("Errore di lettura dal DB: {}", e);
            Response::builder().status(500).body("Errore Interno del Server".into()).unwrap()
        }
    }
}
