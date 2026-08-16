mod config;
mod db;
mod models;
mod web;

use config::AppConfig;
use sqlx::MySqlPool;
use tauri::Manager;

/// 桌面端入口：
/// 1. 加载 config/app.yaml 配置
/// 2. 初始化 tracing 日志
/// 3. 在后台异步任务中初始化 sqlx MySQL 连接池（需要 Tokio 上下文）
///    并启动内嵌的 axum Web 服务；连接池通过 AppHandle 注册为 Tauri 状态
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 1. 加载配置
            let cfg = AppConfig::load()?;

            // 2. 初始化日志
            init_tracing(&cfg);

            // redis 配置仅保留解析（本示例未接入），启动时打印便于确认
            if let Some(redis) = &cfg.redis_conf {
                tracing::debug!("redis 配置已加载: {}", redis.dsn);
            }

            // 3. 后台任务：初始化 MySQL 连接池 + 启动 axum Web 服务
            //    注意：setup 钩子运行在主线程、没有 Tokio 上下文，
            //    sqlx 连接池的创建必须在异步任务/运行时内进行。
            let handle = app.handle().clone();
            let port = cfg.app_port;
            let mysql_conf = cfg.mysql_conf.clone();
            tauri::async_runtime::spawn(async move {
                let pool = match db::init_pool(&mysql_conf) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("初始化 mysql 连接池失败: {e:#}");
                        return;
                    }
                };
                handle.manage(pool.clone());
                tracing::info!("mysql 连接池就绪: {}", mysql_conf.dsn);

                if let Err(e) = web::run_server(port, pool).await {
                    tracing::error!("axum server 退出: {e:#}");
                }
            });

            tracing::info!("{} 启动完成，web 端口: {}", cfg.app_name, cfg.app_port);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_users])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing(cfg: &AppConfig) {
    let filter = tracing_subscriber::EnvFilter::try_new(&cfg.log_level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

/// Tauri 命令：从 MySQL 读取 users 表（前端通过 window.__TAURI__.core.invoke 调用）
#[tauri::command]
async fn get_users(pool: tauri::State<'_, MySqlPool>) -> Result<Vec<models::User>, String> {
    let users = sqlx::query_as::<_, models::User>("SELECT id, username FROM users ORDER BY id")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(users)
}
