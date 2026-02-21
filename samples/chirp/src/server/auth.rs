use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::models::user::UserSummary;

/// Sign up a new user
#[server]
pub async fn signup(
    username: String,
    email: String,
    password: String,
    display_name: String,
) -> Result<(), ServerFnError> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;
    use uuid::Uuid;

    let pool = super::db::pool()?;

    if username.len() < 3 || username.len() > 30 {
        return Err(ServerFnError::new(
            "Username must be between 3 and 30 characters",
        ));
    }
    if password.len() < 8 {
        return Err(ServerFnError::new("Password must be at least 8 characters"));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(ServerFnError::new(
            "Username can only contain letters, numbers, and underscores",
        ));
    }

    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = $1 OR email = $2")
            .bind(&username)
            .bind(&email)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if existing.is_some() {
        return Err(ServerFnError::new("Username or email is already taken"));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ServerFnError::new(format!("Password hashing error: {e}")))?
        .to_string();

    let id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, display_name) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .bind(&display_name)
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let session = extract_session().await?;
    session
        .insert("user_id", id)
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?;

    leptos_axum::redirect("/");
    Ok(())
}

/// Log in an existing user
#[server]
pub async fn login(username: String, password: String) -> Result<(), ServerFnError> {
    use crate::models::user::User;
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    let pool = super::db::pool()?;

    let user: Option<User> = sqlx::query_as(
        "SELECT id, username, email, password_hash, display_name, bio, avatar_url, header_url, \
         followers_count, following_count, posts_count, created_at, updated_at \
         FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let user = user.ok_or_else(|| ServerFnError::new("Invalid username or password"))?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| ServerFnError::new(format!("Password hash parse error: {e}")))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| ServerFnError::new("Invalid username or password"))?;

    let session = extract_session().await?;
    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?;

    leptos_axum::redirect("/");
    Ok(())
}

/// Log out the current user
#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    let session = extract_session().await?;
    session
        .flush()
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?;
    leptos_axum::redirect("/login");
    Ok(())
}

/// Get the currently logged-in user
#[server]
pub async fn get_current_user() -> Result<Option<crate::models::user::UserSummary>, ServerFnError> {
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let session = extract_session().await?;

    let user_id: Option<Uuid> = session
        .get("user_id")
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?;

    let Some(user_id) = user_id else {
        return Ok(None);
    };

    let user: Option<UserSummary> =
        sqlx::query_as("SELECT id, username, display_name, avatar_url FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(user)
}

#[cfg(feature = "ssr")]
pub(crate) async fn extract_session() -> Result<tower_sessions::Session, ServerFnError> {
    leptos_axum::extract::<tower_sessions::Session>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract session: {e}")))
}
