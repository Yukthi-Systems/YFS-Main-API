use crate::models::files::BasicFileInfo;
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


pub async fn get_file_location(db_pool: &PgPool, folder_id: &Uuid, file_id: &Uuid, file_version: i32) -> Result<Option<String>, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_opt(
            r#"
            SELECT fv.file_location
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

    Ok(row.map(|r| r.get("file_location")))
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
        SET is_locked = $1
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
            file_location,
            file_size,
            metadata,
            file_hash
        )
        VALUES ($1, $2, $3, $4, $5, $6)
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
            &file_location,
            &file_size,
            metadata,
            &file_hash
        ],
    )
    .await?;

    Ok(())
}


pub async fn delete_file(db_pool: &PgPool, folder_id: &Uuid, user_id: &Uuid, file_id: &Uuid) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client.execute(
        r#"
        DELETE FROM files
        WHERE file_id = $1 AND folder_id = $2 AND user_id = $3
        "#,
        &[file_id, folder_id, user_id],
    )
    .await?;

    Ok(())
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
