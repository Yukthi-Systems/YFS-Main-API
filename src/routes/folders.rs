use crate::database::folders::{create_new_folder, edit_folder_info, get_folders_and_files, get_root_folders, move_folder_under, get_folder_total_size};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, patch, post, put, web};
use crate::models::folders::{EditFolderRequest, MoveFolderRequest, NewFolderRequest};
use crate::handlers::access::{authorize_folder_access, SharedPermission};
use crate::handlers::storage_api::generate_folder_download_session;
use crate::handlers::deletion::delete_folder_and_contents;
use crate::models::errors::{ApiResponse, AppError};
use crate::state::{AppState, API_SETTINGS};
use crate::models::user::SessionUser;
use crate::models::PageQuery;
use uuid::Uuid;



#[get("/list/root")]
pub async fn list_root_folders(request: HttpRequest, query: web::Query<PageQuery>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let folders_and_files = get_root_folders(&state.pg_pool, &session_user.user_id, query.limit, query.offset).await?;

    Ok(HttpResponse::Ok().json(folders_and_files))
}


#[get("/list/under/{parent_folder_id}")]
pub async fn list_folders_and_files_under(request: HttpRequest, parent_folder_id: web::Path<Uuid>, query: web::Query<PageQuery>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let folders_and_files = get_folders_and_files(&state.pg_pool, &session_user.user_id, &parent_folder_id, query.limit, query.offset).await?;

    Ok(HttpResponse::Ok().json(folders_and_files))
}


#[post("/create")]
pub async fn create_folder(request: HttpRequest, new_folder: web::Json<NewFolderRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // If its an shared folder, then both the shared folder ID and the parent folder ID need to be present
    if new_folder.shared_folder_id.is_some() && new_folder.parent_folder_id.is_none() {
        return Err(AppError::BadRequest("Parent folder ID must be specified for a shared folder".into()));
    }

    // Check if the folder is under a shared folder and authorize access accordingly
    if let Some(shared_folder_id) = new_folder.shared_folder_id {
        let request_folder_id = new_folder.parent_folder_id.unwrap();
        let folder_access = authorize_folder_access(
            &state.pg_pool,
            &session_user.user_id,
            &request_folder_id,
            Some(shared_folder_id),
            Some(SharedPermission::Create),
        ).await?;

        // Create the folder under the user who own's the shared folder
        create_new_folder(&state.pg_pool, &folder_access.owner_user_id, new_folder.parent_folder_id, &new_folder.folder_name, &new_folder.folder_info).await?;

        return Ok(HttpResponse::Ok().finish());
    }

    // Check if the parent folder exists and belongs to the current session user before creating a new folder
    if let Some(parent_folder_id) = new_folder.parent_folder_id {
        authorize_folder_access(
            &state.pg_pool,
            &session_user.user_id,
            &parent_folder_id,
            None,
            None,
        ).await?;
    }

    // If it's not a shared folder, create it under the current session user
    create_new_folder(&state.pg_pool, &session_user.user_id, new_folder.parent_folder_id, &new_folder.folder_name, &new_folder.folder_info).await?;

    Ok(HttpResponse::Ok().finish())
}


#[patch("/edit")]
pub async fn edit_folder_details(request: HttpRequest, edit_folder: web::Json<EditFolderRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Authorize access to the folder being edited, considering shared folder permissions if applicable
    let folder_access = authorize_folder_access(
        &state.pg_pool,
        &session_user.user_id,
        &edit_folder.folder_id,
        edit_folder.shared_folder_id,
        edit_folder.shared_folder_id.map(|_| SharedPermission::Update),
    ).await?;

    edit_folder_info(
        &state.pg_pool,
        &folder_access.owner_user_id,
        &edit_folder.folder_id,
        &edit_folder.folder_name,
        &edit_folder.folder_info,
    ).await?;

    Ok(HttpResponse::Ok().finish())
}


#[put("/move")]
pub async fn move_folder(request: HttpRequest, move_request: web::Json<MoveFolderRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    if move_request.shared_folder_id.is_some() && move_request.new_parent_folder_id.is_none() {
        return Err(AppError::BadRequest("New parent folder ID must be specified for shared folder move".into()));
    }

    // Authorize access to the source folder, considering shared folder permissions if applicable
    let source_access = authorize_folder_access(
        &state.pg_pool,
        &session_user.user_id,
        &move_request.folder_id,
        move_request.shared_folder_id,
        move_request.shared_folder_id.map(|_| SharedPermission::Move),
    ).await?;

    if let Some(new_parent_folder_id) = move_request.new_parent_folder_id {
        authorize_folder_access(
            &state.pg_pool,
            &session_user.user_id,
            &new_parent_folder_id,
            move_request.shared_folder_id,
            move_request.shared_folder_id.map(|_| SharedPermission::Move),
        ).await?;
    }

    move_folder_under(
        &state.pg_pool,
        &source_access.owner_user_id,
        &move_request.folder_id,
        move_request.new_parent_folder_id,
    ).await?;

    Ok(HttpResponse::Ok().finish())
}


#[delete("/delete")]
pub async fn delete_folder(request: HttpRequest, delete_request: web::Json<EditFolderRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Authorize access to the folder being edited, considering shared folder permissions if applicable
    let folder_access = authorize_folder_access(
        &state.pg_pool,
        &session_user.user_id,
        &delete_request.folder_id,
        delete_request.shared_folder_id,
        delete_request.shared_folder_id.map(|_| SharedPermission::Delete),
    ).await?;

    edit_folder_info(
        &state.pg_pool,
        &folder_access.owner_user_id,
        &delete_request.folder_id,
        &delete_request.folder_name,
        &delete_request.folder_info,
    ).await?;

    // Spawn a background task to delete the folder and its contents asynchronously
    tokio::spawn(delete_folder_and_contents(
        state.pg_pool.clone(),
        folder_access.owner_user_id,
        delete_request.folder_id
    ));

    Ok(HttpResponse::Accepted().finish())
}


#[post("/download/{export_type}")]
pub async fn download_folder(request: HttpRequest, export_type: web::Path<String>, edit_folder: web::Json<EditFolderRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Authorize access to the folder being edited, considering shared folder permissions if applicable
    authorize_folder_access(
        &state.pg_pool,
        &session_user.user_id,
        &edit_folder.folder_id,
        edit_folder.shared_folder_id,
        edit_folder.shared_folder_id.map(|_| SharedPermission::Download),
    ).await?;

    // Get total size of the folder contents before generating the download session
    let total_size = get_folder_total_size(&state.pg_pool, &edit_folder.folder_id).await?;
    if total_size == 0 {
        return Err(AppError::Unprocessable("There are no files to download in the folder".into()));
    }
    if total_size >= API_SETTINGS.max_instant_download_size {
        return Err(AppError::Unprocessable("The folder is too large to download".into()));
    }

    // Generate an folder download session for the requested folder
    let download_session_response = generate_folder_download_session(
        &API_SETTINGS.download_manager_api_url,
        &API_SETTINGS.download_manager_api_key,
        &serde_json::json!({
            "root_folder_id": edit_folder.folder_id,
            "archive_name": edit_folder.folder_name,
            "export_type": export_type.into_inner()
        }),
    ).await?;

    Ok(HttpResponse::Ok().json(download_session_response))
}
