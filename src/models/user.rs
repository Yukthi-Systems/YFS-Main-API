use crate::models::{PermissionSet, errors::AppError, shares::ExternalShare};
use serde::{Deserialize, Serialize};
use crate::state::API_SETTINGS;
use tokio_postgres::row::Row;
use uuid::Uuid;

type ChronoUtc = chrono::DateTime<chrono::Utc>;



#[derive(Serialize, Deserialize)]
pub struct SessionUser {
    pub refresh_token: Uuid,
    pub sso_token: String,
    pub fcm_token: Option<String>,

    pub organization_id: Uuid,
    pub organization_name: String,

    pub user_id: Uuid,
    pub email: String,
    pub domain_name: String,

    pub is_file_versioning_enabled: bool,
    pub is_sharing_enabled: bool,

    pub quota_allocated: f64,
    pub quota_utilized: f64,
}


#[derive(Serialize, Deserialize)]
pub struct PublicSessionUser {
    pub share_id: String,
    pub created_by: Uuid,
    pub access_token: Uuid,
    pub is_session_active: bool,

    pub share_folder_target_id: Option<Uuid>,
    pub share_file_target_id: Option<Uuid>,
    pub permission_set: PermissionSet,
}


#[derive(Deserialize)]
struct FileServiceInfo {
    email: String,
    domain_name: String,

    organization_id: Uuid,
    organization_name: String,

    is_file_versioning_enabled: bool,
    is_sharing_enabled: bool,

    quota_allocated: f64,
    quota_utilized: f64,
}


#[derive(Serialize, Deserialize)]
pub struct BasicUserInfo {
    pub user_id: Uuid,
    pub email: String,
    pub domain_name: String,
    pub private_info: serde_json::Value,
    pub public_info: serde_json::Value,
    pub last_seen_at: String,
}


#[derive(Serialize)]
pub struct UserQuota {
    pub used_storage_bytes: i64,
    pub used_file_count: i32,
}


#[derive(Serialize)]
pub struct ServerInfo {
    pub host_address: String,
    pub secret_key: String,

    pub dedicated_to_organization_id: Option<Uuid>,

    pub server_name: String,
    pub server_description: String,

    pub quota_allocated_bytes: i64,
    pub quota_utilized_bytes: i64,
}


// ------- Implementations ------- //


impl From<Row> for SessionUser {
    fn from(row: Row) -> Self {
        SessionUser {
            refresh_token: row.get("refresh_token"),
            sso_token: row.get("sso_token"),
            fcm_token: row.get("fcm_token"),

            email: row.get("email"),
            domain_name: row.get("domain"),

            organization_id: row.get("organization_id"),
            organization_name: row.get("organization_name"),

            user_id: row.get("user_id"),
            is_file_versioning_enabled: row.get("is_file_versioning_enabled"),
            is_sharing_enabled: row.get("is_sharing_enabled"),

            quota_allocated: row.get("quota_allocated"),
            quota_utilized: row.get("quota_utilized"),
        }
    }
}


impl From<Row> for ServerInfo {
    fn from(row: Row) -> Self {
        ServerInfo {
            host_address: row.get("host_address"),
            secret_key: row.get("secret_key"),
            dedicated_to_organization_id: row.get("dedicated_to_organization_id"),
            server_name: row.get("server_name"),
            server_description: row.get("server_description"),
            quota_allocated_bytes: row.get("quota_allocated_bytes"),
            quota_utilized_bytes: row.get("quota_utilized_bytes"),
        }
    }
}


impl ServerInfo {
    pub fn from_rows(rows: Vec<Row>) -> Vec<Self> {
        rows.into_iter().map(Self::from).collect()
    }
}


impl SessionUser {
    pub async fn new(sso_cookie_token: &str, refresh_token: &Uuid) -> Result<Self, AppError> {
        // Make API call and fetch info and check the SSO cookie token
        let file_service_info = Self::fetch_sso_info(sso_cookie_token).await?;
        
        // TODO: Make sure the user id is from SSO and not generated locally (currently its fake)
        let generated_user_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, file_service_info.email.as_bytes());

        Ok(SessionUser {
            refresh_token: *refresh_token,
            sso_token: sso_cookie_token.to_string(),
            fcm_token: None,

            email: file_service_info.email,
            domain_name: file_service_info.domain_name,

            organization_id: file_service_info.organization_id,
            organization_name: file_service_info.organization_name,

            user_id: generated_user_id,
            is_file_versioning_enabled: file_service_info.is_file_versioning_enabled,
            is_sharing_enabled: file_service_info.is_sharing_enabled,

            quota_allocated: file_service_info.quota_allocated,
            quota_utilized: file_service_info.quota_utilized,
        })
    }


    /// Fetches the SSO information for the user using the provided SSO cookie token
    async fn fetch_sso_info(sso_cookie_token: &str) -> Result<FileServiceInfo, AppError> {
        let client = reqwest::Client::new();
        let url = format!("{}/internal/file/user-info/{}", API_SETTINGS.sso_api_url, sso_cookie_token);

        let resp = client
            .get(&url)
            .header("accept", "application/json")
            .header("X-API-Key", &API_SETTINGS.sso_api_key)
            .send()
            .await?
            .json::<FileServiceInfo>()
            .await?;

        Ok(resp)
    }


    /// Validates the user's quota to ensure they have enough available storage for new uploads.
    pub fn validate_quota(&self, required_space_bytes: f64) -> Result<(), AppError> {
        let available_space: f64 = self.quota_allocated - self.quota_utilized;
        let required_space: f64 = required_space_bytes / (1024.0 * 1024.0 * 1024.0);

        if available_space < required_space {
            return Err(AppError::Unprocessable("Not enough quota available".into()));
        }
        Ok(())
    }
}


impl From<Row> for BasicUserInfo {
    fn from(row: Row) -> Self {
        BasicUserInfo {
            user_id: row.get("user_id"),
            email: row.get("email"),
            domain_name: row.get("domain"),
            private_info: row.get("private_info"),
            public_info: row.get("public_info"),
            last_seen_at: row.get::<_, ChronoUtc>("last_seen_at").to_rfc3339(),
        }
    }
}


impl PublicSessionUser {
    pub fn new(external_share: ExternalShare, activate: bool) -> Self {
        PublicSessionUser {
            share_id: external_share.share_id,
            created_by: external_share.created_by,
            access_token: Uuid::new_v4(),
            is_session_active: activate,
            share_file_target_id: external_share.share_file_target_id,
            share_folder_target_id: external_share.share_folder_target_id,
            permission_set: external_share.permission_set,
        }
    }

    pub fn cache_key(&self) -> String {
        format!("public:{}", self.access_token)
    }
}


impl From<Row> for UserQuota {
    fn from(row: Row) -> Self {
        UserQuota {
            used_storage_bytes: row.get("used_storage_bytes"),
            used_file_count: row.get("used_file_count"),
        }
    }
}
