use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::MySqlPool;
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{
    auth::{
        client_ip, client_user_agent, create_session, current_user, current_user_optional,
        hash_password, login_response, logout_response, record_login, require_role,
        verify_password, SESSION_COOKIE,
    },
    error::AppError,
    models::{
        AccountRow, AccountStatusRequest, AccountView, AuthResponse, CreateProductRequest,
        DashboardResponse, DashboardStats, LoginDayRow, LoginPoint, LoginRequest, ProductRow,
        ProductStatusRequest, ProductView, RecentLoginRow, RecentLoginView, RegisterRequest,
        StorefrontResponse, UserPublic,
    },
    AppState,
};

const ACCOUNT_SELECT: &str = r#"
    SELECT u.id, u.username, u.display_name, u.role, u.status, u.rejection_reason,
           u.approved_at, u.last_login_at, u.login_count, u.created_at,
           COALESCE((
               SELECT COUNT(*) FROM products p
               WHERE p.owner_id = u.id AND p.status = 'active'
           ), 0) AS active_product_count
    FROM users u
"#;

const PRODUCT_SELECT: &str = r#"
    SELECT p.id, p.owner_id, u.display_name AS owner_name, p.name, p.subtitle,
           p.description, p.category, CAST(p.price AS DOUBLE) AS price,
           p.image_url, p.status, p.stock, p.created_at
    FROM products p
    INNER JOIN users u ON u.id = p.owner_id
"#;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/storefront", get(storefront))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route(
            "/seller/products",
            get(list_own_products).post(create_product),
        )
        .route("/seller/products/{id}/status", patch(update_product_status))
        .route("/admin/dashboard", get(admin_dashboard))
        .route("/admin/accounts", get(admin_accounts))
        .route("/admin/accounts/{id}/status", patch(admin_update_account))
        .with_state(state);

    let static_files = ServeDir::new("frontend").fallback(ServeFile::new("frontend/index.html"));

    Router::new()
        .nest("/api", api)
        .fallback_service(static_files)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "mori-shop",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StorefrontQuery {
    category: Option<String>,
    q: Option<String>,
}

