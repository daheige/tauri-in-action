use crate::application::services::Services;
use crate::interfaces::routers::build_router;
use std::net::SocketAddr;

/// 启动内嵌的 axum Web 服务，监听 127.0.0.1:port
pub async fn run(port: u16, services: Services) -> anyhow::Result<()> {
    let app = build_router(services.user_service);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("axum web server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
