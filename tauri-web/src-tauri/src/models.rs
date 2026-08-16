use serde::Serialize;

/// users 表结构
///
/// ```sql
/// CREATE TABLE `users` (
///     `id` bigint unsigned NOT NULL AUTO_INCREMENT,
///     `username` varchar(100) NOT NULL DEFAULT '',
///     PRIMARY KEY (`id`)
/// ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
/// ```
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: u64,
    pub username: String,
}

/// 分页查询结果：GET /api/users?page=1&page_size=10
#[derive(Debug, Clone, Serialize)]
pub struct UserList {
    /// 总记录数
    pub total: u64,
    /// 当前页码（从 1 开始）
    pub page: u64,
    /// 每页条数
    pub page_size: u64,
    /// 当前页数据
    pub items: Vec<User>,
}
