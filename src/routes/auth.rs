use std::sync::Arc;

use argon2::{password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use axum::{extract::{Form, State}, http::{header, HeaderMap, HeaderValue, StatusCode}, response::{Html, IntoResponse, Redirect, Response}};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use serde::Deserialize;
use tera::Context;

use crate::{app_state::AppState, error::AppError};
use crate::routes::pages::insert_navigation_context;

/// Fields submitted by the local registration form.
#[derive(Deserialize)]
pub struct RegistrationForm {
    nickname: String,
    email: String,
    password: String,
}

/// Renders the registration form with the current navigation state.
pub async fn show_registration(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Html<String>, AppError> {
    let mut context = Context::new();
    let user = current_user(&state, &headers).await?;
    insert_navigation_context(&mut context, user.as_ref());
    let html = state.tera.render("registration.html", &context)?;
    Ok(Html(html))
}

/// Validates a new account and stores an Argon2 password hash.
///
/// SQLite enforces nickname and email uniqueness.
pub async fn register(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RegistrationForm>,
) -> Result<Response, AppError> {
    let nickname = form.nickname.trim();
    let email = form.email.trim();

    if nickname.chars().count() < 3 || nickname.chars().count() > 40
        || !nickname.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::BadRequest("Il nickname deve contenere da 3 a 40 caratteri: lettere ASCII, numeri, _ o -.".into()));
    }
    if email.len() > 254 || email.len() < 3 || email.contains(char::is_whitespace)
        || email.matches('@').count() != 1
        || !email.split('@').nth(1).is_some_and(|domain| domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')) {
        return Err(AppError::BadRequest("Inserisci un indirizzo email valido.".into()));
    }
    if form.password.len() < 12 || form.password.len() > 1024 {
        return Err(AppError::BadRequest("La password deve contenere almeno 12 caratteri (massimo 1024 byte).".into()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(form.password.as_bytes(), &salt)
        .map_err(|error| {
            eprintln!("Errore hashing password: {error}");
            AppError::Internal("Impossibile completare la registrazione.".into())
        })?
        .to_string();

    let result = sqlx::query("INSERT INTO users (nickname, email, password_hash) VALUES (?, ?, ?)")
        .bind(nickname)
        .bind(email)
        .bind(password_hash)
        .execute(&state.database)
        .await;

    match result {
        Ok(_) => {
            let mut context = Context::new();
            context.insert("nickname", nickname);
            let html = state.tera.render("registration_success.html", &context)?;
            Ok((StatusCode::CREATED, Html(html)).into_response())
        }
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
            Err(AppError::Conflict("Nickname o indirizzo email già registrato.".into()))
        }
        Err(error) => Err(AppError::from(error)),
    }
}


const SESSION_COOKIE: &str = "dawiki_session";

/// Credentials submitted by the local login form.
#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

/// Application permissions, independent of the login provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    User,
    Staff,
    Admin,
}

impl UserRole {
    /// Converts a database role into a typed value; rejects unknown roles.
    pub fn from_db(value: &str) -> Result<Self, AppError> {
        match value {
            "user" => Ok(Self::User),
            "staff" => Ok(Self::Staff),
            "admin" => Ok(Self::Admin),
            _ => Err(AppError::Internal("Ruolo utente non riconosciuto.".into())),
        }
    }

    /// Returns the database representation of this role.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Staff => "staff",
            Self::Admin => "admin",
        }
    }
}

/// Identity and role recovered from a valid database session.
pub struct CurrentUser {
    pub id: i64,
    pub nickname: String,
    pub role: UserRole,
}

/// Hashes a random session token so the database never stores its raw value.
fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

/// Extracts a well-formed cookie token without validating the session.
fn session_token(headers: &HeaderMap) -> Option<&str> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == SESSION_COOKIE && value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .then_some(value)
    })
}

/// Resolves the session cookie into an identity and its current role.
///
/// Returns `Ok(None)` when no valid session exists.
pub async fn current_user(state: &AppState, headers: &HeaderMap) -> Result<Option<CurrentUser>, AppError> {
    let Some(token) = session_token(headers) else {
        return Ok(None);
    };
    let user = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT u.id, u.nickname, u.role FROM users u
         JOIN user_sessions s ON s.user_id = u.id
         WHERE s.token_hash = ? AND s.expires_at > CURRENT_TIMESTAMP",
    )
    .bind(token_hash(token))
    .fetch_optional(&state.database)
    .await?;
    user.map(|(id, nickname, role)| {
        Ok(CurrentUser { id, nickname, role: UserRole::from_db(&role)? })
    }).transpose()
}

/// Renders the login form with navigation appropriate to the visitor.
pub async fn show_login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Html<String>, AppError> {
    let mut context = Context::new();
    let user = current_user(&state, &headers).await?;
    insert_navigation_context(&mut context, user.as_ref());
    Ok(Html(state.tera.render("login.html", &context)?))
}

/// Verifies local credentials and creates a database-backed session.
pub async fn login(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    if form.email.len() > 254 || form.password.len() > 1024 {
        return Err(AppError::BadRequest("Email o password non corretti.".into()));
    }
    let record = sqlx::query_as::<_, (i64, Option<String>)>(
        "SELECT id, password_hash FROM users WHERE email = ?",
    )
    .bind(form.email.trim())
    .fetch_optional(&state.database)
    .await?;

    let Some((user_id, Some(stored_hash))) = record else {
        return Err(AppError::BadRequest("Email o password non corretti.".into()));
    };
    let parsed = PasswordHash::new(&stored_hash)
        .map_err(|_| AppError::Internal("Hash della password non valido.".into()))?;
    if Argon2::default().verify_password(form.password.as_bytes(), &parsed).is_err() {
        return Err(AppError::BadRequest("Email o password non corretti.".into()));
    }

    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();

    sqlx::query(
        "INSERT INTO user_sessions (user_id, token_hash, expires_at)
         VALUES (?, ?, datetime('now', '+7 days'))",
    )
    .bind(user_id)
    .bind(token_hash(&token))
    .execute(&state.database)
    .await?;

    let cookie = format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800"
    );
    let mut response = Redirect::to("/~profilo").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie)
            .map_err(|_| AppError::Internal("Impossibile impostare il cookie.".into()))?,
    );
    Ok(response)
}

/// Shows account settings to the authenticated user.
pub async fn profile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let Some(user) = current_user(&state, &headers).await? else {
        return Ok(Redirect::to("/~login").into_response());
    };
    let mut context = Context::new();
    context.insert("nickname", &user.nickname);
    context.insert("user_id", &user.id);
    context.insert("role", user.role.as_str());
    insert_navigation_context(&mut context, Some(&user));
    Ok(Html(state.tera.render("profile.html", &context)?).into_response())
}

/// Revokes the session and expires the browser cookie.
///
/// Logout remains a POST action even when styled as a navigation link.
pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(token) = session_token(&headers) {
        sqlx::query("DELETE FROM user_sessions WHERE token_hash = ?")
            .bind(token_hash(token))
            .execute(&state.database)
            .await?;
    }
    let mut response = Redirect::to("/~login").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("dawiki_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"),
    );
    Ok(response)
}
