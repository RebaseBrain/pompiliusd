pub enum CloudsErrors {
    ReqwestError(reqwest::Error),
}

impl From<CloudsErrors> for zbus::fdo::Error {
    fn from(value: CloudsErrors) -> Self {
        match value {
            CloudsErrors::ReqwestError(er) => Self::NoNetwork(er.to_string())
        }
    }
}
