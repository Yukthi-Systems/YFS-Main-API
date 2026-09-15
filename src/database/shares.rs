use crate::models::shares::{ExternalShare, InternalShareRequest};
use deadpool_postgres::Pool as PgPool;
use crate::models::errors::AppError;
use uuid::Uuid;



pub async fn create_internal_share(
    db_pool: &PgPool,
    folder_id: &Uuid,
    shared_with_user_id: &Uuid,
    created_by: &Uuid,
    can_preview: bool,
    can_download: bool,
    can_create: bool,
    can_update: bool,
    can_delete: bool,
) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    // Check if the folder ID is of same org. and belongs to the current user
    let folder_exists = client
        .query(
            r#"
            SELECT folder_id
            FROM folders
            WHERE folder_id = $1
              AND user_id = $2
              AND deleted_at IS NULL
            "#,
            &[folder_id, created_by],
        )
        .await?;

    if folder_exists.is_empty() {
        return Err(AppError::NotFound("Folder does not exist or does not belong to the current user".into()));
    }

    client
        .execute(
            r#"
            INSERT INTO internal_shares (
                folder_id,
                shared_with_user_id,
                can_preview,
                can_download,
                can_create,
                can_update,
                can_delete,
                created_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            &[
                folder_id,
                shared_with_user_id,
                &can_preview,
                &can_download,
                &can_create,
                &can_update,
                &can_delete,
                created_by,
            ],
        )
        .await?;

    Ok(())
}


pub async fn delete_internal_share(db_pool: &PgPool, folder_id: &Uuid, shared_with_user_id: &Uuid, created_by: &Uuid) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            DELETE FROM internal_shares
            WHERE folder_id = $1
              AND shared_with_user_id = $2
              AND created_by = $3
            "#,
            &[folder_id, shared_with_user_id, created_by],
        )
        .await?;

    Ok(())
}


pub async fn update_internal_share(
    db_pool: &PgPool,
    folder_id: &Uuid,
    shared_with_user_id: &Uuid,
    created_by: &Uuid,
    can_preview: bool,
    can_download: bool,
    can_create: bool,
    can_update: bool,
    can_delete: bool,
) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            UPDATE internal_shares
            SET can_preview = $1,
                can_download = $2,
                can_create = $3,
                can_update = $4,
                can_delete = $5
            WHERE folder_id = $6
              AND shared_with_user_id = $7
              AND created_by = $8
            "#,
            &[
                &can_preview,
                &can_download,
                &can_create,
                &can_update,
                &can_delete,
                folder_id,
                shared_with_user_id,
                created_by,
            ],
        )
        .await?;

    Ok(())
}


pub async fn get_internal_share_info(db_pool: &PgPool, folder_id: &Uuid, user_id: &Uuid) -> Result<Vec<InternalShareRequest>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT
                folder_id,
                shared_with_user_id,
                can_preview,
                can_download,
                can_create,
                can_update,
                can_delete
            FROM internal_shares
            WHERE folder_id = $1
              AND created_by = $2
            "#,
            &[folder_id, user_id],
        )
        .await?;

    Ok(InternalShareRequest::from_rows(rows))
}


/// Checks the internal shared permissions for a given folder and user if an internal share exists
pub async fn internal_shared_permissions(db_pool: &PgPool, folder_id: &Uuid, shared_with_user_id: &Uuid) -> Result<Option<InternalShareRequest>, AppError> {
    let client = db_pool.get().await?;

    // Note: created_by is used as shared_with_user_id to maintain consistency with InternalShareRequest structure
    let row = client
        .query_opt(
            r#"
            SELECT
                folder_id,
                created_by AS shared_with_user_id,
                can_preview,
                can_download,
                can_create,
                can_update,
                can_delete
            FROM internal_shares
            WHERE folder_id = $1
              AND shared_with_user_id = $2
            "#,
            &[folder_id, shared_with_user_id],
        )
        .await?;

    Ok(row.map(InternalShareRequest::from))
}


