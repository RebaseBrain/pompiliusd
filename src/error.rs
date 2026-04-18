use reqwest::StatusCode;
use thiserror::Error;

use crate::json_result::to_err;

#[derive(Error, Debug)]
pub enum CloudError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Rclone error: {message}")]
    RcloneError { status: StatusCode, message: String },

    #[error("Convert error: {message}")]
    ConvertError { status: StatusCode, message: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Toml error: {0}")]
    TomlError(#[from] toml::ser::Error),
}

impl From<CloudError> for String {
    fn from(value: CloudError) -> Self {
        match value {
            CloudError::ReqwestError(err) => {
                to_err(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
            CloudError::RcloneError { status, message } => to_err(status, &message),
            CloudError::ConvertError { status, message } => to_err(status, &message),
            CloudError::TomlError(err) => to_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Ошибка создания конфигурационного toml-а: {}", err),
            ),
            CloudError::IoError(err) => to_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Системная ошибка (IO): {}", err),
            ),
        }
    }
}
