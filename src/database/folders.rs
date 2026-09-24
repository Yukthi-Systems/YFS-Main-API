use crate::models::shares::InternalSharedResource;
use deadpool_postgres::Pool as PgPool;
use crate::models::folders::Resource;
use crate::models::errors::AppError;
use uuid::Uuid;



pub async fn get_folders_and_files(db_pool: &PgPool, user_id: &Uuid, parent_folder_id: &Uuid, limit: i64, offset: i64) -> Result<Vec<Resource>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT
                true AS is_resource_folder,
                f.parent_folder_id,
                f.folder_id AS resource_id,
                f.folder_name AS resource_name,
                f.folder_info AS resource_info,

                COALESCE(
                    (
                        SELECT SUM(fv.file_size)
                        FROM files fi
                        INNER JOIN file_versions fv
                            ON fv.file_id = fi.file_id
                        WHERE fi.folder_id = f.folder_id
                          AND fi.user_id = $1
                          AND fi.deleted_at IS NULL
                    ),
                    0
                )::BIGINT AS total_resource_size,

                f.created_at,
                f.updated_at,
                f.deleted_at

            FROM folders f
            WHERE f.user_id = $1
              AND f.parent_folder_id = $2
              AND f.deleted_at IS NULL

            UNION ALL

            SELECT
                false AS is_resource_folder,
                fi.folder_id AS parent_folder_id,
                fi.file_id AS resource_id,
                fi.file_name AS resource_name,
                fi.file_info AS resource_info,

                COALESCE(
                    (
                        SELECT SUM(fv.file_size)
                        FROM file_versions fv
                        WHERE fv.file_id = fi.file_id
                    ),
                    0
                )::BIGINT AS total_resource_size,

                fi.created_at,
                fi.updated_at,
                fi.deleted_at

            FROM files fi
            WHERE fi.user_id = $1
              AND fi.folder_id = $2
              AND fi.deleted_at IS NULL

            ORDER BY updated_at DESC
            LIMIT $3
            OFFSET $4
            "#,
            &[
                user_id,
                parent_folder_id,
                &limit,
                &offset,
            ],
        )
        .await?;

    Ok(Resource::from_rows(rows))
}


pub async fn get_root_folders(db_pool: &PgPool, user_id: &Uuid, limit: i64, offset: i64) -> Result<Vec<Resource>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT
                true AS is_resource_folder,
                f.parent_folder_id AS parent_folder_id,
                f.folder_id AS resource_id,
                f.folder_name AS resource_name,
                f.folder_info AS resource_info,

                COALESCE(
                    (
                        SELECT SUM(fv.file_size)
                        FROM files fi
                        INNER JOIN file_versions fv
                            ON fv.file_id = fi.file_id
                        WHERE fi.folder_id = f.folder_id
                          AND fi.user_id = $1
                          AND fi.deleted_at IS NULL
                    ),
                    0
                )::BIGINT AS total_resource_size,

                f.created_at,
                f.updated_at,
                f.deleted_at

            FROM folders f
            WHERE f.user_id = $1
              AND f.parent_folder_id IS NULL
              AND f.deleted_at IS NULL
            ORDER BY updated_at DESC
            LIMIT $2
            OFFSET $3
            "#,
        &[&user_id, &limit, &offset],
        )
        .await?;

    Ok(Resource::from_rows(rows))
}


pub async fn create_new_folder(db_pool: &PgPool, user_id: &Uuid, parent_folder_id: Option<Uuid>, folder_name: &str, folder_info: &serde_json::Value) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            INSERT INTO folders (user_id, parent_folder_id, folder_id, folder_name, folder_info)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        &[
            &user_id,
            &parent_folder_id,
            &Uuid::new_v4(),
            &folder_name,
            &folder_info,
        ],
    )
    .await?;

    Ok(())
}


