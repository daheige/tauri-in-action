use crate::domain::entity::User;
use crate::domain::repository::{RepoError, UserRepository};
use async_trait::async_trait;
use sqlx::mysql::{MySqlPool, MySqlRow};
use sqlx::Row;

/// UserRepository 接口的 MySQL 实现（基于 sqlx）。
///
/// 这是基础设施层对领域接口的实现，可通过 providers 组合根
/// 以抽象类型注入到应用服务（依赖倒置）。
pub struct UserRepoImpl {
    pool: MySqlPool,
}

pub fn new_user_repo(pool: MySqlPool) -> impl UserRepository {
    UserRepoImpl { pool }
}

// 推荐使用 new_user_repo 函数方式创建 UserRepository trait 对象
// impl UserRepoImpl {
//     pub fn new(pool: MySqlPool) -> impl UserRepository {
//         Self { pool }
//     }
// }

/// 将 sqlx 的数据库错误映射为 RepoError
impl From<sqlx::Error> for RepoError {
    fn from(e: sqlx::Error) -> Self {
        RepoError::Db(e.to_string())
    }
}

/// 数据库行 -> 领域实体的手动映射（保持领域层不依赖 sqlx）
impl<'r> sqlx::FromRow<'r, MySqlRow> for User {
    fn from_row(row: &'r MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(User {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
        })
    }
}

#[async_trait]
impl UserRepository for UserRepoImpl {
    async fn ping(&self) -> Result<(), RepoError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    async fn count(&self) -> Result<i64, RepoError> {
        let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(total)
    }

    async fn find_page(&self, offset: i64, limit: i64) -> Result<Vec<User>, RepoError> {
        let items = sqlx::query_as::<_, User>(
            "SELECT id, username FROM users ORDER BY id LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(items)
    }

    async fn find_all(&self) -> Result<Vec<User>, RepoError> {
        let items = sqlx::query_as::<_, User>("SELECT id, username FROM users ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        Ok(items)
    }

    async fn find_by_id(&self, id: u64) -> Result<Option<User>, RepoError> {
        let user = sqlx::query_as::<_, User>("SELECT id, username FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(user)
    }

    async fn create(&self, username: &str) -> Result<User, RepoError> {
        let result = sqlx::query("INSERT INTO users (username) VALUES (?)")
            .bind(username)
            .execute(&self.pool)
            .await?;
        let id = result.last_insert_id();

        let user = sqlx::query_as::<_, User>("SELECT id, username FROM users WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(user)
    }

    async fn update(&self, id: u64, username: &str) -> Result<Option<User>, RepoError> {
        let result = sqlx::query("UPDATE users SET username = ? WHERE id = ?")
            .bind(username)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }

        let user = sqlx::query_as::<_, User>("SELECT id, username FROM users WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(Some(user))
    }

    async fn delete(&self, id: u64) -> Result<bool, RepoError> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
