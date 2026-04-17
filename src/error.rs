use thiserror::Error;

#[derive(Error, Debug)]
pub enum CloudError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Rclone error: {0}")]
    RcloneError(String),
}
