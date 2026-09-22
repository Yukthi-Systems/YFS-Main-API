use crate::models::files::{BasicFileInfo, FileLocation, FileVersionInfo};
use deadpool_postgres::Pool as PgPool;
use crate::models::errors::AppError;
use uuid::Uuid;



pub async fn get_file_info_by_id(db_pool: &PgPool, folder_id: &Uuid, file_id: &Uuid) -> Result<Option<BasicFileInfo>, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_opt(
            r#"
            SELECT
                f.file_id,
                f.folder_id,
                f.user_id,
                f.file_name,
                f.file_info,
                f.is_locked,
                ARRAY(
                    SELECT fv.file_version
                    FROM file_versions fv
                    WHERE fv.file_id = f.file_id
                ) AS available_versions,
                f.created_at,
                f.updated_at
            FROM files f
            LEFT JOIN file_versions fv
                ON fv.file_id = f.file_id
            WHERE f.folder_id = $1 AND f.file_id = $2 AND f.deleted_at IS NULL
            GROUP BY
                f.file_id,
                f.folder_id,
                f.user_id,
                f.file_name,
                f.file_info,
                f.is_locked,
                f.created_at,
                f.updated_at
            "#,
            &[folder_id, file_id],
        )
        .await?;

    Ok(row.map(BasicFileInfo::from))
}


pub async fn get_file_location(db_pool: &PgPool, folder_id: &Uuid, file_id: &Uuid, file_version: i32) -> Result<Option<FileLocation>, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_opt(
            r#"
            SELECT fv.file_location, fv.hosted_at
            FROM file_versions fv
            INNER JOIN files f
                ON f.file_id = fv.file_id
            WHERE f.folder_id = $1
            AND fv.file_id = $2
            AND fv.file_version = $3
            "#,
            &[folder_id, file_id, &file_version],
        )
        .await?;

    Ok(row.map(FileLocation::from))
}


pub async fn create_base_file_entry(
    db_pool: &PgPool,
    file_id: &Uuid,
    folder_id: &Uuid,
    user_id: &Uuid,
    file_name: &str,
    file_info: &serde_json::Value,
    is_locked: bool
) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client.execute(
        r#"
        INSERT INTO files (file_id, folder_id, user_id, file_name, file_info, is_locked)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        &[file_id, folder_id, user_id, &file_name, file_info, &is_locked],
    )
    .await?;

    Ok(())
}


pub async fn lock_base_file_entry(db_pool: &PgPool, folder_id: &Uuid, user_id: &Uuid, file_id: &Uuid, is_locked: bool) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client.execute(
        r#"
        UPDATE files
        SET is_locked = $1,
            updated_at = CURRENT_TIMESTAMP
        WHERE folder_id = $2 AND user_id = $3 AND file_id = $4
        "#,
        &[&is_locked, folder_id, user_id, file_id],
    )
    .await?;

    Ok(())
}


pub async fn add_or_update_file_version(
    db_pool: &PgPool,
    file_id: &Uuid,
    file_version: i32,
    file_location: &str,
    owner_id: &Uuid,
    hosted_at: &str,
    file_size: i64,
    metadata: &serde_json::Value,
    file_hash: &str
) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client.execute(
        r#"
        INSERT INTO file_versions (
            file_id,
            file_version,
            user_id,
            hosted_at,
            file_location,
            file_size,
            metadata,
            file_hash
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (file_id, file_version) DO UPDATE
        SET file_location = EXCLUDED.file_location,
            file_size = EXCLUDED.file_size,
            metadata = EXCLUDED.metadata,
            file_hash = EXCLUDED.file_hash,
            created_at = CURRENT_TIMESTAMP
        "#,
        &[
            &file_id,
            &file_version,
            &owner_id,
            &hosted_at,
            &file_location,
            &file_size,
            metadata,
            &file_hash
        ],
    )
    .await?;

    Ok(())
}


pub async fn delete_file(db_pool: &PgPool, folder_id: &Uuid, user_id: &Uuid, file_id: &Uuid) -> Result<(i64, i32), AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_one(
            r#"
            WITH file_stats AS (
                SELECT
                    COALESCE(SUM(fv.file_size), 0)::BIGINT AS total_size,
                    COUNT(fv.file_id)::INT AS total_versions
                FROM files f
                LEFT JOIN file_versions fv
                    ON fv.file_id = f.file_id
                WHERE f.file_id = $1
                  AND f.folder_id = $2
                  AND f.user_id = $3
            ),
            deleted AS (
                DELETE FROM files
                WHERE file_id = $1
                  AND folder_id = $2
                  AND user_id = $3
                RETURNING file_id
            )
            SELECT
                file_stats.total_size,
                file_stats.total_versions
            FROM file_stats
            WHERE EXISTS (
                SELECT 1 FROM deleted
            )
            "#,
            &[file_id, folder_id, user_id],
        )
        .await?;

    Ok((row.get("total_size"), row.get("total_versions")))
}


