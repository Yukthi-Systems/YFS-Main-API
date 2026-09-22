use crate::database::folders::{
    list_internal_sharing_in_folders,
    list_internal_sharing_out_folders,
    is_folder_under_parent,
    get_folders_and_files,
    create_new_folder,
    move_folder_under,
    edit_folder_info
};
use crate::database::shares::{
    create_external_share_in_db,
    delete_external_share_in_db,
    update_external_share_in_db,
    list_external_shares_in_db,
    get_internal_share_info,
    create_internal_share,
    delete_internal_share,
    update_internal_share,
};
use crate::models::shares::{CreateExternalShareRequest, InternalShareRequest, UpdateExternalShareRequest};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, patch, post, put, web};
use crate::models::folders::{EditFolderRequest, MoveFolderRequest, NewFolderRequest};
use crate::models::user::{SessionUser, PublicSessionUser};
use crate::handlers::deletion::delete_folder_and_contents;
use crate::handlers::access::authorize_folder_access;
use crate::models::errors::{ApiResponse, AppError};
use crate::handlers::auth::generate_password_hash;
use crate::database::user::get_user_by_id;
use crate::models::PageQuery;
use crate::state::AppState;
use uuid::Uuid;



#[get("/list/sharing-in")]
pub async fn list_sharing_in(request: HttpRequest, query: web::Query<PageQuery>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Shared with the current user by other users
    let shared_folders = list_internal_sharing_in_folders(&state.pg_pool, &session_user.user_id, query.limit, query.offset).await?;

    Ok(HttpResponse::Ok().json(shared_folders))
}


#[get("/list/sharing-out")]
pub async fn list_sharing_out(request: HttpRequest, query: web::Query<PageQuery>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Shared by the current user to other users
    let shared_folders = list_internal_sharing_out_folders(&state.pg_pool, &session_user.user_id, query.limit, query.offset).await?;

    Ok(HttpResponse::Ok().json(shared_folders))
}


#[post("/create")]
pub async fn create_internal_folder_share(request: HttpRequest, body: web::Json<InternalShareRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // See if the user can share the foldes
    if !session_user.is_sharing_enabled {
        return Ok(HttpResponse::Forbidden().body("Sharing is not enabled, please enable sharing in your admin settings"));
    }

    // Check if the user is trying to share the folder with themselves
    if session_user.user_id == body.shared_with_user_id {
        return Ok(HttpResponse::BadRequest().body("You cannot share a folder with yourself"));
    }

    let shared_with_user = get_user_by_id(&state.pg_pool, &body.shared_with_user_id, &session_user.organization_id).await?;
    if shared_with_user.is_none() {
        return Ok(HttpResponse::BadRequest().body("The user you are trying to share with does not exist or is not part of your organization"));
    }

    create_internal_share(
        &state.pg_pool,
        &body.folder_id,
        &body.shared_with_user_id,
        &session_user.user_id,
        body.can_preview,
        body.can_download,
        body.can_create,
        body.can_update,
        body.can_delete,
    ).await?;

    Ok(HttpResponse::Ok().finish())
}


#[delete("/delete/{folder_id}/{shared_with_user_id}")]
pub async fn delete_internal_folder_share(request: HttpRequest, path: web::Path<(Uuid, Uuid)>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let (folder_id, shared_with_user_id) = path.into_inner();

    delete_internal_share(&state.pg_pool, &folder_id, &shared_with_user_id, &session_user.user_id).await?;

    Ok(HttpResponse::Ok().finish())
}


#[patch("/update")]
pub async fn update_internal_folder_share(request: HttpRequest, body: web::Json<InternalShareRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    update_internal_share(
        &state.pg_pool,
        &body.folder_id,
        &body.shared_with_user_id,
        &session_user.user_id,
        body.can_preview,
        body.can_download,
        body.can_create,
        body.can_update,
        body.can_delete,
    ).await?;

    Ok(HttpResponse::Ok().finish())
}


#[get("/info/sharing-out/{folder_id}")]
pub async fn get_internal_share_out_info(request: HttpRequest, folder_id: web::Path<Uuid>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Get all the users who the folder is shared with (sharing-out)
    // Note: sharing-in /info endpoint is not required since its only relevant to the owner of the resource
    let share_info = get_internal_share_info(&state.pg_pool, &folder_id, &session_user.user_id).await?;

    Ok(HttpResponse::Ok().json(share_info))
}


