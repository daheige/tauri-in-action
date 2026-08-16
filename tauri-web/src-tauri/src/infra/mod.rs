//! 基础设施层：配置读取与持久化实现。
//!
//! 本层实现领域层定义的接口（如 UserRepository），
//! 依赖方向：infra -> domain。
pub mod config;
pub mod persistence;
