use crate::database::files::{add_or_update_file_version, create_base_file_entry, delete_file, get_file_info_by_id, get_file_location, lock_base_file_entry, move_file_to_folder, update_base_file_info};
use crate::database::user::update_quota;
use crate::handlers::deletion::{delete_file_and_versions, delete_file_version};
use crate::handlers::storage_api::{generate_upload_sessions, build_file_location, generate_download_sessions, generate_wopi_session};
use crate::handlers::access::{authorize_file_access, authorize_folder_access, SharedPermission};
use crate::models::files::{FileOpsCallBack, FileOpsRequest, FileOpsType, FileLocation};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, patch, post, put, web};
use crate::models::errors::{ApiResponse, AppError};
use crate::state::{AppState, API_SETTINGS};
use crate::models::user::SessionUser;
use uuid::Uuid;


const BASE_FOLDER_PATH: &str = "/data/yfs";


#[post("/operation/{operation_type}")]
pub async fn single_file_operation(request: HttpRequest, operation_type: web::Path<FileOpsType>, file_request: web::Json<FileOpsRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();


    // TODO: Check the organization-level constraints for file operations (Taken from SSO API)
    // Example: Encryption at rest, File size limits, Allowed file types, etc.
    // Also SSO should only give the available servers list, so that Org level changes do not affect ongoing operations

    // TODO: Check if the folder is accessible by the user
    // TODO: Check if its a shared folder, then try to do the same

    let operation_type = operation_type.into_inner();

    // Validate the file operation request
    file_request.validate(&operation_type, session_user.is_file_versioning_enabled)?;

    let file_id = file_request.file_id.unwrap_or(Uuid::new_v4());

    // If file_id is there
    if file_request.file_id.is_some() {
        // Get the file information from the database
        let file_info = get_file_info_by_id(&state.pg_pool, &file_request.folder_id, &file_id).await?;
        if file_info.is_none() {
            return Err(AppError::BadRequest("File not found".into()));
        }
        let file_info = file_info.unwrap();

        // Validate the file operation against the current file information
        file_request.validate_against_info(&operation_type, &file_info)?;

        // Now do match for each type
        match operation_type {
            FileOpsType::Delete => {
                // TODO: Handle delete operation

                let file_location = get_file_location(&state.pg_pool, &file_request.folder_id, &file_id, file_request.file_version).await?;
                if file_location.is_none() {
                    return Err(AppError::BadRequest("File location not found".into()));
                }
                let file_location = file_location.unwrap();
                let file_storage_api_base_url = file_location.hosted_at;

                // TODO: Delete the file using DB call (just that version)

                return Err(AppError::NotImplemented("Delete operation is not implemented yet".into()));
            },
            FileOpsType::Replace => {
                // TODO: Handle update operation
                // Mark the file as locked

                let file_location = get_file_location(&state.pg_pool, &file_request.folder_id, &file_id, file_request.file_version).await?;
                if file_location.is_none() {
                    return Err(AppError::BadRequest("File location not found".into()));
                }
                let file_location = file_location.unwrap();
                let file_storage_api_base_url = file_location.hosted_at;

                // Quota should be = new file size - existing file size

                // return Err(AppError::NotImplemented("Replace operation is not implemented yet".into()));
            },
            _ => {}
        }
    }

    Ok(HttpResponse::Ok().finish())
}


