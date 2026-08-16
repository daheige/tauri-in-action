use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// 应用配置，对应项目根目录的 app.yaml
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app_name: String,
    /// 以下字段与标准配置格式保持一致，暂未参与业务逻辑
    #[allow(dead_code)]
    pub app_debug: bool,
    pub app_port: u16,
    #[allow(dead_code)]
    pub monitor_port: u16,
    #[allow(dead_code)]
    pub graceful_wait_time: u64,
    /// 日志级别，优先级从高到低：error > warn > info > debug > trace
    pub log_level: String,
    /// redis 配置（本示例仅保留解析，未实际使用）
    #[serde(default)]
    pub redis_conf: Option<RedisConf>,
    pub mysql_conf: MysqlConf,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // redis 配置仅保留解析，本示例未实际使用
pub struct RedisConf {
    pub dsn: String,
    pub max_size: u32,
    pub min_idle: u32,
    pub max_lifetime: u64,
    pub idle_timeout: u64,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MysqlConf {
    pub dsn: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub max_lifetime: u64,
    pub idle_timeout: u64,
    pub connect_timeout: u64,
}

impl AppConfig {
    /// 加载配置：优先使用 APP_CONFIG 环境变量指定的路径；
    /// 否则按顺序尝试 ./app.yaml（项目根目录运行）与
    /// ../app.yaml（在 src-tauri 目录运行）。
    pub fn load() -> Result<Self> {
        if let Ok(path) = std::env::var("APP_CONFIG") {
            return Self::load_from(&PathBuf::from(path));
        }
        let candidates = [
            PathBuf::from("app.yaml"),
            PathBuf::from("../app.yaml"),
        ];
        let mut last_err = None;
        for path in &candidates {
            match Self::load_from(path) {
                Ok(cfg) => return Ok(cfg),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("未找到配置文件")))
    }

    pub fn load_from(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("读取配置文件失败: {}", path.display()))?;
        let cfg: AppConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("解析配置文件失败: {}", path.display()))?;
        Ok(cfg)
    }
}
