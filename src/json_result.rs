use reqwest::StatusCode;
use serde::Serialize;

pub fn to_ok<T: Serialize>(status_code: StatusCode, data: T) -> String {
    serde_json::json!({
        "status_code": status_code.to_string(),
        "data": serde_json::to_string(&data) .expect("Он должен быть Serialize")
    })
    .to_string()
}

pub fn to_err(status_code: StatusCode, error: String) -> String {
    serde_json::json!({
        "status_code": status_code.to_string(),
        "message": error
    })
    .to_string()
}