async fn storefront(
    State(state): State<AppState>,
    Query(query): Query<StorefrontQuery>,
) -> Result<Json<StorefrontResponse>, AppError> {
    let category = query.category.unwrap_or_default().trim().to_string();
    let keyword = query.q.unwrap_or_default().trim().to_string();
    let like_keyword = format!("%{keyword}%");

    let products = sqlx::query_as::<_, ProductRow>(&format!(
        r#"
        {PRODUCT_SELECT}
        WHERE p.status = 'active'
          AND (? = '' OR p.category = ?)
          AND (? = '' OR p.name LIKE ? OR p.subtitle LIKE ? OR p.description LIKE ?)
        ORDER BY p.created_at DESC, p.id DESC
        "#
    ))
    .bind(&category)
    .bind(&category)
    .bind(&keyword)
    .bind(&like_keyword)
    .bind(&like_keyword)
    .bind(&like_keyword)
    .fetch_all(&state.db)
    .await?;

    let categories = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT category FROM products WHERE status = 'active' ORDER BY category",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(StorefrontResponse {
        categories,
        products: products.into_iter().map(ProductView::from).collect(),
    }))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Response, AppError> {
    let username = normalize_username(&payload.username)?;
    let display_name = payload
        .display_name
        .unwrap_or_else(|| username.clone())
        .trim()
        .chars()
        .take(80)
        .collect::<String>();
    let display_name = if display_name.is_empty() {
        username.clone()
    } else {
        display_name
    };

    validate_password(&payload.password)?;

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE username = ?")
        .bind(&username)
        .fetch_one(&state.db)
        .await?;
    if exists > 0 {
        return Err(AppError::Conflict("用户名已被使用".to_string()));
    }

    let password_hash = hash_password(&payload.password)?;
    sqlx::query(
        r#"
        INSERT INTO users (username, display_name, password_hash, role, status)
        VALUES (?, ?, ?, 'seller', 'pending')
        "#,
    )
    .bind(&username)
    .bind(&display_name)
    .bind(password_hash)
    .execute(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "注册申请已提交，管理员审核通过后即可登录"
        })),
    )
        .into_response())
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, AppError> {
    let username = payload.username.trim().to_string();
    let ip = client_ip(&headers);
    let user_agent = client_user_agent(&headers);

    if username.is_empty() || payload.password.is_empty() {
        return Err(AppError::BadRequest("请输入用户名和密码".to_string()));
    }

    // shortcut: 只按用户名统计失败次数，单实例够用；多实例或换用户名爆破需改 Redis/按 IP 限流。
    let recent_failures: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM login_events
        WHERE username = ? AND success = FALSE
          AND created_at >= UTC_TIMESTAMP(6) - INTERVAL 15 MINUTE
        "#,
    )
    .bind(&username)
    .fetch_one(&state.db)
    .await?;

    if recent_failures >= 10 {
        return Err(AppError::TooManyRequests(
            "尝试登录过于频繁，请 15 分钟后再试".to_string(),
        ));
    }

    let user = sqlx::query_as::<_, crate::models::UserRow>(
        r#"
        SELECT id, username, display_name, password_hash, role, status
        FROM users
        WHERE username = ?
        LIMIT 1
        "#,
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await?;

    let Some(user) = user else {
        record_login(
            &state.db,
            None,
            &username,
            false,
            Some("账号不存在"),
            &ip,
            &user_agent,
        )
        .await;
        return Err(AppError::Unauthorized("用户名或密码错误".to_string()));
    };

    if !verify_password(&payload.password, &user.password_hash)? {
        record_login(
            &state.db,
            Some(user.id),
            &user.username,
            false,
            Some("密码错误"),
            &ip,
            &user_agent,
        )
        .await;
        return Err(AppError::Unauthorized("用户名或密码错误".to_string()));
    }

    if user.status != "approved" {
        let reason = match user.status.as_str() {
            "pending" => "账号正在人工审核中，审核通过后方可登录",
            "rejected" => "账号审核未通过，请联系管理员",
            "disabled" => "账号已被停用，请联系管理员",
            _ => "账号当前不可登录",
        };
        record_login(
            &state.db,
            Some(user.id),
            &user.username,
            false,
            Some(reason),
            &ip,
            &user_agent,
        )
        .await;
        return Err(AppError::Forbidden(reason.to_string()));
    }

    let token = create_session(&state, user.id, &ip, &user_agent).await?;

    sqlx::query(
        r#"
        UPDATE users
        SET last_login_at = UTC_TIMESTAMP(6), login_count = login_count + 1
        WHERE id = ?
        "#,
    )
    .bind(user.id)
    .execute(&state.db)
    .await?;

    record_login(
        &state.db,
        Some(user.id),
        &user.username,
        true,
        None,
        &ip,
        &user_agent,
    )
    .await;

    Ok(login_response(
        &user,
        &token,
        state.config.session_ttl_hours,
        state.config.secure_cookie,
    ))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(token) = crate::auth::cookie_value(&headers, SESSION_COOKIE) {
        let token_hash = crate::auth::hash_session_token(&token);
        let _ = sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(token_hash)
            .execute(&state.db)
            .await;
    }

    logout_response(state.config.secure_cookie)
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthResponse>, AppError> {
    let user = current_user_optional(&state, &headers)
        .await?
        .ok_or_else(|| AppError::Unauthorized("请先登录".to_string()))?;

    Ok(Json(AuthResponse {
        user: UserPublic::from(&user),
    }))
}

async fn list_own_products(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProductView>>, AppError> {
    let user = current_user(&state, &headers).await?;
    require_role(&user, &["seller"])?;

    let products = sqlx::query_as::<_, ProductRow>(&format!(
        r#"
        {PRODUCT_SELECT}
        WHERE p.owner_id = ?
        ORDER BY p.created_at DESC, p.id DESC
        "#
    ))
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(products.into_iter().map(ProductView::from).collect()))
}

async fn create_product(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateProductRequest>,
) -> Result<Response, AppError> {
    let user = current_user(&state, &headers).await?;
    require_role(&user, &["seller"])?;

    let name = payload.name.trim();
    if name.chars().count() < 2 || name.chars().count() > 120 {
        return Err(AppError::BadRequest(
            "商品名称需要 2-120 个字符".to_string(),
        ));
    }

    let category = payload.category.trim();
    if category.is_empty() || category.chars().count() > 40 {
        return Err(AppError::BadRequest("请选择有效分类".to_string()));
    }

    if !payload.price.is_finite() || payload.price <= 0.0 {
        return Err(AppError::BadRequest("商品价格必须大于 0".to_string()));
    }
    if payload.stock < 0 {
        return Err(AppError::BadRequest("库存不能为负数".to_string()));
    }

    let status = payload.status.unwrap_or_else(|| "draft".to_string());
    if !matches!(status.as_str(), "draft" | "active") {
        return Err(AppError::BadRequest("商品状态无效".to_string()));
    }

    let subtitle = payload
        .subtitle
        .unwrap_or_default()
        .trim()
        .chars()
        .take(180)
        .collect::<String>();
    let description = payload
        .description
        .trim()
        .chars()
        .take(5000)
        .collect::<String>();
    if description.is_empty() {
        return Err(AppError::BadRequest("请填写商品描述".to_string()));
    }

    let image_url = payload
        .image_url
        .unwrap_or_default()
        .trim()
        .chars()
        .take(500)
        .collect::<String>();
    let image_url = if image_url.is_empty() {
        default_product_image(category).to_string()
    } else {
        image_url
    };

    let result = sqlx::query(
        r#"
        INSERT INTO products
            (owner_id, name, subtitle, description, category, price, image_url, status, stock)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(user.id)
    .bind(name)
    .bind(subtitle)
    .bind(description)
    .bind(category)
    .bind(payload.price)
    .bind(image_url)
    .bind(status)
    .bind(payload.stock)
    .execute(&state.db)
    .await?;

    let product = product_by_id(&state.db, result.last_insert_id()).await?;
    Ok((StatusCode::CREATED, Json(ProductView::from(product))).into_response())
}

async fn update_product_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(product_id): Path<u64>,
    Json(payload): Json<ProductStatusRequest>,
) -> Result<Json<ProductView>, AppError> {
    let user = current_user(&state, &headers).await?;
    require_role(&user, &["seller"])?;

    if !matches!(payload.status.as_str(), "draft" | "active" | "offline") {
        return Err(AppError::BadRequest("商品状态无效".to_string()));
    }

    let result = sqlx::query("UPDATE products SET status = ? WHERE id = ? AND owner_id = ?")
        .bind(&payload.status)
        .bind(product_id)
        .bind(user.id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("商品不存在".to_string()));
    }

    let product = product_by_id(&state.db, product_id).await?;
    Ok(Json(ProductView::from(product)))
}

