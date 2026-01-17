use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    routing::get,
};
use bcrypt::verify;
use rand::{Rng, distr::Alphanumeric};
use serde::Deserialize;
use sqlx::{Pool, Row, Sqlite, sqlite::SqlitePoolOptions};
use std::{path::Path, time::Duration};
use tokio::time::sleep;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

const ERROR: i32 = 1;

const DB_URL: &str = "sqlite://users.db";
const HTML_TEMPLATE: &str = include_str!("../templates/index.html");
const CSRF_K: &str = "csrf_token";
const ATTEMPTS_K: &str = "failed_attempts";

const MAX_CONNECTIONS: u32 = 5;
const RATE_LIMIT_SEC: u64 = 2;

#[derive(Clone)]
struct AppState {
    db: Pool<Sqlite>,
}

#[derive(Deserialize)]
struct LoginParams {
    username: Option<String>,
    password: Option<String>,
    user_token: Option<String>,
    #[serde(rename = "Login")]
    login_btn: Option<String>,
}

#[derive(Deserialize)]
struct BlindSqliParams {
    id: Option<String>,
    #[serde(rename = "Submit")]
    submit: Option<String>,
}

#[tokio::main]
async fn main() {
    if !Path::new("users.db").exists() {
        eprintln!("Error: cannot find users.db file. Run db_init.sh");
        std::process::exit(ERROR);
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect(DB_URL)
        .await
        .expect("Could not connect to the DB");

    let session_layer =
        SessionManagerLayer::new(MemoryStore::default()).with_secure(false);

    let app = Router::new()
        .route("/vulnerabilities/brute/", get(login_handler))
        .route("/vulnerabilities/sqli_blind/", get(blind_sqli_handler))
        .layer(session_layer)
        .with_state(AppState { db: pool });

    println!("Server: http://127.0.0.1:3000/vulnerabilities/brute/");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn login_handler(
    State(state): State<AppState>,
    session: Session,
    Query(params): Query<LoginParams>,
) -> Html<String> {
    let mut csrf_token = get_or_create_token(&session).await;
    let attempts: u32 = session.get(ATTEMPTS_K).await.unwrap().unwrap_or(0);

    if params.login_btn.is_none() {
        return render_html(&csrf_token, "", "");
    }

    let user = params.username.as_deref().unwrap();
    let password = params.password.as_deref().unwrap();
    let incoming_token = params.user_token.as_deref().unwrap_or("");

    if incoming_token != csrf_token {
        return render_html(
            &csrf_token,
            "<div class='message error'>Error: CSRF Token mismatch.</div>",
            user,
        );
    }

    if attempts >= 3 {
        sleep(Duration::from_secs(RATE_LIMIT_SEC)).await;
    }

    let is_valid = check_db_credentials(&state.db, user, password).await;

    let message = if is_valid {
        let _ = session.insert(ATTEMPTS_K, 0).await;
        csrf_token = regenerate_token(&session).await;
        success_message(user)
    } else {
        let _ = session.insert(ATTEMPTS_K, attempts + 1).await;
        if attempts >= 3 {
            "<div class='message error'>Too many failed attempts.<br>Username/password incorrect.</div>"
                .into()
        } else {
            "<div class='message error'>Username and/or password incorrect.</div>"
                .into()
        }
    };

    render_html(&csrf_token, &message, user)
}

async fn blind_sqli_handler(
    State(state): State<AppState>,
    Query(params): Query<BlindSqliParams>,
) -> (StatusCode, Html<String>) {
    let html_top = r#"
    <!DOCTYPE html>
    <html>
    <head><title>Secure Blind SQLi</title></head>
    <body style="font-family: sans-serif; padding: 20px;">
        <h2>Blind SQL Injection (Secure Rust Implementation)</h2>
        <div style="border: 1px solid #ccc; padding: 20px; width: 300px;">
            <form action="" method="GET">
                <label>User ID:</label><br>
                <input type="text" name="id" style="width: 100%"><br><br>
                <input type="submit" name="Submit" value="Submit">
            </form>
        </div>
        <br>
    "#;
    let html_bottom = "</body></html>";

    if params.submit.is_none() {
        return (StatusCode::OK, Html(format!("{}{}", html_top, html_bottom)));
    }

    let id_input = params.id.as_deref().unwrap_or("");

    let query = "SELECT id FROM users WHERE id = ?";

    let result = sqlx::query(query)
        .bind(id_input)
        .fetch_optional(&state.db)
        .await;

    match result {
        Ok(Some(_)) => {
            let msg = "<pre>User ID exists in the database.</pre>";
            (
                StatusCode::OK,
                Html(format!("{}{}{}", html_top, msg, html_bottom)),
            )
        }
        Ok(None) => {
            let msg = "<pre>User ID is MISSING from the database.</pre>";
            (
                StatusCode::NOT_FOUND,
                Html(format!("{}{}{}", html_top, msg, html_bottom)),
            )
        }
        Err(_) => {
            let msg = "<pre>Error: Invalid input.</pre>";
            (
                StatusCode::OK,
                Html(format!("{}{}{}", html_top, msg, html_bottom)),
            )
        }
    }
}

fn render_html(token: &str, msg: &str, user_val: &str) -> Html<String> {
    Html(
        HTML_TEMPLATE
            .replace("{message}", msg)
            .replace("{csrf_token}", token)
            .replace("{username_value}", user_val),
    )
}

async fn get_or_create_token(session: &Session) -> String {
    if let Ok(Some(token)) = session.get(CSRF_K).await {
        token
    } else {
        regenerate_token(session).await
    }
}

async fn regenerate_token(session: &Session) -> String {
    let token: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let _ = session.insert(CSRF_K, &token).await;
    token
}

async fn check_db_credentials(
    pool: &Pool<Sqlite>,
    user: &str,
    pass: &str,
) -> bool {
    sqlx::query("SELECT password_hash FROM users WHERE username = ?")
        .bind(user)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten() // Превращает Option<Result> в Option<Row>
        .map(|row| {
            let hash: String = row.get("password_hash");
            verify(pass, &hash).unwrap_or(false)
        })
        .unwrap_or(false)
}

fn success_message(user: &str) -> String {
    format!(
        "<div class='message success'>Welcome, <b>{}</b></div>",
        user
    )
}
