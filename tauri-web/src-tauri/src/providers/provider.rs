use crate::application::services::{Services, UserService};
use crate::domain::repository::UserRepository;
use crate::infra::config::mysql::init_db_pool;
use crate::infra::config::AppConfig;
use crate::infra::persistence::users::new_user_repo;
use anyhow::Result;
use std::sync::Arc;

/// 组合根：显式初始化全部依赖，完成依赖倒置装配。
///
/// 初始化顺序（也是依赖方向）：
/// 1. infra/config      —— 读取配置文件
/// 2. infra/persistence —— 创建 MySQL 连接池
/// 3. 仓储实现以抽象注入 —— `MySqlUserRepository` 作为 `Arc<dyn UserRepository>`
/// 4. application       —— 构建应用服务（业务编排）
pub struct AppProvider {
    pub config: AppConfig,
    pub services: Services,
}

impl AppProvider {
    /// 显式初始化所有依赖（异步阶段：配置 + 连接池 + 依赖注入）。
    ///
    /// 说明：sqlx 连接池的创建需要 Tokio 上下文，
    /// 调用方应通过 `tauri::async_runtime::block_on` 或在异步任务中调用。
    pub async fn init() -> Result<Self> {
        // 1. 配置读取（基础设施）
        let config = AppConfig::load()?;

        // 2. MySQL 连接池（基础设施）
        let pool = init_db_pool(&config.mysql_conf)?;

        // 3. 依赖倒置：具体实现 -> 抽象接口
        let repo: Arc<dyn UserRepository> = Arc::new(new_user_repo(pool));

        // 4. 应用服务（业务编排，只依赖抽象）
        let user_service = UserService::new(repo);
        let services = Services { user_service };
        Ok(Self { config, services })
    }
}