pub async fn create_external_share_in_db(
    db_pool: &PgPool,
    share_id: &str,
    created_by: &Uuid,
    share_file_target_id: Option<Uuid>,
    share_folder_target_id: Option<Uuid>,
    can_preview: bool,
    can_download: bool,
    can_create: bool,
    can_update: bool,
    can_delete: bool,
    share_info: &serde_json::Value,
    password_hash: Option<String>,
    phones_for_otp: &[String],
    emails_for_otp: &[String],
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            INSERT INTO external_shares (
                share_id,
                created_by,
                share_file_target_id,
                share_folder_target_id,
                can_preview,
                can_download,
                can_create,
                can_update,
                can_delete,
                share_info,
                password_hash,
                phones_for_otp,
                emails_for_otp,
                expires_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
            "#,
            &[
                &share_id,
                &created_by,
                &share_file_target_id,
                &share_folder_target_id,
                &can_preview,
                &can_download,
                &can_create,
                &can_update,
                &can_delete,
                &share_info,
                &password_hash,
                &phones_for_otp,
                &emails_for_otp,
                &expires_at,
            ],
        )
        .await?;

    Ok(())
}


pub async fn delete_external_share_in_db(db_pool: &PgPool, user_id: &Uuid, share_id: &str) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            DELETE FROM external_shares
            WHERE share_id = $1 AND created_by = $2
            "#,
            &[&share_id, user_id],
        )
        .await?;

    Ok(())
}


pub async fn update_external_share_in_db(
    db_pool: &PgPool,
    created_by: &Uuid,
    share_id: &str,
    can_preview: bool,
    can_download: bool,
    can_create: bool,
    can_update: bool,
    can_delete: bool,
    share_info: &serde_json::Value,
    update_password_hash: bool,
    password_hash: Option<String>,
    phones_for_otp: &[String],
    emails_for_otp: &[String],
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    // If update_password_hash is true, set password_hash to what ever value is passed (if null then remove)
    client
        .execute(
            r#"
            UPDATE external_shares
            SET
                can_preview = $1,
                can_download = $2,
                can_create = $3,
                can_update = $4,
                can_delete = $5,
                share_info = $6,
                password_hash = CASE WHEN $7 THEN $8 ELSE password_hash END,
                phones_for_otp = $9,
                emails_for_otp = $10,
                expires_at = $11
            WHERE share_id = $12 AND created_by = $13
            "#,
            &[
                &can_preview,
                &can_download,
                &can_create,
                &can_update,
                &can_delete,
                &share_info,
                &update_password_hash,
                &password_hash,
                &phones_for_otp,
                &emails_for_otp,
                &expires_at,
                &share_id,
                &created_by,
            ],
        )
        .await?;

    Ok(())
}


pub async fn list_external_shares_in_db(db_pool: &PgPool, user_id: &Uuid, limit: i64, offset: i64) -> Result<Vec<ExternalShare>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT
                es.share_id,
                es.created_by,
                u.organization_id,
                es.share_file_target_id,
                es.share_folder_target_id,
                es.can_preview,
                es.can_download,
                es.can_create,
                es.can_update,
                es.can_delete,
                es.share_info,
                es.password_hash,
                es.phones_for_otp,
                es.emails_for_otp,
                es.expires_at,
                es.created_at
            FROM external_shares es
            JOIN users u ON u.user_id = es.created_by
            WHERE es.created_by = $1
            ORDER BY es.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            &[user_id, &limit, &offset],
        )
        .await?;

    Ok(ExternalShare::from_rows(rows))
}


pub async fn get_external_share_by_id(db_pool: &PgPool, share_id: &str) -> Result<Option<ExternalShare>, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_opt(
            r#"
            SELECT
                es.share_id,
                es.created_by,
                u.organization_id,
                es.share_file_target_id,
                es.share_folder_target_id,
                es.can_preview,
                es.can_download,
                es.can_create,
                es.can_update,
                es.can_delete,
                es.share_info,
                es.password_hash,
                es.phones_for_otp,
                es.emails_for_otp,
                es.expires_at,
                es.created_at
            FROM external_shares es
            JOIN users u ON u.user_id = es.created_by
            WHERE es.share_id = $1
            "#,
            &[&share_id],
        )
        .await?;

    Ok(row.map(ExternalShare::from))
}
