use crate::application::dto::page::{PageQuery, PageResult};
use crate::application::dto::user::{CreateUserInput, UpdateUserInput};
use crate::application::services::error::ServiceError;
use crate::domain::entity::User;
use crate::domain::repository::UserRepository;
use std::sync::Arc;

/// 用户应用服务：业务逻辑编排。
///
/// 只依赖 `UserRepository` 抽象（依赖倒置），
/// 具体实现由组合根（providers）在初始化时注入。
#[derive(Clone)]
pub struct UserService {
    repo: Arc<dyn UserRepository>,
}

impl UserService {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    /// 数据库连通性探测
    pub async fn ping(&self) -> Result<(), ServiceError> {
        self.repo.ping().await?;
        Ok(())
    }

    /// 分页查询：业务规则（页码/页大小默认值与上限、偏移计算）
    pub async fn page(&self, query: PageQuery) -> Result<PageResult<User>, ServiceError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);
        let offset = (page - 1) * page_size;

        let total = self.repo.count().await? as u64;
        let items = self.repo.find_page(offset as i64, page_size as i64).await?;

        Ok(PageResult {
            total,
            page,
            page_size,
            items,
        })
    }

    /// 全量查询（Tauri 命令演示场景）
    pub async fn list_all(&self) -> Result<Vec<User>, ServiceError> {
        self.repo.find_all().await.map_err(Into::into)
    }

    /// 按 id 查询，不存在抛出 NotFound
    pub async fn get(&self, id: u64) -> Result<User, ServiceError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("用户 id={id} 不存在")))
    }

    /// 新增用户
    pub async fn create(&self, input: CreateUserInput) -> Result<User, ServiceError> {
        let username = validate_username(&input.username)?;
        self.repo.create(&username).await.map_err(Into::into)
    }

    /// 更新用户，不存在抛出 NotFound
    pub async fn update(&self, id: u64, input: UpdateUserInput) -> Result<User, ServiceError> {
        let username = validate_username(&input.username)?;
        self.repo
            .update(id, &username)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("用户 id={id} 不存在")))
    }

    /// 删除用户，不存在抛出 NotFound
    pub async fn delete(&self, id: u64) -> Result<(), ServiceError> {
        let deleted = self.repo.delete(id).await?;
        if !deleted {
            return Err(ServiceError::NotFound(format!("用户 id={id} 不存在")));
        }
        Ok(())
    }
}

/// 业务规则：用户名非空、去首尾空白、长度 ≤ 100（与表结构 varchar(100) 一致）
fn validate_username(username: &str) -> Result<String, ServiceError> {
    let username = username.trim();
    if username.is_empty() {
        return Err(ServiceError::Validation("用户名不能为空".into()));
    }
    if username.chars().count() > 100 {
        return Err(ServiceError::Validation(
            "用户名长度不能超过 100 个字符".into(),
        ));
    }
    Ok(username.to_string())
}
