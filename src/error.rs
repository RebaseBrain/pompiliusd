use reqwest::StatusCode;
use thiserror::Error;

use crate::json_result::to_err;

#[derive(Error, Debug)]
pub enum CloudeError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Rclone error")]
    RcloneError((StatusCode, String)),
}

impl From<CloudeError> for String {
    fn from(value: CloudeError) -> Self {
        match value {
            CloudeError::ReqwestError(err) => {
                to_err(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
            CloudeError::RcloneError((status, err)) => to_err(status, &err),
        }
    }
}
