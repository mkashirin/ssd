use bcrypt::{DEFAULT_COST, hash};
use sqlx::sqlite::SqlitePoolOptions;

const DB_URL: &str = "sqlite://users.db?mode=rwc";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[*] Establishing connection with the database...");

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(DB_URL)
        .await?;

    println!("[*] Building the table scheme...");
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    let users = vec![
        ("admin", "admin123"),
        ("user1", "password"),
        ("john", "123456"),
        ("jane", "qwerty"),
        ("alice", "alice_pass"),
        ("bob", "builder"),
        ("charlie", "chocolate"),
        ("dave", "secret"),
        ("eve", "hacker"),
        ("mallory", "malicious"),
    ];

    println!("[*] Inserting users, hashing their passwords:");

    for (user, password) in users {
        let password_hash = hash(password, DEFAULT_COST)?;

        sqlx::query(
            "INSERT INTO users (username, password_hash) VALUES (?, ?)",
        )
        .bind(user)
        .bind(password_hash)
        .execute(&pool)
        .await?;

        println!("  User with login \"{}\" was inserted.", user);
    }

    println!("[+] Done!");
    Ok(())
}