pub async fn update_base_file_info(
    db_pool: &PgPool,
    file_id: &Uuid,
    folder_id: &Uuid,
    user_id: &Uuid,
    file_name: &str,
    file_info: &serde_json::Value,
) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client.execute(
        r#"
        UPDATE files
        SET file_name = $1,
            file_info = $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE file_id = $3 AND folder_id = $4 AND user_id = $5 AND is_locked = FALSE
        "#,
        &[&file_name, file_info, file_id, folder_id, user_id],
    )
    .await?;

    Ok(())
}


pub async fn move_file_to_folder(
    db_pool: &PgPool,
    file_id: &Uuid,
    current_folder_id: &Uuid,
    destination_folder_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client.execute(
        r#"
        UPDATE files
        SET folder_id = $1,
            updated_at = CURRENT_TIMESTAMP
        WHERE file_id = $2 AND folder_id = $3 AND user_id = $4 AND is_locked = FALSE
        "#,
        &[destination_folder_id, file_id, current_folder_id, user_id],
    )
    .await?;

    Ok(())
}


pub async fn remove_all_expired_file_locks(db_pool: &PgPool) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    log::info!("Executing SQL query to remove all expired file locks");

    client.execute(
        r#"
        UPDATE files
        SET is_locked = FALSE,
            updated_at = CURRENT_TIMESTAMP
        WHERE is_locked = TRUE AND updated_at < NOW() - INTERVAL '5400 seconds'
        "#,
        &[],
    )
    .await?;

    log::info!("Expired file locks removed successfully");

    Ok(())
}


pub async fn delete_all_orphaned_files(db_pool: &PgPool) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    log::info!("Executing SQL query to delete all orphaned files");

    client.execute(
        r#"
        DELETE FROM files f
        WHERE f.created_at < NOW() - INTERVAL '65536 seconds'
        AND NOT EXISTS (
            SELECT 1
            FROM file_versions fv
            WHERE fv.file_id = f.file_id
        )
        "#,
        &[],
    )
    .await?;

    log::info!("Orphaned files deleted successfully");

    Ok(())
}


pub async fn get_file_version_details(db_pool: &PgPool, owner_id: &Uuid, file_id: &Uuid) -> Result<Vec<FileVersionInfo>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT
                file_version,
                hosted_at,
                file_location,
                file_size
            FROM file_versions
            WHERE
                user_id = $1
                AND file_id = $2
            "#,
            &[owner_id, file_id],
        )
        .await?;

    Ok(FileVersionInfo::from_rows(rows))
}


pub async fn delete_file_versions(db_pool: &PgPool, file_id: &Uuid, file_versions: &[i32]) -> Result<(i64, i32), AppError> {
    let client = db_pool.get().await?;

    // This will delete the specified file versions, update the file's timestamp if there are remaining versions,
    // and delete the file itself if no versions remain, all in a single query using CTEs.
    let row = client
        .query_one(
            r#"
            WITH existing AS (
                SELECT COUNT(*)::INT AS total_count
                FROM file_versions
                WHERE file_id = $1
            ),
            deleted AS (
                DELETE FROM file_versions
                WHERE file_id = $1
                  AND file_version = ANY($2)
                RETURNING file_id, file_size
            ),
            deleted_file AS (
                DELETE FROM files
                WHERE file_id = $1
                  AND EXISTS (
                      SELECT 1
                      FROM deleted
                  )
                  AND (
                      SELECT total_count
                      FROM existing
                  ) = (
                      SELECT COUNT(*)::INT
                      FROM deleted
                  )
                RETURNING file_id
            )
            SELECT
                COALESCE(SUM(file_size), 0)::BIGINT AS total_deleted_size,
                COUNT(*)::INT AS total_deleted_count
            FROM deleted
            "#,
            &[file_id, &file_versions],
        )
        .await?;

    Ok((row.get("total_deleted_size"), row.get("total_deleted_count")))
}