pub async fn edit_folder_info(db_pool: &PgPool, user_id: &Uuid, folder_id: &Uuid, folder_name: &str, folder_info: &serde_json::Value) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            UPDATE folders
            SET folder_name = $1,
                folder_info = $2,
                updated_at = CURRENT_TIMESTAMP
            WHERE folder_id = $3
              AND user_id = $4
              AND deleted_at IS NULL
            "#,
        &[
            &folder_name,
            &folder_info,
            &folder_id,
            &user_id,
        ],
    )
    .await?;

    Ok(())
}


pub async fn move_folder_under(db_pool: &PgPool, user_id: &Uuid, folder_id: &Uuid, new_parent_folder_id: Option<Uuid>) -> Result<(), AppError> {
    // Folder can not move under itself
    if let Some(new_parent_id) = new_parent_folder_id {
        if new_parent_id == *folder_id {
            return Err(AppError::BadRequest("Folder cannot be moved under itself".into()));
        }
    }

    // Folder can not move under a child of itself to prevent circular hierarchy
    if let Some(new_parent_id) = new_parent_folder_id {
        if is_folder_under_parent(db_pool, &new_parent_id, folder_id).await? {
            return Err(AppError::BadRequest("Folder cannot be moved under a child of itself".into()));
        }
    }

    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            UPDATE folders
            SET parent_folder_id = $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE folder_id = $2
              AND user_id = $3
              AND deleted_at IS NULL
            "#,
        &[
            &new_parent_folder_id,
            &folder_id,
            &user_id,
        ],
    )
    .await?;

    Ok(())
}


pub async fn list_internal_sharing_in_folders(db_pool: &PgPool, user_id: &Uuid, limit: i64, offset: i64) -> Result<Vec<InternalSharedResource>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT
                true AS is_resource_folder,

                f.user_id,

                f.folder_id AS resource_id,
                f.folder_name AS resource_name,
                f.folder_info AS resource_info,

                COALESCE(
                    (
                        SELECT SUM(fv.file_size)
                        FROM files fi
                        INNER JOIN file_versions fv
                            ON fv.file_id = fi.file_id
                        WHERE fi.folder_id = f.folder_id
                          AND fi.deleted_at IS NULL
                    ),
                    0
                )::BIGINT AS total_resource_size,

                s.can_preview,
                s.can_download,
                s.can_create,
                s.can_update,
                s.can_delete,

                f.created_at,
                f.updated_at

            FROM internal_shares s
            INNER JOIN folders f
                ON f.folder_id = s.folder_id
               AND f.deleted_at IS NULL

            WHERE s.shared_with_user_id = $1
              AND f.deleted_at IS NULL

            ORDER BY f.created_at DESC
            LIMIT $2
            OFFSET $3
            "#,
            &[
                user_id,
                &limit,
                &offset,
            ],
        )
        .await?;

    Ok(InternalSharedResource::from_rows(rows))
}


pub async fn list_internal_sharing_out_folders(db_pool: &PgPool, user_id: &Uuid, limit: i64, offset: i64) -> Result<Vec<InternalSharedResource>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT
                true AS is_resource_folder,

                s.shared_with_user_id AS user_id,

                f.folder_id AS resource_id,
                f.folder_name AS resource_name,
                f.folder_info AS resource_info,

                COALESCE(
                    (
                        SELECT SUM(fv.file_size)
                        FROM files fi
                        INNER JOIN file_versions fv
                            ON fv.file_id = fi.file_id
                        WHERE fi.folder_id = f.folder_id
                          AND fi.deleted_at IS NULL
                    ),
                    0
                )::BIGINT AS total_resource_size,

                s.can_preview,
                s.can_download,
                s.can_create,
                s.can_update,
                s.can_delete,

                f.created_at,
                f.updated_at

            FROM folders f
            INNER JOIN LATERAL (
                SELECT
                    s.shared_with_user_id,
                    s.can_preview,
                    s.can_download,
                    s.can_create,
                    s.can_update,
                    s.can_delete
                FROM internal_shares s
                WHERE s.folder_id = f.folder_id
                  AND s.created_by = $1
                  AND f.deleted_at IS NULL
                ORDER BY s.created_at DESC
                LIMIT 1
            ) s ON TRUE

            WHERE f.user_id = $1
              AND f.deleted_at IS NULL

            ORDER BY f.created_at DESC
            LIMIT $2
            OFFSET $3
            "#,
            &[
                user_id,
                &limit,
                &offset,
            ],
        )
        .await?;

    Ok(InternalSharedResource::from_rows(rows))
}


