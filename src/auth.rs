use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use chrono::{Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;

use crate::{
    error::AppError,
    models::{UserPublic, UserRow},
    AppState,
};

pub const SESSION_COOKIE: &str = "mori_session";

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(password_hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn generate_session_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_session_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|part| part.trim().split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.to_string())
}

pub fn session_cookie(token: &str, ttl_hours: i64, secure: bool) -> HeaderValue {
    let max_age = ttl_hours.max(1) * 3600;
    let secure_part = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure_part}"
    ))
    .expect("session cookie value should be valid")
}

pub fn clear_session_cookie(secure: bool) -> HeaderValue {
    let secure_part = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure_part}"
    ))
    .expect("clear cookie value should be valid")
}

pub fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("local")
        .chars()
        .take(64)
        .collect()
}

pub fn client_user_agent(headers: &HeaderMap) -> String {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .chars()
        .take(255)
        .collect()
}

pub async fn current_user_optional(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<UserRow>, AppError> {
    let Some(token) = cookie_value(headers, SESSION_COOKIE) else {
        return Ok(None);
    };

    let token_hash = hash_session_token(&token);
    let now = Utc::now().naive_utc();
    let user = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT u.id, u.username, u.display_name, u.password_hash, u.role, u.status,
               u.last_login_at, u.login_count, u.created_at
        FROM sessions s
        INNER JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = ? AND s.expires_at > ?
        LIMIT 1
        "#,
    )
    .bind(&token_hash)
    .bind(now)
    .fetch_optional(&state.db)
    .await?;

    let Some(user) = user else {
        return Ok(None);
    };

    if user.status != "approved" {
        return Err(AppError::Forbidden(
            "账号当前不可用，请联系管理员".to_string(),
        ));
    }

    let _ = sqlx::query("UPDATE sessions SET last_seen_at = ? WHERE token_hash = ?")
        .bind(now)
        .bind(&token_hash)
        .execute(&state.db)
        .await;

    Ok(Some(user))
}

pub async fn current_user(state: &AppState, headers: &HeaderMap) -> Result<UserRow, AppError> {
    current_user_optional(state, headers)
        .await?
        .ok_or_else(|| AppError::Unauthorized("请先登录".to_string()))
}

pub fn require_role(user: &UserRow, roles: &[&str]) -> Result<(), AppError> {
    if roles.iter().any(|role| *role == user.role) {
        Ok(())
    } else {
        Err(AppError::Forbidden("当前账号没有此操作权限".to_string()))
    }
}

pub async fn record_login(
    pool: &MySqlPool,
    user_id: Option<i64>,
    username: &str,
    success: bool,
    reason: Option<&str>,
    ip: &str,
    user_agent: &str,
) {
    let result = sqlx::query(
        r#"
        INSERT INTO login_events (user_id, username, success, reason, ip, user_agent)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(user_id)
    .bind(username.chars().take(50).collect::<String>())
    .bind(success)
    .bind(reason.map(|value| value.chars().take(255).collect::<String>()))
    .bind(ip)
    .bind(user_agent)
    .execute(pool)
    .await;

    if let Err(error) = result {
        tracing::warn!(%error, "failed to record login event");
    }
}

pub async fn create_session(
    state: &AppState,
    user_id: i64,
    ip: &str,
    user_agent: &str,
) -> Result<String, AppError> {
    let token = generate_session_token();
    let token_hash = hash_session_token(&token);
    let now = Utc::now().naive_utc();
    let expires_at = now + Duration::hours(state.config.session_ttl_hours.max(1));

    let _ = sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(now)
        .execute(&state.db)
        .await;

    sqlx::query(
        r#"
        INSERT INTO sessions (token_hash, user_id, ip, user_agent, created_at, last_seen_at, expires_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&token_hash)
    .bind(user_id)
    .bind(ip)
    .bind(user_agent)
    .bind(now)
    .bind(now)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    Ok(token)
}

pub fn login_response(
    user: &UserRow,
    token: &str,
    ttl_hours: i64,
    secure: bool,
) -> axum::response::Response {
    let body = axum::Json(crate::models::AuthResponse {
        user: UserPublic::from(user),
    });
    let mut response = body.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        session_cookie(token, ttl_hours, secure),
    );
    response
}

pub fn logout_response(secure: bool) -> axum::response::Response {
    let mut response = (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "message": "已退出登录" })),
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, clear_session_cookie(secure));
    response
}

