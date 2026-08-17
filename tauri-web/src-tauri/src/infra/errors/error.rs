//! 错误类型：仓储错误与应用层错误。
//!
//! `RepoError` 由基础设施层实现数据库访问时映射 sqlx 错误而来，
//! `ServiceError` 面向接口层暴露业务错误语义（校验/不存在/数据访问失败）。

/// 应用层错误：面向接口层暴露的业务错误语义
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// 参数/业务规则校验失败（接口层映射为 400）
    #[error("{0}")]
    Validation(String),
    /// 资源不存在（接口层映射为 404）
    #[error("资源不存在: {0}")]
    NotFound(String),
    /// 数据访问失败（接口层映射为 500）
    #[error("数据访问失败: {0}")]
    Repo(#[from] RepoError),
}

/// 仓储层错误：基础设施层将 sqlx 错误映射为此类型
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("数据库访问失败: {0}")]
    Db(String),
}
