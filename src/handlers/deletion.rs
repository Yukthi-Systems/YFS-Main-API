use crate::database::files::{get_file_version_details, delete_file_versions};
use super::storage_api::delete_paths_from_server;
use crate::database::user::update_quota;
use deadpool_postgres::Pool as PgPool;
use crate::models::errors::AppError;
use crate::state::API_SETTINGS;
use uuid::Uuid;



pub async fn delete_folder_and_contents(db_pool: &PgPool, owner_id: Uuid, folder_id: Uuid) -> Result<(), AppError> {
    // Given a folder ID - delete the folder and all its contents recursively, update the quota accordingly
    Ok(())
}


pub async fn delete_file_and_versions(db_pool: &PgPool, owner_id: Uuid, file_id: Uuid) -> Result<(), AppError> {
    // Given a file ID - delete the file and all its versions, update the quota accordingly
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
