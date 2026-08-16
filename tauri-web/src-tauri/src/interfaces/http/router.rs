use super::handler::{create_user, delete_user, get_user, health, list_users, update_user};
use crate::application::service::UserService;
use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;
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

/// 启动内嵌的 axum Web 服务，监听 127.0.0.1:port
pub async fn run_server(port: u16, service: UserService) -> anyhow::Result<()> {
    let app = build_router(service);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("axum web server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
