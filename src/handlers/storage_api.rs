use crate::models::errors::AppError;
use serde_json::Value as JsonValue;
use reqwest::Client;
use uuid::Uuid;


/// Builds the file location string for a given file info
pub fn build_file_location(
    base_folder_path: &str,
    org_id: &Uuid,
    user_id: &Uuid,
    folder_id: &Uuid,
    file_id: &Uuid,
    file_version: i32,
) -> String {
    // Example Location
    // "/data/yfs/org-id/user-id/folder-id/file-id.version-count"
    format!(
        "{}/{}/{}/{}/{}.{}",
        base_folder_path,
        org_id,
        user_id,
        folder_id,
        file_id,
        file_version
    )
}


/// Request to generate an upload sessions for the storage server
pub async fn generate_upload_sessions<T: ToString>(base_url: &str, api_key: &str, body: &T) -> Result<JsonValue, AppError> {
    let client = Client::new();
    let url = format!("{}/sessions/upload", base_url);
    let response = client.post(&url)
        .header("X-API-Token", api_key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;

    if response.status().is_success() {
        let json_response = response.json::<JsonValue>().await?;
        Ok(json_response)
    } else {
        let error_text = response.text().await?;
        Err(AppError::BadRequest(format!("Failed to generate upload session: {}", error_text)))
    }
}


/// Request to generate a download sessions for the storage server
pub async fn generate_download_sessions<T: ToString>(base_url: &str, api_key: &str, body: &T) -> Result<JsonValue, AppError> {
    let client = Client::new();
    let url = format!("{}/sessions/download", base_url);
    let response = client.post(&url)
        .header("X-API-Token", api_key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;

    if response.status().is_success() {
        let json_response = response.json::<JsonValue>().await?;
        Ok(json_response)
    } else {
        let error_text = response.text().await?;
        Err(AppError::BadRequest(format!("Failed to generate download session: {}", error_text)))
    }
}


/// Request to generate a WOPI session for the storage server
pub async fn generate_wopi_session<T: ToString>(base_url: &str, api_key: &str, body: &T) -> Result<JsonValue, AppError> {
    let client = Client::new();
    let url = format!("{}/sessions/wopi", base_url);
    let response = client.post(&url)
        .header("X-API-Token", api_key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;

    if response.status().is_success() {
        let json_response = response.json::<JsonValue>().await?;
        Ok(json_response)
    } else {
        let error_text = response.text().await?;
        Err(AppError::BadRequest(format!("Failed to create WOPI session: {}", error_text)))
    }
}


/// Delete multiple file/folder paths from the storage server
pub async fn delete_paths_from_server<T: ToString>(base_url: &str, api_key: &str, body: &T) -> Result<(), AppError> {
    let client = Client::new();
    let url = format!("{}/files/delete", base_url);
    let response = client.post(&url)
        .header("X-API-Token", api_key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await?;
        Err(AppError::BadRequest(format!("Failed to delete paths: {}", error_text)))
    }
}


/// Generate a folder download session from the YFS-Archive-API (alias for the Download Manager)
pub async fn generate_folder_download_session<T: ToString>(base_url: &str, api_key: &str, body: &T) -> Result<JsonValue, AppError> {
    let client = Client::new();
    let url = format!("{}/internal/archives", base_url);
    let response = client.post(&url)
        .header("X-API-Token", api_key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;

    if response.status().is_success() {
        let json_response = response.json::<JsonValue>().await?;
        Ok(json_response)
    } else {
        let error_text = response.text().await?;
        Err(AppError::BadRequest(format!("Failed to generate folder download session: {}", error_text)))
    }
}
