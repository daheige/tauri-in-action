use serde::{Deserialize, Serialize};

/// 分页查询参数（page/page_size 均可省略，业务规则在应用服务中处理）
#[derive(Debug, Default, Deserialize)]
pub struct PageQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 分页结果
#[derive(Debug, Clone, Serialize)]
pub struct PageResult<T> {
    /// 总记录数
    pub total: u64,
    /// 当前页码（从 1 开始）
    pub page: u64,
    /// 每页条数
    pub page_size: u64,
    /// 当前页数据
    pub items: Vec<T>,
}

/// 新增用户入参
#[derive(Debug, Deserialize)]
pub struct CreateUserInput {
    pub username: String,
}

/// 更新用户入参
#[derive(Debug, Deserialize)]
pub struct UpdateUserInput {
    pub username: String,
}
