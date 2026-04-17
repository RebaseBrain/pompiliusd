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
    fn mount(&self, profile_name: &str, domain: &str) -> impl Future<Output = String>;
    fn link(&self, profile_name: &str, path: &str) -> impl Future<Output = String>;
}

pub struct Cloud {
    pub rclone: RcClone,
}

#[interface(name = "org.zbus.cloud_api")]
impl CloudApi for Cloud {
    async fn list_profiles(&self) -> String {
        match self.rclone.list_profiles().await {
            Ok(profiles) => to_ok(StatusCode::OK, profiles),
            Err(e) => e.into(),
        }
    }

    async fn create_profile(&self, profile_name: &str, domain: &str) -> String {
        match self.rclone.create_config(profile_name, domain).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.to_string(),
        }
    }

    async fn delete_profile(&self, profile_name: &str) -> String {
        match self.rclone.delete_config(profile_name).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.to_string(),
        }
    }

    async fn mount(&self, profile_name: &str, domain: &str) -> String {
        todo!()
    }

    async fn link(&self, profile_name: &str, path: &str) -> String {
        match self.rclone.link(profile_name, path).await {
            Ok(url) => to_ok(StatusCode::OK, url),
            Err(e) => e.into(),
        }
    }
}