#[post("/upload")]
pub async fn request_file_upload(request: HttpRequest, file_request: web::Json<FileOpsRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // TODO: Check the organization-level constraints for file operations (Taken from SSO API)
    // Example: Encryption at rest, File size limits, Allowed file types, etc.
    // Also SSO should only give the available servers list, so that Org level changes do not affect ongoing operations

    // TODO: Check quota for upload, see if the user and the server has enough storage or not

    let operation_type = FileOpsType::Upload;

    // Validate the file operation request
    file_request.validate(&operation_type, session_user.is_file_versioning_enabled)?;

    let file_id = file_request.file_id.unwrap_or(Uuid::new_v4());
    let file_owner_id: Uuid;

    // If file_id is there
    if file_request.file_id.is_some() {
        let file_access = authorize_file_access(
            &state.pg_pool,
            &session_user.user_id,
            &file_request.folder_id,
            &file_id,
            file_request.shared_folder_id,
            file_request.shared_folder_id.map(|_| SharedPermission::Create),
        ).await?;

        // Validate the file operation against the current file information
        file_request.validate_against_info(&operation_type, &file_access.file_info)?;

        // Update the file owner ID based on the existing file information
        file_owner_id = file_access.owner_user_id;
        
        // This is not the first version upload operation
        // Just mark the file as locked
        lock_base_file_entry(
            &state.pg_pool,
            &file_request.folder_id,
            &file_owner_id,
            &file_id,
            true
        ).await?;
    } else {
        let folder_access = authorize_folder_access(
            &state.pg_pool,
            &session_user.user_id,
            &file_request.folder_id,
            file_request.shared_folder_id,
            file_request.shared_folder_id.map(|_| SharedPermission::Create),
        ).await?;
        file_owner_id = folder_access.owner_user_id;

        // Create the file and lock it for the first version upload operation
        create_base_file_entry(
            &state.pg_pool,
            &file_id,
            &file_request.folder_id,
            &file_owner_id,
            &file_request.file_name,
            &file_request.file_info,
            true
        ).await?;
    }

    // Build the file location URL for the storage server
    let file_location = build_file_location(
        BASE_FOLDER_PATH,
        &session_user.organization_id,
        &file_owner_id,
        &file_request.folder_id,
        &file_id,
        file_request.file_version
    );

    let file_location_struct = FileLocation {
        file_location: file_location.clone(),
        hosted_at: API_SETTINGS.file_store_host.clone(),
    };

    let file_storage_api = file_request.generate_api_struct(file_location_struct, file_owner_id, file_id);

    // Generate an upload session for the file with the storage server
    let upload_session_response = generate_upload_sessions(
        &API_SETTINGS.file_store_host,
        &API_SETTINGS.file_store_api_key,
        &serde_json::json!([file_storage_api]),
    ).await?;

    Ok(HttpResponse::Ok().json(upload_session_response))
}


#[post("/upload/{is_success}")]
pub async fn callback_file_upload(path: web::Path<bool>, file_request: web::Json<FileOpsCallBack>, state: web::Data<AppState>) -> ApiResponse {
    // Handle the file upload callback from the storage server
    let is_success = path.into_inner();
    let file_request = file_request.into_inner();

    // Remove the file lock either on success or failure of the upload
    lock_base_file_entry(
        &state.pg_pool,
        &file_request.folder_id,
        &file_request.owner_id,
        &file_request.file_id,
        false
    ).await?;

    // TODO: Implement the logic to process the file upload callback
    if is_success {
        // Add the file version record or replace
        add_or_update_file_version(
            &state.pg_pool,
            &file_request.file_id,
            file_request.file_version,
            &file_request.file_location,
            &file_request.owner_id,
            &file_request.hosted_at,
            file_request.file_size,
            &file_request.metadata,
            &file_request.file_hash,
        ).await?;

        // TODO: Handle if its a replacement of an existing file version
        // - If it is replacing an existing file version, we might need to adjust the quota accordingly

        // Update the quota after adding the file version
        update_quota(
            &state.pg_pool,
            &file_request.owner_id,
            &file_request.hosted_at,
            file_request.file_size,
            1
        ).await?;

    } else {
        // If the upload failed and it's the first version, delete the file entry
        if file_request.file_version == 1 {
            delete_file(
                &state.pg_pool,
                &file_request.folder_id,
                &file_request.owner_id,
                &file_request.file_id,
            ).await?;
        }
    }

    Ok(HttpResponse::Ok().finish())
}


#[post("/download")]
pub async fn request_file_download(request: HttpRequest, file_request: web::Json<FileOpsRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let operation_type = FileOpsType::Download;

    // Validate the file operation request
    file_request.validate(&operation_type, session_user.is_file_versioning_enabled)?;

    // File ID should be provided for download operations
    let file_id = file_request.file_id.unwrap();    // Checks are already done in the validation step

    // Check file access permissions for the requested file
    let file_access = authorize_file_access(
        &state.pg_pool,
        &session_user.user_id,
        &file_request.folder_id,
        &file_id,
        file_request.shared_folder_id,
        file_request.shared_folder_id.map(|_| SharedPermission::Download),
    ).await?;

    // Validate the file operation against the current file information
    file_request.validate_against_info(&operation_type, &file_access.file_info)?;

    let file_location = get_file_location(&state.pg_pool, &file_request.folder_id, &file_id, file_request.file_version).await?;
    if file_location.is_none() {
        return Err(AppError::Gone("File location not found".into()));
    }
    let file_location = file_location.unwrap();
    let hosted_at = file_location.hosted_at.clone();

    let file_storage_api = file_request.generate_api_struct(file_location, file_access.owner_user_id, file_id);

    // Generate a download session for the file with the storage server
    let download_session_response = generate_download_sessions(
        &hosted_at,
        &API_SETTINGS.file_store_api_key,
        &serde_json::json!([file_storage_api]),
    ).await?;

    Ok(HttpResponse::Ok().json(download_session_response))
}


