use crate::infra::errors::ServiceError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// HTTP 统一错误响应。
///
/// 接口层把应用层错误（ServiceError）映射为 HTTP 状态码：
/// - Validation -> 400
/// - NotFound   -> 404
/// - Repo       -> 500
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl From<ServiceError> for ApiError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::Validation(msg) => ApiError {
                status: StatusCode::BAD_REQUEST,
                message: msg,
            },
            ServiceError::NotFound(msg) => ApiError {
                status: StatusCode::NOT_FOUND,
                message: msg,
            },
            ServiceError::Repo(err) => ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: err.to_string(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            tracing::error!("api error {}: {}", self.status, self.message);
        } else {
            tracing::warn!("api error {}: {}", self.status, self.message);
        }
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
