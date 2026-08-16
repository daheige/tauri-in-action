use serde::Serialize;

/// 用户领域实体（对应 users 表）。
///
/// 纯业务结构，不含任何 ORM/框架注解；
/// 数据库行到实体的映射由基础设施层（infra/persistence）负责。
#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: u64,
    pub username: String,
}
