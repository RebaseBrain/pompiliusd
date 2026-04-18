use crate::{
    entities::{ConfigCreateRequest, RemoteConfig},
    error::CloudError,
};
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::io::prelude::*;
use std::{
    collections::HashMap,
    fs::{self, File},
    process::Command,
};
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
    fn cache_directory(
        &self,
        profile_name: &str,
        path: &str,
    ) -> impl Future<Output = Result<String>>;
}

pub struct Rclone {
    pub client: Client,
    pub url: String,
}

impl RcloneApi for Rclone {
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
        if let Ok(profiles) = self.list_profiles().await
            && profiles.iter().any(|(name, _)| name == profile_name)
        {
            println!(
                "DEBUG: Found existing profile {}, deleting before recreation...",
                profile_name
            );
            let _ = self.delete_profile(profile_name).await;
        }

        // Spawn rclone as a separate child process for auth
        // Replace "rclone.conf" with actual path if it's not default.
        let mut child = Command::new("rclone")
            .args([
                "config",
                "create",
                profile_name,
                domain,
                "--config-is-local=true",
                "--config-login-port=53682",
                "--non-interactive",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| CloudError::RcloneError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: format!("Failed to spawn rclone process: {}", e),
            })?;

        let timeout_duration = std::time::Duration::from_secs(60);
        let timeout = tokio::time::sleep(timeout_duration);

        let response = self
            .client
            .post(format!("{}config/create?_async=true", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        let job_info: serde_json::Value =
            response.json().await.map_err(|e| CloudError::RcloneError {
                status: StatusCode::BAD_GATEWAY,
                message: e.to_string(),
            })?;
        let job_id = job_info["jobid"]
            .as_i64()
            .ok_or_else(|| CloudError::RcloneError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "No jobid returned".into(),
            })?;

        // Timeout
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(30);

        while start_time.elapsed() < timeout {
            let status_response = self
                .client
                .post(format!("{}job/status", self.url))
                .json(&serde_json::json!({ "jobid": job_id }))
                .send()
                .await;

            if let Ok(res) = status_response {
                let status_data: serde_json::Value = res.json().await.unwrap_or_default();

                if status_data["finished"].as_bool().unwrap_or(false) {
                    if status_data["error"].is_null()
                        || status_data["error"].as_str().unwrap_or("").is_empty()
                    {
                        return Ok(format!("Success: Profile {} created", profile_name));
                    } else {
                        let _ = self.delete_profile(profile_name).await;
                        return Err(CloudError::RcloneError {
                            status: StatusCode::BAD_REQUEST,
                            message: "Auth failed".into(),
                        });
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        let _ = self
            .client
            .post(format!("{}job/stop", self.url))
            .json(&serde_json::json!({ "jobid": job_id }))
            .send()
            .await;

        // Remove invalid profile
        let _ = self.delete_profile(profile_name).await;

        Err(CloudError::RcloneError {
            status: StatusCode::GATEWAY_TIMEOUT,
            message: "Auth timeout, try again".into(),
        })
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
               // "CacheMaxAge": "3600s",
                "CacheMaxSize": "10G",
               // "CachePollInterval": "1m"
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
            fs::create_dir(&format!("{}/.pompiliuys", profile_name))?;
            let path = format!("{}/{}/.pompiliuys/config", profile_name, file_path);
            let mut file = File::create(&path)?;
            file.write_all(profile_name.as_bytes())?;
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

    async fn cache_directory(&self, profile_name: &str, path: &str) -> Result<String> {
        let body = json!({
            "fs": format!("{}:", profile_name),
            "dir": path,
            "recursive": true,
            "prefetch": true,
            "_async": true
        });

        let response = self
            .client
            .post(format!("{}vfs/refresh", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        if response.status().is_success() {
            Ok(format!("Success: {} cached", path))
        } else {
            Err(CloudError::RcloneError {
                status: StatusCode::CONFLICT,
                message: "Failed to cache".into(),
            })
        }
    }
}
