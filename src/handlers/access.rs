use crate::database::folders::{is_folder_belongs_to_user, is_folder_under_parent};
use crate::database::shares::internal_shared_permissions;
use crate::database::files::get_file_info_by_id;
use crate::models::files::BasicFileInfo;
use deadpool_postgres::Pool as PgPool;
use crate::models::errors::AppError;
use uuid::Uuid;


pub enum SharedPermission {
    Create,
    Update,
    Move,
    Delete,
    Download,
}


pub struct FolderAccess {
    pub owner_user_id: Uuid,
}


pub struct FileAccess {
    pub owner_user_id: Uuid,
    pub file_info: BasicFileInfo,
}


pub async fn authorize_folder_access(
    db_pool: &PgPool,
    actor_user_id: &Uuid,
    target_folder_id: &Uuid,
    shared_folder_id: Option<Uuid>,
    required_shared_permission: Option<SharedPermission>,
) -> Result<FolderAccess, AppError> {
    // Determine the owner of the folder based on whether it is accessed via a shared folder or directly by the actor
    let owner_user_id = if let Some(shared_folder_id) = shared_folder_id {
        // Check if the target folder is under the specified shared folder
        if !is_folder_under_parent(db_pool, target_folder_id, &shared_folder_id).await? {
            return Err(AppError::BadRequest("The requested folder is not under the specified shared folder".into()));
        }

        // Retrieve the shared permissions for the actor in the context of the shared folder
        let shared_permissions = internal_shared_permissions(db_pool, &shared_folder_id, actor_user_id).await?;
        if shared_permissions.is_none() {
            return Err(AppError::BadRequest("You do not have access to this shared folder".into()));
        }
        let shared_permissions = shared_permissions.unwrap();

        // Check if the actor has the required shared permission in the context of the shared folder
        if let Some(required_permission) = required_shared_permission {
            let is_allowed = match required_permission {
                SharedPermission::Create => shared_permissions.can_create,
                SharedPermission::Update => shared_permissions.can_update,
                SharedPermission::Move => shared_permissions.can_create || shared_permissions.can_update,
                SharedPermission::Delete => shared_permissions.can_delete,
                SharedPermission::Download => shared_permissions.can_download || shared_permissions.can_preview,
            };

            if !is_allowed {
                return Err(AppError::BadRequest("You do not have permission to perform this action in the shared folder".into()));
            }
        }

        // shared_with_user_id is actually the owner of the shared folder, not the actor accessing it (SQL query hack to not use a separate structure)
        shared_permissions.shared_with_user_id
    } else {
        // If the folder is accessed directly by the actor, the actor is considered the owner
        *actor_user_id
    };

    // Verify that the folder actually belongs to the determined owner
    if !is_folder_belongs_to_user(db_pool, target_folder_id, &owner_user_id).await? {
        return Err(AppError::BadRequest("The requested folder does not exist or is not accessible for this user".into()));
    }

    Ok(FolderAccess { owner_user_id })
}


pub async fn authorize_file_access(
    db_pool: &PgPool,
    actor_user_id: &Uuid,
    folder_id: &Uuid,
    file_id: &Uuid,
    shared_folder_id: Option<Uuid>,
    required_shared_permission: Option<SharedPermission>,
) -> Result<FileAccess, AppError> {
    // Authorize access to the folder containing the file first
    let folder_access = authorize_folder_access(
        db_pool,
        actor_user_id,
        folder_id,
        shared_folder_id,
        required_shared_permission,
    ).await?;

    // Retrieve the file information from the database
    let file_info = get_file_info_by_id(db_pool, folder_id, file_id).await?;
    if file_info.is_none() {
        return Err(AppError::BadRequest("File not found".into()));
    }
    let file_info = file_info.unwrap();

    // Verify that the file belongs to the owner of the folder
    if file_info.user_id != folder_access.owner_user_id {
        return Err(AppError::BadRequest("The requested file is not accessible for this user".into()));
    }

    Ok(FileAccess {
        owner_user_id: folder_access.owner_user_id,
        file_info,
    })
}