#[patch("/update")]
pub async fn update_file_info(request: HttpRequest, file_request: web::Json<FileOpsRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let operation_type = FileOpsType::Download;

    // Validate the file operation request
    file_request.validate(&operation_type, session_user.is_file_versioning_enabled)?;

    // Checks are already done in the validation step so we can safely unwrap the file_id
    let file_id = file_request.file_id.unwrap();

    // Check file access permissions for the requested file
    let file_access = authorize_file_access(
        &state.pg_pool,
        &session_user.user_id,
        &file_request.folder_id,
        &file_id,
        file_request.shared_folder_id,
        file_request.shared_folder_id.map(|_| SharedPermission::Update),
    ).await?;

    // Validate the file operation against the current file information
    file_request.validate_against_info(&operation_type, &file_access.file_info)?;

    // This will only work, if its not locked (if its locked, the update will not be applied)

    // Update only File Name and Replace the file info
    update_base_file_info(
        &state.pg_pool,
        &file_id,
        &file_request.folder_id,
        &file_access.owner_user_id,
        &file_request.file_name,
        &file_request.file_info,
    ).await?;

    Ok(HttpResponse::Ok().finish())
}


#[put("/move/{destination_folder_id}")]
pub async fn move_file(request: HttpRequest, destination_folder_id: web::Path<Uuid>, file_request: web::Json<FileOpsRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let operation_type = FileOpsType::Download;

    // Validate the file operation request
    file_request.validate(&operation_type, session_user.is_file_versioning_enabled)?;

    // Checks are already done in the validation step so we can safely unwrap the file_id
    let file_id = file_request.file_id.unwrap();

    // Check if the user has access to the destination folder
    let folder_access = authorize_folder_access(
        &state.pg_pool,
        &session_user.user_id,
        &destination_folder_id,
        file_request.shared_folder_id,
        file_request.shared_folder_id.map(|_| SharedPermission::Move),
    ).await?;

    // Check file access permissions for the requested file
    let file_access = authorize_file_access(
        &state.pg_pool,
        &session_user.user_id,
        &file_request.folder_id,
        &file_id,
        file_request.shared_folder_id,
        file_request.shared_folder_id.map(|_| SharedPermission::Move),
    ).await?;

    // Validate the file operation against the current file information
    file_request.validate_against_info(&operation_type, &file_access.file_info)?;

    // This will only work, if its not locked (if its locked, the update will not be applied)

    // Move the file by updating its folder_id
    move_file_to_folder(
        &state.pg_pool,
        &file_id,
        &file_request.folder_id,
        &destination_folder_id,
        &folder_access.owner_user_id,
    ).await?;

    Ok(HttpResponse::Ok().finish())
}


#[post("/get-info")]
pub async fn get_file_basic_info(request: HttpRequest, file_request: web::Json<FileOpsRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let operation_type = FileOpsType::Download;

    // Validate the file operation request
    file_request.validate(&operation_type, session_user.is_file_versioning_enabled)?;

    // File ID should be provided for download operations
    let file_id = file_request.file_id.unwrap();    // Checks are already done in the validation step

    // Check file access permissions for the requested file
    let file_access = authorize_file_access(
        &state.pg_pool,
        &session_user.user_id,
        &file_request.folder_id,
        &file_id,
        file_request.shared_folder_id,
        file_request.shared_folder_id.map(|_| SharedPermission::Download),
    ).await?;

    // Validate the file operation against the current file information
    file_request.validate_against_info(&operation_type, &file_access.file_info)?;

    Ok(HttpResponse::Ok().json(file_access.file_info))
}


