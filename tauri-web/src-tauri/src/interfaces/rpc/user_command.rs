use crate::application::service::UserService;
use crate::domain::entity::User;
use tauri::State;

/// Tauri 命令：全量查询 users（演示 interfaces -> application -> domain(repo) 链路）。
///
/// 依赖通过 Tauri 状态注入（组合根在启动时注册），
/// 命令内部不触碰任何具体实现。
#[tauri::command]
pub async fn get_users(service: State<'_, UserService>) -> Result<Vec<User>, String> {
    service.list_all().await.map_err(|e| e.to_string())
}
