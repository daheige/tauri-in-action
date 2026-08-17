use crate::application::dto::page::{PageQuery, PageResult};
use crate::application::dto::user::{CreateUserInput, UpdateUserInput};
use crate::application::services::UserService;
use crate::domain::entity::User;
use crate::interfaces::handler::ApiError;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

/// GET /api/health —— 健康检查（含数据库连通性）
pub async fn health(State(service): State<UserService>) -> Json<serde_json::Value> {
    let db_ok = service.ping().await.is_ok();
    Json(json!({ "status": "ok", "db": if db_ok { "ok" } else { "error" } }))
}

/// GET /api/users?page=1&page_size=10 —— 分页查询
pub async fn list_users(
    State(service): State<UserService>,
    Query(query): Query<PageQuery>,
) -> Result<Json<PageResult<User>>, ApiError> {
    let result = service.page(query).await?;
    Ok(Json(result))
}

/// GET /api/users/{id} —— 按 id 查询
pub async fn get_user(
    State(service): State<UserService>,
    Path(id): Path<u64>,
) -> Result<Json<User>, ApiError> {
    let user = service.get(id).await?;
    Ok(Json(user))
}

/// POST /api/users —— 新增用户
pub async fn create_user(
    State(service): State<UserService>,
    Json(input): Json<CreateUserInput>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    let user = service.create(input).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

/// PUT /api/users/{id} —— 更新用户名
pub async fn update_user(
    State(service): State<UserService>,
    Path(id): Path<u64>,
    Json(input): Json<UpdateUserInput>,
) -> Result<Json<User>, ApiError> {
    let user = service.update(id, input).await?;
    Ok(Json(user))
}

/// DELETE /api/users/{id} —— 删除用户
pub async fn delete_user(
    State(service): State<UserService>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    service.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
