use crate::domain::entity::User;
use crate::infra::errors::RepoError;
use async_trait::async_trait;

/// 用户仓储接口（面向接口编程的核心抽象）。
///
/// - 接口定义在领域层（domain/repository）
/// - 实现放在基础设施层（infra/persistence，基于 sqlx MySQL）
/// - 应用层（application/services）只依赖本接口，不感知具体实现
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// 数据库连通性探测
    async fn ping(&self) -> Result<(), RepoError>;

    /// 总记录数
    async fn count(&self) -> Result<i64, RepoError>;

    /// 分页查询（offset/limit 由应用层计算后传入）
    async fn find_page(&self, offset: i64, limit: i64) -> Result<Vec<User>, RepoError>;

    /// 全量查询（Tauri 命令演示场景）
    async fn find_all(&self) -> Result<Vec<User>, RepoError>;

    /// 按 id 查询，不存在返回 None
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, RepoError>;

    /// 新增，返回创建后的完整实体
    async fn create(&self, username: &str) -> Result<User, RepoError>;

    /// 更新，返回更新后的实体（不存在返回 None）
    async fn update(&self, id: u64, username: &str) -> Result<Option<User>, RepoError>;

    /// 删除，返回是否真正删除了记录
    async fn delete(&self, id: u64) -> Result<bool, RepoError>;
}
