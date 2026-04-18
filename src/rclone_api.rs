use crate::{entities::RemoteConfig, error::CloudError, setup_conf_dir};
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::{
    collections::HashMap,
    fs::{self, File},
    future::Future,
    io::prelude::*,
    process::Stdio,
};
use tokio::process::Command;

type Result<T> = std::result::Result<T, CloudError>;

pub trait RcloneApi {
    fn list_profiles(&self) -> impl Future<Output = Result<Vec<(String, String)>>>;
    fn create_config(
        &self,
        profile_name: &str,
        domain: &str,
        parameters: HashMap<String, String>,
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

impl Rclone {
    fn cleanup_auth_port() {
        if let Ok(output) = std::process::Command::new("lsof")
            .args(["-t", "-i:53682"])
            .output()
        {
            let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

            if !pid_str.is_empty() {
                for pid in pid_str.lines() {
                    let _ = std::process::Command::new("kill")
                        .arg("-9")
                        .arg(pid)
                        .status();
                    println!("DEBUG: Killed hanging auth process with PID {}", pid);
                }
            }
        }
    }
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

    async fn create_config(
        &self,
        profile_name: &str,
        domain: &str,
        parameters: HashMap<String, String>,
    ) -> Result<String> {
        Self::cleanup_auth_port();

        let current_profiles = self.list_profiles().await.unwrap_or_default();
        if current_profiles
            .iter()
            .any(|(name, _)| name == profile_name)
        {
            println!("DEBUG: Deleting existing profile: {}", profile_name);
            let _ = self.delete_profile(profile_name).await;
        }

        // Base rclone arguments
        let mut args = vec![
            "config".to_string(),
            "create".to_string(),
            profile_name.to_string(),
            domain.to_string(),
        ];

        // Add custom parameters
        for (key, value) in parameters {
            args.push(key);
            args.push(value);
        }

        // Add rclone flags
        args.extend([
            "config_is_local".to_string(),
            "true".to_string(),
            "config_login_port".to_string(),
            "53682".to_string(),
            "--non-interactive".to_string(),
            "--quiet".to_string(),
        ]);

        let mut child = Command::new("rclone")
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| CloudError::RcloneError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: format!("Failed to spawn rclone: {}", e),
            })?;

        let timeout = tokio::time::sleep(std::time::Duration::from_secs(120));
        tokio::pin!(timeout);

        tokio::select! {
            status = child.wait() => {
                match status {
                    Ok(s) if s.success() => {
                        let _ = self.client
                            .post(format!("{}config/reload", self.url))
                            .send()
                            .await;

                        Ok(format!("Profile '{}' created successfully", profile_name))
                    }
                    Ok(s) => {
                        println!("DEBUG: Rclone exited with error: {}", s);
                        let _ = self.delete_profile(profile_name).await;
                        Err(CloudError::RcloneError {
                            status: StatusCode::BAD_REQUEST,
                            message: format!("Rclone failed with status: {}", s),
                        })
                    }
                    Err(e) => {
                        Err(CloudError::RcloneError {
                            status: StatusCode::INTERNAL_SERVER_ERROR,
                            message: format!("Wait error: {}", e),
                        })
                    }
                }
            }
            _ = &mut timeout => {
                println!("DEBUG: Auth timeout reached for {}", profile_name);
                let _ = child.kill().await;
                let _ = self.delete_profile(profile_name).await;

                Err(CloudError::RcloneError {
                    status: StatusCode::GATEWAY_TIMEOUT,
                    message: "Authentication timed out".into(),
                })
            }
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
            setup_conf_dir::setup(profile_name, file_path)?;
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

        let res_json: serde_json::Value =
            response
                .json()
                .await
                .map_err(|err| CloudError::RcloneError {
                    status: StatusCode::IM_A_TEAPOT,
                    message: err.to_string(),
                })?;

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
