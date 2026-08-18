mod error;
mod user_service;

pub use error::ServiceError;
pub use user_service::UserService;

// Services 服务列表
pub struct Services {
    pub user_service: UserService,
}