async fn admin_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccountView>>, AppError> {
    let admin = current_user(&state, &headers).await?;
    require_role(&admin, &["admin"])?;
    Ok(Json(fetch_accounts(&state.db).await?))
}

async fn admin_update_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(account_id): Path<u64>,
    Json(payload): Json<AccountStatusRequest>,
) -> Result<Json<AccountView>, AppError> {
    let admin = current_user(&state, &headers).await?;
    require_role(&admin, &["admin"])?;

    if admin.id == account_id {
        return Err(AppError::BadRequest("不能修改自己的审核状态".to_string()));
    }

    let target_role =
        sqlx::query_scalar::<_, String>("SELECT role FROM users WHERE id = ? LIMIT 1")
            .bind(account_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("账号不存在".to_string()))?;

    if target_role == "admin" {
        return Err(AppError::Forbidden("不能修改管理员账号".to_string()));
    }

    if !matches!(
        payload.status.as_str(),
        "approved" | "rejected" | "disabled"
    ) {
        return Err(AppError::BadRequest("审核状态无效".to_string()));
    }

    if payload.status == "approved" {
        sqlx::query(
            r#"
            UPDATE users
            SET status = 'approved', rejection_reason = NULL,
                approved_by = ?, approved_at = UTC_TIMESTAMP(6)
            WHERE id = ?
            "#,
        )
        .bind(admin.id)
        .bind(account_id)
        .execute(&state.db)
        .await?;
    } else {
        let reason = payload
            .reason
            .unwrap_or_else(|| {
                if payload.status == "rejected" {
                    "人工审核未通过".to_string()
                } else {
                    "管理员停用账号".to_string()
                }
            })
            .trim()
            .chars()
            .take(255)
            .collect::<String>();

        sqlx::query("UPDATE users SET status = ?, rejection_reason = ? WHERE id = ?")
            .bind(&payload.status)
            .bind(reason)
            .bind(account_id)
            .execute(&state.db)
            .await?;
    }

    if payload.status != "approved" {
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(account_id)
            .execute(&state.db)
            .await?;
    }

    let account = fetch_account_by_id(&state.db, account_id).await?;
    Ok(Json(AccountView::from(account)))
}

