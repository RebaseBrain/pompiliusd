use reqwest::{Client, StatusCode};
use std::collections::HashMap;

use crate::{entities::{ConfigCreateRequest, ListRemotesResponse}, error::CloudError};
type Result<T> = std::result::Result<T, CloudError>;


pub trait RcloneApi {
    fn list_profiles(&self) -> impl Future<Output = Result<Vec<String>>>;
    fn create_config(
        &self,
        profile_name: &str,
        domain: &str,
    ) -> impl Future<Output = Result<String>>;

    fn delete_config(&self, profile_name: &str) -> impl Future<Output = Result<String>>;
    fn mount(&self, profile_name: &str, domen: &str) -> impl Future<Output = Result<String>>;
    fn link(&self, profile_name: &str, path: &str) -> impl Future<Output = Result<String>>;
}

pub struct RcClone {
    pub client: Client,
    pub url: String,
}

impl RcloneApi for RcClone {
    async fn list_profiles(&self) -> Result<Vec<String>> {
        let response = self
            .client
            .post(format!("{}config/listremotes", self.url))
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        let data: ListRemotesResponse = response
            .json()
            .await
            .map_err(|err| CloudError::RcloneError((StatusCode::IM_A_TEAPOT, err.to_string())))?;

        Ok(data.remotes)
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
            Err(CloudError::RcloneError((
                StatusCode::CONFLICT,
                "Failed to create profile".into(),
            )))
        }
    }

    async fn delete_config(&self, profile_name: &str) -> Result<String> {
        let body = HashMap::from([("name", profile_name)]);

        self.client
            .post(format!("{}config/delete", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        Ok(format!("Success: Profile {} deleted", profile_name))
    }

    async fn mount(&self, profile_name: &str, _domain: &str) -> Result<String> {
        let body = HashMap::from([
            ("fs", profile_name.to_string() + ":"),
            ("mountPoint", format!("/tmp/{}", profile_name)),
        ]);

        self.client
            .post(format!("{}mount/mount", self.url))
            .json(&body)
            .send()
            .await
            .map_err(CloudError::ReqwestError)?;

        Ok(format!("Mounting {} started", profile_name))
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
        let res_json: serde_json::Value = response
            .json()
            .await
            .map_err(|err| CloudError::RcloneError((StatusCode::IM_A_TEAPOT, err.to_string())))?;

        // Печатаем в консоль Rust-приложения, что прислал rclone
        println!("Rclone link response: {:?}", res_json);

        match res_json["url"].as_str() {
            Some(url) => Ok(url.to_string()),
            None => Err(CloudError::RcloneError((
                StatusCode::NOT_FOUND,
                "No link generated".to_string(),
            ))),
        }
    }
}
