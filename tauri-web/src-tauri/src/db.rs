use crate::config::MysqlConf;
use anyhow::{Context, Result};
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions, MySqlSslMode};
use std::str::FromStr;
use std::time::Duration;

/// 根据配置创建 MySQL 连接池。
///
/// 使用 lazy 模式：启动时不强制连接成功，数据库暂不可用时应用仍可启动，
/// 首次请求时再建立连接（健康检查接口可用来探测数据库状态）。
pub fn init_pool(conf: &MysqlConf) -> Result<MySqlPool> {
    // 注: sqlx 0.8 的 PoolOptions 无 connect_timeout，acquire_timeout 即连接获取超时
    let opts = MySqlConnectOptions::from_str(&conf.dsn)
        .with_context(|| format!("解析 mysql dsn 失败: {}", conf.dsn))?
        .ssl_mode(MySqlSslMode::Disabled);

    let pool = MySqlPoolOptions::new()
        .max_connections(conf.max_connections)
        .min_connections(conf.min_connections)
        .max_lifetime(Duration::from_secs(conf.max_lifetime))
        .idle_timeout(Duration::from_secs(conf.idle_timeout))
        .acquire_timeout(Duration::from_secs(conf.connect_timeout))
        .connect_lazy_with(opts);

    Ok(pool)
}
