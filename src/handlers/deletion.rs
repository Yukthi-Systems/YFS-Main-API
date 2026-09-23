use crate::database::folders::{mark_folders_and_files_as_deleted, get_files_in_folder, get_subfolders_in_folder, delete_folder_if_empty};
use crate::database::files::{get_file_version_details, delete_file_versions};
use crate::models::files::FileVersionInfo;
use super::storage_api::delete_paths_from_server;
use crate::database::user::update_quota;
use deadpool_postgres::Pool as PgPool;
use crate::models::errors::AppError;
use crate::state::API_SETTINGS;
use std::collections::HashMap;
use uuid::Uuid;



async fn delete_folder_contents_recursively(db_pool: &PgPool, owner_id: Uuid, folder_id: Uuid, current_level: usize) -> Result<(), AppError> {
    // If there is nothing left in the folder, then delete the folder itself
    let is_empty = delete_folder_if_empty(&db_pool, &folder_id).await?;
    if is_empty {
        return Ok(());
    }
    log::trace!("Current level {}: Finished checking if folder with ID: {} is empty", current_level, folder_id);

    let first_level_files = get_files_in_folder(&db_pool, &folder_id).await?;
    let first_level_subfolders = get_subfolders_in_folder(&db_pool, &folder_id).await?;

    // Delete all files in the current folder
    for file_id in first_level_files {
        log::trace!("Current level {}: Deleting file with ID: {} in folder with ID: {}", current_level, file_id, folder_id);
        delete_file_and_versions(&db_pool, owner_id, file_id).await?;
    }

    // Recursively delete all subfolders
    for subfolder_id in first_level_subfolders {
        log::trace!("Current level {}: Recursively deleting subfolder with ID: {} in folder with ID: {}", current_level, subfolder_id, folder_id);
        Box::pin(delete_folder_contents_recursively(db_pool, owner_id, subfolder_id, current_level + 1)).await?;
    }

    // Delete the folder itself if it is now empty
    let is_empty = delete_folder_if_empty(&db_pool, &folder_id).await?;
    if is_empty {
        log::trace!("Current level {}: Deleted folder with ID: {} as it is now empty", current_level, folder_id);
    }

    Ok(())
}


pub async fn delete_folder_and_contents(db_pool: PgPool, owner_id: Uuid, folder_id: Uuid) -> Result<(), AppError> {
    log::trace!("Deleting folder with ID: {} for owner ID: {}", folder_id, owner_id);

    // First Mark the folder as deleted in the database (soft delete)
    mark_folders_and_files_as_deleted(&db_pool, &[folder_id]).await?;

    log::trace!("Marked folder with ID: {} as deleted for owner ID: {}", folder_id, owner_id);
    log::trace!("Starting recursive deletion of contents for folder with ID: {} for owner ID: {}", folder_id, owner_id);

    // Loop through all files and subfolders within the folder and delete them recursively
    delete_folder_contents_recursively(&db_pool, owner_id, folder_id, 0).await?;

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

    log::trace!("Deleting file with ID: {} and its versions: {}; Unique hosted locations: {}", file_id, file_info.len(), file_locations_map.len());

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

        log::trace!("Deleted file versions from host: {}; Total deleted size: {}; Total deleted count: {}", hosted_at, total_deleted_size, total_deleted_count);
    }

    Ok(())
}


pub async fn delete_file_version(db_pool: &PgPool, owner_id: Uuid, file_id: Uuid, version: i32) -> Result<(), AppError> {
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

    log::trace!("Deleting specific file version: {} for file ID: {} from host: {}", version, file_id, specific_version.hosted_at);

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

    log::trace!(
        "Deleted specific file version: {} for file ID: {} from host: {}; Total deleted size: {}; Total deleted count: {}",
        version, file_id, specific_version.hosted_at, total_deleted_size, total_deleted_count
    );

    Ok(())
}
