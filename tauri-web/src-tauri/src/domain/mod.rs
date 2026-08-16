//! 领域层：实体与仓储接口。
//!
//! 本层不依赖任何框架/基础设施（axum、sqlx、tauri 均不可见），
//! 只定义业务抽象，实现由基础设施层（infra/persistence）提供。
pub mod entity;
pub mod repository;
