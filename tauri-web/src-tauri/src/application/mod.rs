//! 应用层：业务逻辑编排。
//!
//! 依赖领域层的仓储抽象（UserRepository）与 DTO，
//! 不感知任何框架与具体实现（面向接口编程）。
pub mod dto;
pub mod services;
