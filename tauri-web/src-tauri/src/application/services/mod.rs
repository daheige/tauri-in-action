mod user_service;

pub use user_service::UserService;

// Services 服务列表
pub struct Services {
    pub user_service: UserService,
}
