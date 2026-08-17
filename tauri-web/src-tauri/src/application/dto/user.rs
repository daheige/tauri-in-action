use serde::Deserialize;

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