#[get("/list/under/{shared_folder_id}/{request_folder_id}")]
pub async fn list_folders_and_files_under_shared(request: HttpRequest, path: web::Path<(Uuid, Uuid)>, query: web::Query<PageQuery>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let (shared_folder_id, request_folder_id) = path.into_inner();

    // Authorize access to the requested folder under the shared folder first
    let folder_access = authorize_folder_access(
        &state.pg_pool,
        &session_user.user_id,
        &request_folder_id,
        Some(shared_folder_id),
        None,   // No specific access level required
    ).await?;

    let folders_and_files = get_folders_and_files(
        &state.pg_pool,
        &folder_access.owner_user_id,
        &request_folder_id,
        query.limit,
        query.offset,
    ).await?;

    Ok(HttpResponse::Ok().json(folders_and_files))
}


#[post("/create")]
pub async fn create_external_share(request: HttpRequest, body: web::Json<CreateExternalShareRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Either share_file_target_id or share_folder_target_id must be provided
    if body.share_file_target_id.is_none() && body.share_folder_target_id.is_none() {
        return Err(AppError::BadRequest("Either share_file_target_id or share_folder_target_id must be provided".into()));
    }

    // Share ID should be 3 to 36 characters long
    if body.share_id.len() < 3 || body.share_id.len() > 36 {
        return Err(AppError::BadRequest("Share ID must be between 3 and 36 characters long".into()));
    }

    // Generate password hash if password is provided
    let password_hash = if let Some(password) = &body.raw_password {
        Some(generate_password_hash(password))
    } else {
        None
    };

    create_external_share_in_db(
        &state.pg_pool,
        &body.share_id,
        &session_user.user_id,
        body.share_file_target_id,
        body.share_folder_target_id,
        body.can_preview,
        body.can_download,
        body.can_create,
        body.can_update,
        body.can_delete,
        &body.share_info,
        password_hash,
        &body.phones_for_otp,
        &body.emails_for_otp,
        body.expires_at,
    ).await?;

    Ok(HttpResponse::Ok().finish())
}


#[delete("/delete/{share_id}")]
pub async fn delete_external_share(request: HttpRequest, share_id: web::Path<String>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Only the user who created the external share can delete it
    delete_external_share_in_db(&state.pg_pool, &session_user.user_id, &share_id).await?;

    Ok(HttpResponse::Ok().finish())
}


#[patch("/update/{share_id}")]
pub async fn update_external_share(request: HttpRequest, share_id: web::Path<String>, body: web::Json<UpdateExternalShareRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Generate password hash if password is provided
    let password_hash = if let Some(password) = &body.raw_password {
        Some(generate_password_hash(password))
    } else {
        None
    };

    // Only the user who created the external share can update it
    update_external_share_in_db(
        &state.pg_pool,
        &session_user.user_id,
        &share_id,
        body.can_preview,
        body.can_download,
        body.can_create,
        body.can_update,
        body.can_delete,
        &body.share_info,
        body.update_password_hash,
        password_hash,
        &body.phones_for_otp,
        &body.emails_for_otp,
        body.expires_at,
    ).await?;

    Ok(HttpResponse::Ok().finish())
}


#[get("/list")]
pub async fn list_external_shares(request: HttpRequest, query: web::Query<PageQuery>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let external_shares = list_external_shares_in_db(&state.pg_pool, &session_user.user_id, query.limit, query.offset).await?;

    Ok(HttpResponse::Ok().json(external_shares))
}


#[get("/list/under/{request_folder_id}")]
pub async fn list_folders_and_files_under_public(request: HttpRequest, request_folder_id: web::Path<Uuid>, query: web::Query<PageQuery>, state: web::Data<AppState>) -> ApiResponse {
    // Get PublicSessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<PublicSessionUser>().unwrap();

    // This will work only if the public session is of type shared folder access
    if !session_user.share_folder_target_id.is_some() {
        return Err(AppError::BadRequest("Public session is not of type shared folder access".into()));
    }

    // Check if the shared folder is under the specified shared folder
    if !is_folder_under_parent(&state.pg_pool, &request_folder_id, &session_user.share_folder_target_id.unwrap()).await? {
        return Err(AppError::BadRequest("The requested folder is not under the specified shared folder".into()));
    }

    let folders_and_files = get_folders_and_files(&state.pg_pool, &session_user.created_by, &request_folder_id, query.limit, query.offset).await?;

    Ok(HttpResponse::Ok().json(folders_and_files))
}


