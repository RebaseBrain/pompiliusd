use crate::{
    entities::{ConfigCreateRequest, RemoteConfig},
    error::CloudError,
};
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::collections::HashMap;
type Result<T> = std::result::Result<T, CloudError>;

pub trait RcloneApi {
    fn list_profiles(&self) -> impl Future<Output = Result<Vec<(String, String)>>>;
    fn create_config(
        &self,
        profile_name: &str,
        domain: &str,
    ) -> impl Future<Output = Result<String>>;
    fn delete_profile(&self, profile_name: &str) -> impl Future<Output = Result<String>>;
    fn mount(&self, profile_name: &str, file_path: &str) -> impl Future<Output = Result<String>>;
    fn link(&self, profile_name: &str, path: &str) -> impl Future<Output = Result<String>>;
    fn check_sync(&self, profile_name: &str) -> impl Future<Output = Result<String>>;
    fn check_connection(&self) -> impl Future<Output = Result<String>>;
}

pub struct RcClone {
    pub client: Client,
    pub url: String,
}

impl RcloneApi for RcClone {
    async fn list_profiles(&self) -> Result<Vec<(String, String)>> {
        let response = self
            .client
            .post(format!("{}config/dump", self.url))
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        let data: HashMap<String, RemoteConfig> =
            response
                .json()
                .await
                .map_err(|err| CloudError::RcloneError {
                    status: StatusCode::IM_A_TEAPOT,
                    message: err.to_string(),
                })?;

        Ok(data
            .into_iter()
            .map(|(name, _type)| (name, _type.r#type))
            .collect())
    }

    async fn create_config(&self, profile_name: &str, domain: &str) -> Result<String> {
        let body = ConfigCreateRequest {
            name: profile_name.to_string(),
            r_type: domain.to_string(),
            parameters: HashMap::new(),
        };

        let response = self
            .client
            .post(format!("{}config/create", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        if response.status().is_success() {
            Ok(format!("Success: Profile {} created", profile_name))
        } else {
            Err(CloudError::RcloneError {
                status: StatusCode::CONFLICT,
                message: "Failed to create profile".into(),
            })
        }
    }

    async fn delete_profile(&self, profile_name: &str) -> Result<String> {
        let body = HashMap::from([("name", profile_name)]);

        self.client
            .post(format!("{}config/delete", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        Ok(format!("Success: Profile {} deleted", profile_name))
    }

    async fn mount(&self, profile_name: &str, file_path: &str) -> Result<String> {
        let mount_path = std::path::Path::new(file_path).join(profile_name);
        std::fs::create_dir_all(&mount_path).map_err(CloudError::IoError)?;
        let mount_path_str = mount_path.to_string_lossy().to_string();

        let body = json!({
            "fs": format!("{}:", profile_name),
            "mountPoint": mount_path_str,
            "vfsOpt": {
                "CacheMode": "full",
                "CacheMaxAge": "3600s",
                "CacheMaxSize": "10G",
                "CachePollInterval": "1m"
            }
        });

        let response = self
            .client
            .post(format!("{}mount/mount", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        if response.status().is_success() {
            Ok(format!("Mounting {} started", profile_name))
        } else {
            Err(CloudError::RcloneError {
                status: StatusCode::NOT_FOUND,
                message: "Failed to mount".into(),
            })
        }
    }
    async fn link(&self, profile_name: &str, path: &str) -> Result<String> {
        let body = HashMap::from([
            ("fs", profile_name.to_string() + ":"),
            ("remote", path.to_string()),
        ]);

        let response = self
            .client
            .post(format!("{}operations/publiclink", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        // Читаем весь JSON для отладки
        let res_json: serde_json::Value =
            response
                .json()
                .await
                .map_err(|err| CloudError::RcloneError {
                    status: StatusCode::IM_A_TEAPOT,
                    message: err.to_string(),
                })?;

        // Печатаем в консоль Rust-приложения, что прислал rclone
        println!("Rclone link response: {:?}", res_json);

        match res_json["url"].as_str() {
            Some(url) => Ok(url.to_string()),
            None => Err(CloudError::RcloneError {
                status: StatusCode::NOT_FOUND,
                message: "No link generated".to_string(),
            }),
        }
    }

    async fn check_sync(&self, profile_name: &str) -> Result<String> {
        let body = HashMap::from([("fs", format!("{}:", profile_name))]);

        let response = self
            .client
            .post(format!("{}operations/fsinfo", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        let status = response.status();

        if status.is_success() {
            Ok(format!("Remote '{}' is reachable", profile_name))
        } else {
            Err(CloudError::RcloneError {
                status,
                message: format!("Remote '{}' not reachable", profile_name),
            })
        }
    }

    // Проверка жизнеспособности rclone
    async fn check_connection(&self) -> Result<String> {
        let response = self
            .client
            .post(format!("{}rc/noop", self.url))
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        if response.status().is_success() {
            Ok(format!("Rclone RC is reachable"))
        } else {
            Err(CloudError::RcloneError {
                status: response.status(),
                message: "Rclone responded but not OK".to_string(),
            })
        }
    }
}
