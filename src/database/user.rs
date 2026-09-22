use crate::models::user::{BasicUserInfo, SessionUser, UserQuota, ServerInfo};
use deadpool_postgres::Pool as PgPool;
use crate::models::errors::AppError;
use uuid::Uuid;


pub async fn create_user_session(db_pool: &PgPool, user_session: &SessionUser) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    // Create a user if not exists, then create the session with the provided details
    client
        .execute(
            r#"
            WITH upsert_user AS (
                INSERT INTO users (
                    user_id,
                    email,
                    domain,

                    organization_id,
                    organization_name,

                    private_info,
                    public_info
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (email) DO UPDATE
                SET
                    domain = EXCLUDED.domain,
                    organization_id = EXCLUDED.organization_id,
                    organization_name = EXCLUDED.organization_name,
                    private_info = EXCLUDED.private_info,
                    public_info = EXCLUDED.public_info
                RETURNING user_id
            )

            INSERT INTO sessions (
                user_id,
                refresh_token,
                sso_token
            )
            SELECT 
                user_id,
                $8,     -- refresh_token
                $9     -- sso_token
            FROM upsert_user
            "#,
        &[
            &user_session.user_id,
            &user_session.email,
            &user_session.domain_name,
            &user_session.organization_id,
            &user_session.organization_name,
            &serde_json::json!({}), // private_info
            &serde_json::json!({}), // public_info
            &user_session.refresh_token,
            &user_session.sso_token,
        ],
    )
    .await?;

    Ok(())
}


pub async fn update_user_session_fcm_token(db_pool: &PgPool, user_id: &Uuid, fcm_token: &str) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            UPDATE sessions
            SET fcm_token = $2
            WHERE user_id = $1
            "#,
            &[user_id, &fcm_token],
        )
        .await?;

    Ok(())
}


pub async fn update_user_last_seen(db_pool: &PgPool, user_id: &Uuid) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            UPDATE users
            SET last_seen_at = CURRENT_TIMESTAMP
            WHERE user_id = $1
            "#,
            &[user_id],
        )
        .await?;

    Ok(())
}


pub async fn delete_user_session(db_pool: &PgPool, user_id: &Uuid, refresh_token: &Uuid) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            DELETE FROM sessions WHERE user_id = $1 AND refresh_token = $2
            "#,
            &[user_id, refresh_token],
        )
        .await?;

    Ok(())
}


pub async fn check_user_session(db_pool: &PgPool, refresh_token: &Uuid, sso_token: &str, user_id: &Uuid) -> Result<bool, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_one(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM sessions s
                INNER JOIN users u
                    ON u.user_id = s.user_id
                WHERE s.user_id = $1
                  AND s.refresh_token = $2
                  AND s.sso_token = $3
                  AND s.expires_at >= CURRENT_TIMESTAMP
            )
            "#,
            &[user_id, refresh_token, &sso_token],
        )
        .await?;

    Ok(row.get(0))
}


pub async fn get_user_by_id(db_pool: &PgPool, user_id: &Uuid, organization_id: &Uuid) -> Result<Option<BasicUserInfo>, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_opt(
            r#"
            SELECT user_id, email, domain, private_info, public_info, last_seen_at
            FROM users
            WHERE user_id = $1 AND organization_id = $2
            "#,
            &[user_id, &organization_id],
        )
        .await?;

    Ok(row.map(BasicUserInfo::from))
}


pub async fn search_user_by_email(db_pool: &PgPool, email: &str, organization_id: &Uuid) -> Result<Vec<BasicUserInfo>, AppError> {
    let client = db_pool.get().await?;

    // Note: Fakes a private_info column to maintain consistency with BasicUserInfo
    // Have a limit of 10 results
    let rows = client
        .query(
            r#"
            SELECT user_id, email, domain, '{}'::jsonb AS private_info, public_info, last_seen_at
            FROM users
            WHERE email ILIKE $1 AND organization_id = $2
            LIMIT 10
            "#,
            &[&format!("%{}%", email), &organization_id],
        )
        .await?;

    Ok(rows.into_iter().map(BasicUserInfo::from).collect())
}