#[post("/create")]
pub async fn create_public_folder(request: HttpRequest, body: web::Json<NewFolderRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get PublicSessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<PublicSessionUser>().unwrap();

    // Check if the public session can create folders
    if !session_user.permission_set.can_create {
        return Err(AppError::BadRequest("Public session does not have permission to create folders".into()));
    }

    // This will work only if the public session is of type shared folder access
    if !session_user.share_folder_target_id.is_some() {
        return Err(AppError::BadRequest("Public session is not of type shared folder access".into()));
    }

    // There should be a parent folder specified in the request body
    if body.parent_folder_id.is_none() {
        return Err(AppError::BadRequest("Parent folder must be specified".into()));
    }

    // Check if the shared folder is under the specified shared folder
    if !is_folder_under_parent(&state.pg_pool, &body.parent_folder_id.unwrap(), &session_user.share_folder_target_id.unwrap()).await? {
        return Err(AppError::BadRequest("The parent folder is not under the specified shared folder".into()));
    }

    create_new_folder(&state.pg_pool, &session_user.created_by, body.parent_folder_id, &body.folder_name, &body.folder_info).await?;

    Ok(HttpResponse::Ok().finish())
}


#[put("/move")]
pub async fn move_public_folder(request: HttpRequest, body: web::Json<MoveFolderRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get PublicSessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<PublicSessionUser>().unwrap();

    // Check if the public session can create, edit
    if !session_user.permission_set.can_create || !session_user.permission_set.can_update {
        return Err(AppError::BadRequest("Public session does not have permission to move folders".into()));
    }

    // This will work only if the public session is of type shared folder access
    if !session_user.share_folder_target_id.is_some() {
        return Err(AppError::BadRequest("Public session is not of type shared folder access".into()));
    }

    // There should be a parent folder specified in the request body
    if body.new_parent_folder_id.is_none() {
        return Err(AppError::BadRequest("Parent folder must be specified".into()));
    }

    // Check if the current folder also under the specified shared folder
    if !is_folder_under_parent(&state.pg_pool, &body.folder_id, &session_user.share_folder_target_id.unwrap()).await? {
        return Err(AppError::BadRequest("The current folder is not under the specified shared folder".into()));
    }

    // Check if the shared folder is under the specified shared folder
    if !is_folder_under_parent(&state.pg_pool, &body.new_parent_folder_id.unwrap(), &session_user.share_folder_target_id.unwrap()).await? {
        return Err(AppError::BadRequest("The parent folder is not under the specified shared folder".into()));
    }
    
    move_folder_under(&state.pg_pool, &session_user.created_by, &body.folder_id, body.new_parent_folder_id).await?;

    Ok(HttpResponse::Ok().finish())
}


#[patch("/edit")]
pub async fn edit_public_folder(request: HttpRequest, body: web::Json<EditFolderRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get PublicSessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<PublicSessionUser>().unwrap();

    // Check if the public session can update folders
    if !session_user.permission_set.can_update {
        return Err(AppError::BadRequest("Public session does not have permission to edit folders".into()));
    }

    // This will work only if the public session is of type shared folder access
    if !session_user.share_folder_target_id.is_some() {
        return Err(AppError::BadRequest("Public session is not of type shared folder access".into()));
    }

    // Check if the shared folder is under the specified shared folder
    if !is_folder_under_parent(&state.pg_pool, &body.folder_id, &session_user.share_folder_target_id.unwrap()).await? {
        return Err(AppError::BadRequest("The parent folder is not under the specified shared folder".into()));
    }
    
    edit_folder_info(&state.pg_pool, &session_user.created_by, &body.folder_id, &body.folder_name, &body.folder_info).await?;

    Ok(HttpResponse::Ok().finish())
}


#[patch("/delete/{folder_id}")]
pub async fn delete_public_folder(request: HttpRequest, path: web::Path<Uuid>, state: web::Data<AppState>) -> ApiResponse {
    // Get PublicSessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<PublicSessionUser>().unwrap();
    let folder_id = path.into_inner();

    // Check if the public session can update folders
    if !session_user.permission_set.can_delete {
        return Err(AppError::BadRequest("Public session does not have permission to delete folders".into()));
    }

    // This will work only if the public session is of type shared folder access
    if !session_user.share_folder_target_id.is_some() {
        return Err(AppError::BadRequest("Public session is not of type shared folder access".into()));
    }

    // Check if the shared folder is under the specified shared folder
    if !is_folder_under_parent(&state.pg_pool, &folder_id, &session_user.share_folder_target_id.unwrap()).await? {
        return Err(AppError::BadRequest("The parent folder is not under the specified shared folder".into()));
    }

    // Can not delete the root shared folder
    if folder_id == session_user.share_folder_target_id.unwrap() {
        return Err(AppError::BadRequest("Cannot delete the root shared folder".into()));
    }

    // Spawn a background task to delete the folder and its contents asynchronously
    tokio::spawn(delete_folder_and_contents(state.pg_pool.clone(), session_user.created_by, folder_id));

    Ok(HttpResponse::Ok().finish())
}
