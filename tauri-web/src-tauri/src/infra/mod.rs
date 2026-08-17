//! 基础设施层：配置读取、持久化实现与错误定义。
//!
//! 本层实现领域层定义的接口（如 UserRepository），
//! 依赖方向：infra -> domain。
pub mod config;
pub mod persistence;

pub mod errors;