pub async fn update_user_public_info(db_pool: &PgPool, user_id: &Uuid, public_info: &serde_json::Value) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            UPDATE users
            SET public_info = $1, last_seen_at = CURRENT_TIMESTAMP
            WHERE user_id = $2
            "#,
            &[public_info, user_id],
        )
        .await?;

    Ok(())
}


pub async fn update_user_private_info(db_pool: &PgPool, user_id: &Uuid, private_info: &serde_json::Value) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    client
        .execute(
            r#"
            UPDATE users
            SET private_info = $1, last_seen_at = CURRENT_TIMESTAMP
            WHERE user_id = $2
            "#,
            &[private_info, user_id],
        )
        .await?;

    Ok(())
}


pub async fn get_user_quota_by_email(db_pool: &PgPool, user_email: &str) -> Result<UserQuota, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_one(
            r#"
            SELECT used_storage_bytes, used_file_count
            FROM user_quotas
            WHERE user_id = (
                SELECT user_id
                FROM users
                WHERE email = $1
            )
            "#,
            &[&user_email],
        )
        .await?;

    Ok(UserQuota::from(row))
}


pub async fn get_user_quota_by_id(db_pool: &PgPool, user_id: &Uuid) -> Result<UserQuota, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_one(
            r#"
            SELECT used_storage_bytes, used_file_count
            FROM user_quotas
            WHERE user_id = $1
            "#,
            &[&user_id],
        )
        .await?;

    Ok(UserQuota::from(row))
}


pub async fn recalculate_user_quota(db_pool: &PgPool, user_id: &Uuid) -> Result<UserQuota, AppError> {
    let client = db_pool.get().await?;

    let row = client
        .query_one(
            r#"
            INSERT INTO user_quotas (
                user_id,
                used_storage_bytes,
                used_file_count
            )
            SELECT
                u.user_id,
                COALESCE(SUM(fv.file_size), 0)::BIGINT AS used_storage_bytes,
                COUNT(fv.file_version)::INT AS used_file_count
            FROM users u
            LEFT JOIN files f
                ON f.user_id = u.user_id
            LEFT JOIN file_versions fv
                ON fv.file_id = f.file_id
            WHERE u.user_id = $1
            GROUP BY u.user_id
            ON CONFLICT (user_id) DO UPDATE
            SET
                used_storage_bytes = EXCLUDED.used_storage_bytes,
                used_file_count = EXCLUDED.used_file_count
            RETURNING
                used_storage_bytes,
                used_file_count
            "#,
            &[&user_id],
        )
        .await?;

    Ok(UserQuota::from(row))
}


pub async fn update_quota(db_pool: &PgPool, user_id: &Uuid, server_host_address: &str, add_quota_bytes: i64, add_file_count: i32) -> Result<(), AppError> {
    let client = db_pool.get().await?;

    // Update the user's quota and the server's utilized quota in a single query using a CTE
    client
        .execute(
            r#"
            WITH updated_user_quota AS (
                UPDATE user_quotas
                SET used_storage_bytes = used_storage_bytes + $1,
                    used_file_count = used_file_count + $2
                WHERE user_id = $3
            )
            UPDATE servers
            SET quota_utilized_bytes = quota_utilized_bytes + $1
            WHERE host_address = $4
            "#,
            &[
                &add_quota_bytes,
                &add_file_count,
                user_id,
                &server_host_address,
            ],
        )
        .await?;

    Ok(())
}


pub async fn get_all_servers_info(db_pool: &PgPool) -> Result<Vec<ServerInfo>, AppError> {
    let client = db_pool.get().await?;

    let rows = client
        .query(
            r#"
            SELECT
                host_address,
                dedicated_to_organization_id,
                server_name,
                server_description,
                quota_allocated_bytes,
                quota_utilized_bytes
            FROM servers
            "#,
            &[],
        )
        .await?;

    Ok(ServerInfo::from_rows(rows))
}