#[post("/wopi/session/create/{to_write}")]
pub async fn create_wopi_session(request: HttpRequest, to_write: web::Path<bool>, file_request: web::Json<FileOpsRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // TODO: Check the organization-level constraints for file operations (Taken from SSO API)
    // Example: Encryption at rest, File size limits, Allowed file types, etc.
    // Also SSO should only give the available servers list, so that Org level changes do not affect ongoing operations

    // TODO: Check quota for upload, see if the user and the server has enough storage or not

    let to_write = to_write.into_inner();
    let operation_type_read = FileOpsType::Download;
    let operation_type_write = if to_write { FileOpsType::Upload } else { FileOpsType::Download };

    // Validate the file operation request for both read and write operations
    // Since a WOPI session will involve both downloading and uploading the file
    file_request.validate(&operation_type_read, session_user.is_file_versioning_enabled)?;
    file_request.validate(&operation_type_write, session_user.is_file_versioning_enabled)?;

    // File ID should be provided for download operations
    let file_id = file_request.file_id.unwrap();    // Checks are already done in the validation step

    // Check file access permissions for the requested file
    let file_access = authorize_file_access(
        &state.pg_pool,
        &session_user.user_id,
        &file_request.folder_id,
        &file_id,
        file_request.shared_folder_id,
        file_request.shared_folder_id.map(|_| if to_write { SharedPermission::Update } else { SharedPermission::Download }),
    ).await?;

    // Validate the file operation against the current file information
    file_request.validate_against_info(&operation_type_read, &file_access.file_info)?;

    let file_location = get_file_location(&state.pg_pool, &file_request.folder_id, &file_id, file_request.file_version).await?;
    if file_location.is_none() {
        return Err(AppError::Gone("File location not found".into()));
    }
    let file_location = file_location.unwrap();

    // Generate an upload session for the file with the storage server
    let upload_session_response = generate_wopi_session(
        &file_location.hosted_at,
        &API_SETTINGS.file_store_api_key,
        &serde_json::json!({
            "file_name": file_request.file_name,
            "file_location": file_location.file_location,
            "server_host": file_location.hosted_at,
            "file_id": file_id,
            "owner_id": file_access.owner_user_id,
            "is_file_versioning_enabled": session_user.is_file_versioning_enabled,
            "latest_file_version": file_access.file_info.available_versions.iter().max().cloned().unwrap_or(1),
            "user_id": session_user.user_id,
            "user_name": &session_user.email,
            "can_write": true,
        }),
    ).await?;

    Ok(HttpResponse::Ok().json(upload_session_response))
}


#[delete("/delete/version")]
pub async fn delete_any_file_version(request: HttpRequest, file_request: web::Json<FileOpsRequest>, state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // File version should be greater than 1
    if file_request.file_version <= 1 {
        return Err(AppError::BadRequest("File version must be greater than 1 for deletion".into()));
    }

    let operation_type = FileOpsType::Delete;

    // Validate the file operation request
    file_request.validate(&operation_type, session_user.is_file_versioning_enabled)?;

    // File ID should be provided for delete operations
    let file_id = file_request.file_id.unwrap();    // Checks are already done in the validation step

    // Check file access permissions for the requested file
    let file_access = authorize_file_access(
        &state.pg_pool,
        &session_user.user_id,
        &file_request.folder_id,
        &file_id,
        file_request.shared_folder_id,
        file_request.shared_folder_id.map(|_| SharedPermission::Delete),
    ).await?;

    // Validate the file operation against the current file information
    file_request.validate_against_info(&operation_type, &file_access.file_info)?;

    // Delete the specified file version
    delete_file_version(
        state.pg_pool.clone(),
        file_access.owner_user_id,
        file_id,
        file_request.file_version
    ).await?;

    Ok(HttpResponse::NoContent().finish())
}


#[delete("/delete/file")]
pub async fn delete_full_file(request: HttpRequest, file_request: web::Json<FileOpsRequest>, state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let operation_type = FileOpsType::Delete;

    // Validate the file operation request
    file_request.validate(&operation_type, session_user.is_file_versioning_enabled)?;

    // File ID should be provided for delete operations
    let file_id = file_request.file_id.unwrap();    // Checks are already done in the validation step

    // Check file access permissions for the requested file
    let file_access = authorize_file_access(
        &state.pg_pool,
        &session_user.user_id,
        &file_request.folder_id,
        &file_id,
        file_request.shared_folder_id,
        file_request.shared_folder_id.map(|_| SharedPermission::Delete),
    ).await?;

    // Validate the file operation against the current file information
    file_request.validate_against_info(&operation_type, &file_access.file_info)?;

    // Delete the full file and all its versions
    delete_file_and_versions(
        &state.pg_pool,
        file_access.owner_user_id,
        file_id
    ).await?;

    Ok(HttpResponse::NoContent().finish())
}
