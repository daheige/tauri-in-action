mod application;
mod domain;
mod infra;
mod interfaces;
mod providers;
mod server;

use interfaces::commands::user::get_users;
use providers::AppProvider;
use tauri::Manager;

/// 桌面端入口（组合编排，保持薄层）：
///
/// 1. providers 显式初始化：配置 -> 连接池 -> 仓储（以抽象注入）-> 应用服务
/// 2. 初始化日志
/// 3. 注册应用服务为 Tauri 状态（供 interfaces/commands 命令注入）
/// 4. 后台启动内嵌 axum Web 服务（interfaces/routers -> server）
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 1. 显式初始化（依赖装配）
            //    block_on 提供 Tokio 上下文（sqlx 连接池创建需要）
            let provider = tauri::async_runtime::block_on(AppProvider::init())?;
            let config = provider.config.clone();

            // 2. 初始化日志
            init_tracing(&config);

            // redis 配置仅保留解析（本示例未接入），启动时打印便于确认
            if let Some(redis) = &config.redis_conf {
                tracing::debug!("redis 配置已加载: {}", redis.dsn);
            }

            // 3. 注册应用服务为 Tauri 状态，供 tauri 命令依赖注入
            app.manage(provider.services.user_service.clone());

            // 4. 后台启动内嵌 axum Web 服务（server 层）
            let port = config.app_port;
            tauri::async_runtime::spawn(async move {
                if let Err(e) = server::run(port, provider.services).await {
                    tracing::error!("axum server 退出: {e:#}");
                }
            });

            tracing::info!(
                "{} 启动完成，web 端口: {}",
                config.app_name,
                config.app_port
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_users])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing(config: &infra::config::AppConfig) {
    let filter = tracing_subscriber::EnvFilter::try_new(&config.log_level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
