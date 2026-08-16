use crate::models::{User, UserList};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use sqlx::MySqlPool;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

/// axum 共享状态：MySQL 连接池
#[derive(Clone)]
pub struct AppState {
    pub pool: MySqlPool,
}

/// 组装 axum 路由
pub fn build_router(pool: MySqlPool) -> Router {
    let state = Arc::new(AppState { pool });
    Router::new()
        .route("/api/health", get(health))
        .route("/api/users", get(list_users).post(create_user))
        .route(
            "/api/users/{id}",
            get(get_user).put(update_user).delete(delete_user),
        )
        // 前端页面与 API 不同源（dev 为 http://localhost:1420），放开 CORS
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// 启动内嵌的 axum Web 服务，监听 127.0.0.1:port
pub async fn run_server(port: u16, pool: MySqlPool) -> anyhow::Result<()> {
    let app = build_router(pool);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("axum web server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// GET /api/health —— 健康检查，附带数据库连通性探测
async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok();
    Json(json!({
        "status": "ok",
        "db": if db_ok { "ok" } else { "error" }
    }))
}

// ---------------------------------------------------------------------------
// 分页查询
// ---------------------------------------------------------------------------

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PAGE_SIZE: u64 = 10;
const MAX_PAGE_SIZE: u64 = 100;

/// 分页查询参数：/api/users?page=1&page_size=10
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// GET /api/users?page=1&page_size=10 —— 分页查询 users 表
async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<UserList>, ApiError> {
    let page = params.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = params
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = (page - 1) * page_size;

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool)
        .await?;

    let items = sqlx::query_as::<_, User>(
        "SELECT id, username FROM users ORDER BY id LIMIT ? OFFSET ?",
    )
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(UserList {
        total: total as u64,
        page,
        page_size,
        items,
    }))
}

// ---------------------------------------------------------------------------
// 查询单个
// ---------------------------------------------------------------------------

/// GET /api/users/{id} —— 按 id 查询单个用户
async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<User>, ApiError> {
    let user = sqlx::query_as::<_, User>("SELECT id, username FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("用户 id={id} 不存在")))?;
    Ok(Json(user))
}

// ---------------------------------------------------------------------------
// 新增
// ---------------------------------------------------------------------------

/// 新增用户请求体
#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
}

/// POST /api/users —— 新增用户
async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    let username = validate_username(&body.username)?;

    let result = sqlx::query("INSERT INTO users (username) VALUES (?)")
        .bind(&username)
        .execute(&state.pool)
        .await?;
    let id = result.last_insert_id();

    let user = sqlx::query_as::<_, User>("SELECT id, username FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    Ok((StatusCode::CREATED, Json(user)))
}

// ---------------------------------------------------------------------------
// 更新
// ---------------------------------------------------------------------------

/// 更新用户请求体
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub username: String,
}

/// PUT /api/users/{id} —— 更新用户名
async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    Json(body): Json<UpdateUser>,
) -> Result<Json<User>, ApiError> {
    let username = validate_username(&body.username)?;

    let result = sqlx::query("UPDATE users SET username = ? WHERE id = ?")
        .bind(&username)
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(format!("用户 id={id} 不存在")));
    }

    let user = sqlx::query_as::<_, User>("SELECT id, username FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(user))
}

// ---------------------------------------------------------------------------
// 删除
// ---------------------------------------------------------------------------

/// DELETE /api/users/{id} —— 删除用户
async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(format!("用户 id={id} 不存在")));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// 校验与错误处理
// ---------------------------------------------------------------------------

/// 用户名校验：非空、去首尾空白、长度 ≤ 100（与表结构 varchar(100) 一致）
fn validate_username(username: &str) -> Result<String, ApiError> {
    let username = username.trim();
    if username.is_empty() {
        return Err(ApiError::bad_request("用户名不能为空"));
    }
    if username.chars().count() > 100 {
        return Err(ApiError::bad_request("用户名长度不能超过 100 个字符"));
    }
    Ok(username.to_string())
}

/// 统一 API 错误：携带 HTTP 状态码与错误信息
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::internal(e.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError::internal(e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            tracing::error!("api error {}: {}", self.status, self.message);
        } else {
            tracing::warn!("api error {}: {}", self.status, self.message);
        }
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
