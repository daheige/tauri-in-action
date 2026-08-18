/// 仓储层错误：基础设施层将 sqlx 错误映射为此类型
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("数据库访问失败: {0}")]
    Db(String),
}
