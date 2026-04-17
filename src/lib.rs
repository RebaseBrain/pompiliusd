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

pub trait CloudeApi {
    fn list_profiles(&self) -> impl Future<Output = String>;
    fn config_create(&self, profile_name: &str, domen: &str) -> impl Future<Output = String>;
    fn delete_profile(&self, profile_name: &str) -> impl Future<Output = String>;
    fn mount(
        &self,
        profile_name: &str,
        domen: &str,
        file_path: &str,
    ) -> impl Future<Output = String>;
    fn link(&self, profile_name: &str, path: &str) -> impl Future<Output = String>;
}

pub struct Cloude {
    pub rclone: RcClone,
}

#[interface(name = "org.zbus.cloud_api")]
impl CloudeApi for Cloude {
    async fn list_profiles(&self) -> String {
        todo!()
    }

    async fn config_create(&self, profile_name: &str, domen: &str) -> String {
        todo!()
    }

    async fn delete_profile(&self, profile_name: &str) -> String {
        todo!()
    }

    async fn mount(&self, profile_name: &str, domen: &str, file_path: &str) -> String {
        match self.rclone.mount(profile_name, domen, file_path).await {
            Ok(res) => to_ok(StatusCode::OK, res),
            Err(err) => err.to_string(),
        }
    }

    async fn link(&self, profile_name: &str, path: &str) -> String {
        todo!()
    }
}
