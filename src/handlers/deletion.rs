use crate::database::files::{get_file_version_details, delete_file_versions};
use crate::models::files::FileVersionInfo;
use super::storage_api::delete_paths_from_server;
use crate::database::user::update_quota;
use deadpool_postgres::Pool as PgPool;
use crate::models::errors::AppError;
use crate::state::API_SETTINGS;
use std::collections::HashMap;
use uuid::Uuid;



pub async fn delete_folder_and_contents(db_pool: &PgPool, owner_id: Uuid, folder_id: Uuid) -> Result<(), AppError> {
    // Given a folder ID - delete the folder and all its contents recursively, update the quota accordingly
    Ok(())
}


pub async fn delete_file_and_versions(db_pool: &PgPool, owner_id: Uuid, file_id: Uuid) -> Result<(), AppError> {
    // Get the file version details for the given file ID and owner ID
    let file_info = get_file_version_details(&db_pool, &owner_id, &file_id).await?;
    if file_info.is_empty() {
        return Err(AppError::BadRequest("File not found".into()));
    }

    // Map the versions to their respective hosted servers
    let mut file_locations_map: HashMap<&str, Vec<&FileVersionInfo>> = HashMap::new();

    for info in &file_info {
        file_locations_map
            .entry(info.hosted_at.as_str())
            .or_default()
            .push(info);
    }

    // Delete based on the hosted server locations
    for (hosted_at, file_versions) in &file_locations_map {
        let file_paths: Vec<&str> = file_versions.iter().map(|f| f.file_location.as_str()).collect();
        let file_versions: Vec<i32> = file_versions.iter().map(|f| f.file_version).collect();

        // Delete the file paths from the server
        delete_paths_from_server(
            hosted_at,
            &API_SETTINGS.file_store_api_key,
            &serde_json::json!(file_paths),
        ).await?;

        // Delete the file versions from the database
        let (total_deleted_size, total_deleted_count) = delete_file_versions(&db_pool, &file_id, &file_versions).await?;

        // Update the quota accordingly
        if total_deleted_count > 0 {
            update_quota(
                &db_pool,
                &owner_id,
                hosted_at,
                -total_deleted_size,
                -total_deleted_count
            ).await?;
        }
    }

    Ok(())
}


pub async fn delete_file_version(db_pool: PgPool, owner_id: Uuid, file_id: Uuid, version: i32) -> Result<(), AppError> {
    // Get the file version details for the given file ID and owner ID
    let file_info = get_file_version_details(&db_pool, &owner_id, &file_id).await?;
    if file_info.is_empty() {
        return Err(AppError::BadRequest("File not found".into()));
    }

    // Get specific file version from the list of file version details
    let specific_version = file_info.into_iter().find(|f| f.file_version == version);
    if specific_version.is_none() {
        return Err(AppError::BadRequest("File version not found".into()));
    }
    let specific_version = specific_version.unwrap();

    // Delete from server
    delete_paths_from_server(
        &specific_version.hosted_at,
        &API_SETTINGS.file_store_api_key,
        &serde_json::json!([specific_version.file_location]),
    ).await?;

    // Delete from database
    let (total_deleted_size, total_deleted_count) = delete_file_versions(&db_pool, &file_id, &[version]).await?;

    // Update the quota accordingly
    if total_deleted_count > 0 {
        update_quota(
            &db_pool,
            &owner_id,
            &specific_version.hosted_at,
            -total_deleted_size,
            -total_deleted_count
        ).await?;
    }

    Ok(())
}
