use crate::cache::get_all_files;
use crate::{entities::*, error::CloudError, setup_conf_dir};
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::{collections::HashMap, future::Future, process::Stdio};
use tokio::process::Command;

type Result<T> = std::result::Result<T, CloudError>;

pub trait RcloneApi {
    fn delete_cache_path(
        &self,
        profile_name: &str,
        remote_path: &str,
    ) -> impl Future<Output = Result<String>>;
    fn list_profiles(&self) -> impl Future<Output = Result<Vec<(String, String)>>>;
    fn get_provider_options(
        &self,
        provider_type: &str,
    ) -> impl Future<Output = Result<Vec<serde_json::Value>>>;
    fn create_config(
        &self,
        profile_name: &str,
        domain: &str,
        parameters: &str,
    ) -> impl Future<Output = Result<String>>;
    fn delete_profile(&self, profile_name: &str) -> impl Future<Output = Result<String>>;
    fn mount(
        &self,
        profile_name: &str,
        file_path: &str,
        cache_max_size: &str,
        cache_max_age: &str,
    ) -> impl Future<Output = Result<String>>;
    fn link(&self, profile_name: &str, path: &str) -> impl Future<Output = Result<String>>;
    fn cache_directory(&self, path: &str) -> impl Future<Output = Result<String>>;
    fn refresh(&self, profile_name: &str, path: &str) -> impl Future<Output = Result<String>>;
    fn delete_cache_file(
        &self,
        profile_name: &str,
        path: &str,
    ) -> impl Future<Output = Result<String>>;

    fn delete_cache_directory(
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
    async fn delete_cache_path(&self, profile_name: &str, remote_path: &str) -> Result<String> {
        let cache_base = format!(
            "{}/.cache/rclone/vfs/{}/",
            std::env::var("HOME").unwrap(),
            profile_name
        );
        let full_path = Path::new(&cache_base).join(remote_path);

        if full_path.exists() {
            if full_path.is_dir() {
                fs::remove_dir_all(&full_path).map_err(CloudError::IoError)?;
            } else {
                fs::remove_file(&full_path).map_err(CloudError::IoError)?;
            }

            if full_path.is_dir() {
                let _ = self.delete_cache_directory(profile_name, remote_path).await;
            } else {
                let _ = self.delete_cache_file(profile_name, remote_path).await;
            }

            Ok(format!(
                "Local cache for {} deleted. File will be downloaded again on request.",
                remote_path
            ))
        } else {
            Ok("File is not cached".to_string())
        }
    }
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

    async fn get_provider_options(&self, provider_type: &str) -> Result<Vec<serde_json::Value>> {
        let response = self
            .client
            .post(format!("{}config/providers", self.url))
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        let data: ProvidersResponse =
            response.json().await.map_err(|e| CloudError::RcloneError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: format!("Failed to parse providers: {}", e),
            })?;

        let provider = data
            .providers
            .into_iter()
            .find(|p| p.name == provider_type)
            .ok_or_else(|| CloudError::RcloneError {
                status: StatusCode::NOT_FOUND,
                message: format!("Provider '{}' not found", provider_type),
            })?;

        // Filter required and non-default options
        let filtered_options: Vec<serde_json::Value> = provider
            .options
            .into_iter()
            .filter(|opt| {
                !["token", "config_is_local", "config_login_port"].contains(&opt.name.as_str())
                    && opt.required
            })
            .map(|opt| {
                json!({
                    "Name": opt.name,
                    "Help": opt.help
                })
            })
            .collect();

        Ok(filtered_options)
    }

    async fn create_config(
        &self,
        profile_name: &str,
        domain: &str,
        parameters: &str,
    ) -> Result<String> {
        Self::cleanup_auth_port();
        let params = serde_json::from_str::<CreateParameters>(parameters)?.into_string_map();

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
        for (key, value) in params {
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

    async fn mount(
        &self,
        profile_name: &str,
        file_path: &str,
        cache_max_size: &str,
        cache_max_age: &str,
    ) -> Result<String> {
        let mount_path = std::path::Path::new(file_path).join(profile_name);
        std::fs::create_dir_all(&mount_path).map_err(CloudError::IoError)?;
        let mount_path_str = mount_path.to_string_lossy().to_string();

        let cache_max_size = format!(
            "{}G",
            cache_max_size
                .to_lowercase()
                .parse::<u32>()
                .map_err(|err| CloudError::ConvertError {
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                    message: err.to_string(),
                })?
        );

        let cache_max_age = format!(
            "{}h",
            cache_max_age.to_lowercase().parse::<u32>().map_err(|err| {
                CloudError::ConvertError {
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                    message: err.to_string(),
                }
            })?
        );

        let body = json!({
            "fs": format!("{}:", profile_name),
            "mountPoint": mount_path_str,
            "vfsOpt": {
                "CacheMode": "full",
                "CacheMaxAge": &cache_max_age,
                "CacheMaxSize": &cache_max_size,
                "CachePollInterval": "1s",
                "ReadAhead": 0,

                "AttrTimeout": "0s",
                "PollInterval": "10s",
                "DirCacheTime": "10s",

                "NoChecksum": false,
                "NoModTime": false,
            }
        });

        println!("{body}");

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

    async fn cache_directory(&self, path: &str) -> Result<String> {
        let mut file_paths = Vec::new();

        // 1. Собираем все файлы рекурсивно
        get_all_files(Path::new(path), &mut file_paths);

        if file_paths.is_empty() {
            println!("Файлы не найдены.");
            return Ok("Empty dir".to_string());
        }

        let _ = Command::new("cat")
            .args(&file_paths)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| CloudError::RcloneError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: format!("Failed to spawn rclone: {}", e),
            })?;

        Ok("Cached".to_string())
    }

    async fn refresh(&self, profile_name: &str, path: &str) -> Result<String> {
        let body = json!({
            "fs": format!("{}:", profile_name),
            "file": path,
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
            Ok(format!("Success: File {} cached", path))
        } else {
            Err(CloudError::RcloneError {
                status: StatusCode::CONFLICT,
                message: "Failed to cache file".into(),
            })
        }
    }

    async fn delete_cache_file(&self, profile_name: &str, path: &str) -> Result<String> {
        let body = json!({
            "fs": format!("{}:", profile_name),
            "file": path,
        });

        let response = self
            .client
            .post(format!("{}vfs/forget", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        if response.status().is_success() {
            Ok(format!("Success: {} evicted from local cache", path))
        } else {
            Err(CloudError::RcloneError {
                status: StatusCode::CONFLICT,
                message: "Failed to evict from cache".into(),
            })
        }
    }

    async fn delete_cache_directory(&self, profile_name: &str, path: &str) -> Result<String> {
        let body = json!({
            "fs": format!("{}:", profile_name),
            "dir": path,
        });

        let response = self
            .client
            .post(format!("{}vfs/forget", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        if response.status().is_success() {
            Ok(format!("Success: {} evicted from local cache", path))
        } else {
            Err(CloudError::RcloneError {
                status: StatusCode::CONFLICT,
                message: "Failed to evict from cache".into(),
            })
        }
    }
}
