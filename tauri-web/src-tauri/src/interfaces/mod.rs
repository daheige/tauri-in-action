//! 接口层：外部交付方式。
//!
//! - `http`：axum Web API（router / handler / 错误映射）
//! - `commands`：Tauri 命令（IPC）
//!
//! 本层不包含业务逻辑，只做请求解析、参数转换与响应/错误映射。
pub mod commands;
pub mod handler;
pub mod routers;
