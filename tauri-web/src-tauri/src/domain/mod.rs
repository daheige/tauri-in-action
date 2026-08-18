//! 领域层：实体、仓储接口与仓储错误。
//!
//! 本层不依赖任何框架/基础设施（axum、sqlx、tauri 均不可见），
//! 只定义业务抽象，实现由基础设施层（infra/persistence）提供。
pub mod entity;
pub mod repository;