/// Check if the folder_id is part of given parent folder id at any given level
pub async fn is_folder_under_parent(db_pool: &PgPool, child_folder_id: &Uuid, parent_folder_id: &Uuid) -> Result<bool, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_opt(
            r#"
            WITH RECURSIVE folder_hierarchy AS (
                SELECT
                    folder_id,
                    parent_folder_id
                FROM folders
                WHERE folder_id = $1

                UNION ALL

                SELECT
                    f.folder_id,
                    f.parent_folder_id
                FROM folders f
                INNER JOIN folder_hierarchy fh
                    ON f.folder_id = fh.parent_folder_id
                WHERE fh.parent_folder_id IS NOT NULL
            )
            SELECT 1
            FROM folder_hierarchy
            WHERE folder_id = $2
            LIMIT 1
            "#,
            &[child_folder_id, parent_folder_id],
        )
        .await?;

    Ok(row.is_some())
}


pub async fn is_folder_belongs_to_user(db_pool: &PgPool, folder_id: &Uuid, user_id: &Uuid) -> Result<bool, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_opt(
            r#"
            SELECT 1
            FROM folders
            WHERE folder_id = $1
              AND user_id = $2
            LIMIT 1
            "#,
            &[folder_id, user_id],
        )
        .await?;

    Ok(row.is_some())
}


pub async fn mark_folders_and_files_as_deleted(db_pool: &PgPool, folder_ids: &[Uuid]) -> Result<(), AppError> {
    if folder_ids.is_empty() {
        return Ok(());
    }

    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            WITH RECURSIVE folder_tree AS (
                SELECT folder_id
                FROM folders
                WHERE folder_id = ANY($1)

                UNION

                SELECT f.folder_id
                FROM folders f
                INNER JOIN folder_tree ft
                    ON f.parent_folder_id = ft.folder_id
            ),
            deleted_folders AS (
                UPDATE folders
                SET deleted_at = NOW(),
                    updated_at = NOW()
                WHERE folder_id IN (
                    SELECT folder_id
                    FROM folder_tree
                )
                RETURNING folder_id
            )
            UPDATE files
            SET deleted_at = NOW(),
                updated_at = NOW()
            WHERE folder_id IN (
                SELECT folder_id
                FROM folder_tree
            )
            "#,
            &[&folder_ids],
        )
        .await?;

    Ok(())
}


pub async fn get_files_in_folder(db_pool: &PgPool, folder_id: &Uuid) -> Result<Vec<Uuid>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT file_id
            FROM files
            WHERE folder_id = $1
            "#,
            &[folder_id],
        )
        .await?;

    let file_ids: Vec<Uuid> = rows.iter().map(|row| row.get("file_id")).collect();

    Ok(file_ids)
}


pub async fn get_subfolders_in_folder(db_pool: &PgPool, folder_id: &Uuid) -> Result<Vec<Uuid>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT folder_id
            FROM folders
            WHERE parent_folder_id = $1
            "#,
            &[folder_id],
        )
        .await?;

    let folder_ids: Vec<Uuid> = rows.iter().map(|row| row.get("folder_id")).collect();

    Ok(folder_ids)
}


pub async fn delete_folder_if_empty(db_pool: &PgPool, folder_id: &Uuid) -> Result<bool, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_opt(
            r#"
            DELETE FROM folders
            WHERE folder_id = $1
              AND NOT EXISTS (
                  SELECT 1
                  FROM files
                  WHERE folder_id = $1
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM folders
                  WHERE parent_folder_id = $1
              )
            RETURNING folder_id
            "#,
            &[folder_id],
        )
        .await?;

    Ok(row.is_some())
}
