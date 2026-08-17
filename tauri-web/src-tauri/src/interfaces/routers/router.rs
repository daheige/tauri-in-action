use crate::application::services::UserService;
use crate::interfaces::handler::user::{
    create_user, delete_user, get_user, health, list_users, update_user,
};
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;

/// 组装 axum 路由（接口层职责：仅做路由注册与中间件装配）
pub fn build_router(service: UserService) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/users", get(list_users).post(create_user))
        .route(
            "/api/users/{id}",
            get(get_user).put(update_user).delete(delete_user),
        )
        // 前端页面与 API 不同源（dev 为 http://localhost:1420），放开 CORS
        .layer(CorsLayer::permissive())
        .with_state(service)
}
