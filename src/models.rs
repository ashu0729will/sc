use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub role: String,
    pub status: String,
    pub last_login_at: Option<NaiveDateTime>,
    pub login_count: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPublic {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
}

impl From<&UserRow> for UserPublic {
    fn from(user: &UserRow) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            role: user.role.clone(),
            status: user.status.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub username: String,
    pub display_name: Option<String>,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub user: UserPublic,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProductRow {
    pub id: i64,
    pub owner_id: i64,
    pub owner_name: String,
    pub name: String,
    pub subtitle: String,
    pub description: String,
    pub category: String,
    pub price: f64,
    pub image_url: String,
    pub status: String,
    pub stock: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductView {
    pub id: i64,
    pub owner_id: i64,
    pub owner_name: String,
    pub name: String,
    pub subtitle: String,
    pub description: String,
    pub category: String,
    pub price: f64,
    pub image_url: String,
    pub status: String,
    pub stock: i64,
    pub created_at: NaiveDateTime,
}

impl From<ProductRow> for ProductView {
    fn from(product: ProductRow) -> Self {
        Self {
            id: product.id,
            owner_id: product.owner_id,
            owner_name: product.owner_name,
            name: product.name,
            subtitle: product.subtitle,
            description: product.description,
            category: product.category,
            price: product.price,
            image_url: product.image_url,
            status: product.status,
            stock: product.stock,
            created_at: product.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorefrontResponse {
    pub categories: Vec<String>,
    pub products: Vec<ProductView>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProductRequest {
    pub name: String,
    pub subtitle: Option<String>,
    pub description: String,
    pub category: String,
    pub price: f64,
    pub image_url: Option<String>,
    pub status: Option<String>,
    pub stock: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatusRequest {
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AccountRow {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub approved_at: Option<NaiveDateTime>,
    pub last_login_at: Option<NaiveDateTime>,
    pub login_count: i64,
    pub active_product_count: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub approved_at: Option<NaiveDateTime>,
    pub last_login_at: Option<NaiveDateTime>,
    pub login_count: i64,
    pub active_product_count: i64,
    pub created_at: NaiveDateTime,
}

impl From<AccountRow> for AccountView {
    fn from(account: AccountRow) -> Self {
        Self {
            id: account.id,
            username: account.username,
            display_name: account.display_name,
            role: account.role,
            status: account.status,
            rejection_reason: account.rejection_reason,
            approved_at: account.approved_at,
            last_login_at: account.last_login_at,
            login_count: account.login_count,
            active_product_count: account.active_product_count,
            created_at: account.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct LoginDayRow {
    pub day: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginPoint {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct RecentLoginRow {
    pub id: i64,
    pub username: String,
    pub success: bool,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentLoginView {
    pub id: i64,
    pub username: String,
    pub success: bool,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime,
}

impl From<RecentLoginRow> for RecentLoginView {
    fn from(event: RecentLoginRow) -> Self {
        Self {
            id: event.id,
            username: event.username,
            success: event.success,
            reason: event.reason,
            created_at: event.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_accounts: i64,
    pub pending_accounts: i64,
    pub online_accounts: i64,
    pub today_logins: i64,
    pub active_products: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardResponse {
    pub stats: DashboardStats,
    pub accounts: Vec<AccountView>,
    pub login_series: Vec<LoginPoint>,
    pub recent_logins: Vec<RecentLoginView>,
}
