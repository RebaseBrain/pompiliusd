use reqwest::Client;
use zbus::interface;
mod zbus_error;

trait RcloneApi {
    async fn list_remotes(&self) -> zbus::fdo::Result<Vec<String>>;
}

pub struct RcClone {
    pub client: Client,
    pub url: String,
}
use serde::Deserialize;

use crate::zbus_error::CloudsErrors;

#[derive(Deserialize)]
struct ListRemotesResponse {
    remotes: Vec<String>,
}

#[interface(name = "org.zbus.cloud_api")]
impl RcloneApi for RcClone {
    async fn list_remotes(&self) -> zbus::fdo::Result<Vec<String>> {
        let response = self
            .client
            .post(format!("{}config/listremotes", self.url))
            .send()
            .await
            .map_err(CloudsErrors::ReqwestError)?;

        Ok(response
            .json::<ListRemotesResponse>()
            .await
            .map_err(CloudsErrors::ReqwestError)?
            .remotes)
    }
}
