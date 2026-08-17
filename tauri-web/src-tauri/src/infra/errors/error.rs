/// 仓储层错误。
///
/// 定义在领域层，具体数据库错误由基础设施层实现时映射到此类型。
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("数据库访问失败: {0}")]
    Db(String),
}

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
