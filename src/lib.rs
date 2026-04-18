use reqwest::StatusCode;
use zbus::interface;

use crate::{
    json_result::to_ok,
    rclone_api::{RcClone, RcloneApi},
};

pub mod entities;
pub mod error;
pub mod json_result;
pub mod rclone_api;

pub trait CloudApi {
    fn list_profiles(&self) -> impl Future<Output = String>;
    fn create_profile(&self, profile_name: &str, domain: &str) -> impl Future<Output = String>;
    fn delete_profile(&self, profile_name: &str) -> impl Future<Output = String>;
    fn mount(&self, profile_name: &str, file_path: &str) -> impl Future<Output = String>;
    fn link(&self, profile_name: &str, path: &str) -> impl Future<Output = String>;
    fn cache_directory(&self, profile_name: &str, path: &str) -> impl Future<Output = String>;
    fn refresh(&self, profile_name: &str, path: &str) -> impl Future<Output = String>;
    fn delete_cache_file(&self, profile_name: &str, path: &str) -> impl Future<Output = String>;
    fn delete_cache_directory(
        &self,
        profile_name: &str,
        path: &str,
    ) -> impl Future<Output = String>;
    fn delete_cache_path(&self, profile_name: &str, path: &str) -> impl Future<Output = String>;
}

pub struct Cloud {
    pub rclone: RcClone,
}

#[interface(name = "org.zbus.pompiliusd")]
impl CloudApi for Cloud {
    async fn list_profiles(&self) -> String {
        match self.rclone.list_profiles().await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }

    async fn create_profile(&self, profile_name: &str, domain: &str) -> String {
        match self.rclone.create_config(profile_name, domain).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }

    async fn delete_profile(&self, profile_name: &str) -> String {
        match self.rclone.delete_profile(profile_name).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }

    async fn mount(&self, profile_name: &str, file_path: &str) -> String {
        match self.rclone.mount(profile_name, file_path).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }

    async fn link(&self, profile_name: &str, path: &str) -> String {
        match self.rclone.link(profile_name, path).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }

    async fn cache_directory(&self, profile_name: &str, path: &str) -> String {
        match self.rclone.cache_directory(profile_name, path).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }

    async fn refresh(&self, profile_name: &str, path: &str) -> String {
        match self.rclone.refresh(profile_name, path).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }

    async fn delete_cache_file(&self, profile_name: &str, path: &str) -> String {
        match self.rclone.delete_cache_file(profile_name, path).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }

    async fn delete_cache_directory(&self, profile_name: &str, path: &str) -> String {
        match self.rclone.delete_cache_directory(profile_name, path).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }

    async fn delete_cache_path(&self, profile_name: &str, path: &str) -> String {
        match self.rclone.delete_cache_path(profile_name, path).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.into(),
        }
    }
}