async fn admin_dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DashboardResponse>, AppError> {
    let admin = current_user(&state, &headers).await?;
    require_role(&admin, &["admin"])?;

    let total_accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    let pending_accounts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status = 'pending'")
            .fetch_one(&state.db)
            .await?;
    let online_accounts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id) FROM sessions
        WHERE last_seen_at >= UTC_TIMESTAMP(6) - INTERVAL 15 MINUTE
          AND expires_at > UTC_TIMESTAMP(6)
        "#,
    )
    .fetch_one(&state.db)
    .await?;
    let today_logins: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM login_events WHERE success = TRUE AND created_at >= UTC_DATE()",
    )
    .fetch_one(&state.db)
    .await?;
    let active_products: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE status = 'active'")
            .fetch_one(&state.db)
            .await?;

    let accounts = fetch_accounts(&state.db).await?;
    let login_days = sqlx::query_as::<_, LoginDayRow>(
        r#"
        SELECT DATE_FORMAT(created_at, '%Y-%m-%d') AS day, COUNT(*) AS count
        FROM login_events
        WHERE success = TRUE
          AND created_at >= UTC_DATE() - INTERVAL 6 DAY
        GROUP BY day
        ORDER BY day
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let counts: HashMap<String, i64> = login_days
        .into_iter()
        .map(|row| (row.day, row.count))
        .collect();
    let today = Utc::now().date_naive();
    let login_series = (0..7)
        .rev()
        .map(|offset| {
            let date = (today - Duration::days(offset))
                .format("%Y-%m-%d")
                .to_string();
            LoginPoint {
                count: *counts.get(&date).unwrap_or(&0),
                date,
            }
        })
        .collect();

    let recent_logins = sqlx::query_as::<_, RecentLoginRow>(
        r#"
        SELECT le.id, COALESCE(u.username, le.username) AS username,
               le.success, le.reason, le.created_at
        FROM login_events le
        LEFT JOIN users u ON u.id = le.user_id
        ORDER BY le.created_at DESC, le.id DESC
        LIMIT 20
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(DashboardResponse {
        stats: DashboardStats {
            total_accounts,
            pending_accounts,
            online_accounts,
            today_logins,
            active_products,
        },
        accounts: accounts.into_iter().map(AccountView::from).collect(),
        login_series,
        recent_logins: recent_logins
            .into_iter()
            .map(RecentLoginView::from)
            .collect(),
    }))
}

async fn fetch_accounts(pool: &MySqlPool) -> Result<Vec<AccountView>, AppError> {
    let rows = sqlx::query_as::<_, AccountRow>(&format!(
        "{ACCOUNT_SELECT} ORDER BY CASE u.status WHEN 'pending' THEN 0 ELSE 1 END, u.created_at DESC"
    ))
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(AccountView::from).collect())
}

async fn fetch_account_by_id(pool: &MySqlPool, account_id: u64) -> Result<AccountRow, AppError> {
    sqlx::query_as::<_, AccountRow>(&format!("{ACCOUNT_SELECT} WHERE u.id = ? LIMIT 1"))
        .bind(account_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("账号不存在".to_string()))
}

async fn product_by_id(pool: &MySqlPool, product_id: u64) -> Result<ProductRow, AppError> {
    sqlx::query_as::<_, ProductRow>(&format!("{PRODUCT_SELECT} WHERE p.id = ? LIMIT 1"))
        .bind(product_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("商品不存在".to_string()))
}

fn normalize_username(value: &str) -> Result<String, AppError> {
    let username = value.trim().to_lowercase();
    let length = username.chars().count();
    if !(3..=32).contains(&length)
        || !username
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
    {
        return Err(AppError::BadRequest(
            "用户名需为 3-32 个字母、数字、中文或下划线".to_string(),
        ));
    }
    Ok(username)
}

fn validate_password(value: &str) -> Result<(), AppError> {
    let length = value.chars().count();
    if length < 8 || length > 128 {
        return Err(AppError::BadRequest(
            "密码长度需为 8-128 个字符".to_string(),
        ));
    }
    Ok(())
}

fn default_product_image(category: &str) -> &'static str {
    match category {
        "数码" => "/images/headphones.svg",
        "家居" => "/images/candle.svg",
        "穿搭" => "/images/tote.svg",
        "食品" => "/images/coffee.svg",
        "户外" => "/images/bottle.svg",
        _ => "/images/cushion.svg",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_username_trims_and_lowercases() {
        assert_eq!(normalize_username("  Alice_01 ").unwrap(), "alice_01");
        assert_eq!(normalize_username("山野小铺").unwrap(), "山野小铺");
    }

    #[test]
    fn normalize_username_rejects_invalid() {
        assert!(normalize_username("").is_err());
        assert!(normalize_username("ab").is_err());
        assert!(normalize_username("a b").is_err());
        assert!(normalize_username("with-dash").is_err());
        assert!(normalize_username(&"a".repeat(33)).is_err());
    }

    #[test]
    fn normalize_username_accepts_bounds() {
        assert_eq!(normalize_username(&"a".repeat(3)).unwrap().len(), 3);
        assert_eq!(normalize_username(&"a".repeat(32)).unwrap().len(), 32);
    }

    #[test]
    fn validate_password_checks_length_bounds() {
        assert!(validate_password("1234567").is_err());
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password(&"x".repeat(128)).is_ok());
        assert!(validate_password(&"x".repeat(129)).is_err());
    }

    #[test]
    fn default_product_image_maps_known_categories_and_falls_back() {
        assert_eq!(default_product_image("数码"), "/images/headphones.svg");
        assert_eq!(default_product_image("户外"), "/images/bottle.svg");
        assert_eq!(default_product_image("未知分类"), "/images/cushion.svg");
    }
}
