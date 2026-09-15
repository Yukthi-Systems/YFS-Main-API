use serde::{Serialize, Deserialize};
use crate::models::PermissionSet;
use tokio_postgres::row::Row;
use uuid::Uuid;


type ChronoUtc = chrono::DateTime<chrono::Utc>;


#[derive(Serialize)]
pub struct InternalSharedResource {
    pub is_resource_folder: bool,

    // If its sharing-in, then its the owner of the resource, if its sharing-out, then its the user who its shared with
    pub user_id: Uuid,

    pub resource_id: Uuid,
    pub resource_name: String,
    pub resource_info: serde_json::Value,

    pub total_resource_size: i64,
    pub permission_set: PermissionSet,

    pub created_at: String,
    pub updated_at: String,
}


#[derive(Serialize, Deserialize)]
pub struct InternalShareRequest {
    pub shared_with_user_id: Uuid,
    pub folder_id: Uuid,

    pub can_preview: bool,
    pub can_download: bool,
    pub can_create: bool,
    pub can_update: bool,
    pub can_delete: bool,
}


#[derive(Deserialize)]
pub struct CreateExternalShareRequest {
    pub share_id: String,
    pub share_file_target_id: Option<Uuid>,
    pub share_folder_target_id: Option<Uuid>,

    pub can_preview: bool,
    pub can_download: bool,
    pub can_create: bool,
    pub can_update: bool,
    pub can_delete: bool,

    pub share_info: serde_json::Value,
    pub raw_password: Option<String>,
    pub phones_for_otp: Vec<String>,
    pub emails_for_otp: Vec<String>,
    pub expires_at: Option<ChronoUtc>,
}


#[derive(Deserialize)]
pub struct UpdateExternalShareRequest {
    pub can_preview: bool,
    pub can_download: bool,
    pub can_create: bool,
    pub can_update: bool,
    pub can_delete: bool,

    pub share_info: serde_json::Value,
    pub update_password_hash: bool,
    pub raw_password: Option<String>,
    pub phones_for_otp: Vec<String>,
    pub emails_for_otp: Vec<String>,
    pub expires_at: Option<ChronoUtc>,
}


#[derive(Serialize)]
pub struct ExternalShare {
    pub share_id: String,
    pub created_by: Uuid,
    pub organization_id: Uuid,
    pub share_file_target_id: Option<Uuid>,
    pub share_folder_target_id: Option<Uuid>,

    pub permission_set: PermissionSet,

    pub share_info: serde_json::Value,
    pub password_hash: Option<String>,
    pub phones_for_otp: Vec<String>,
    pub emails_for_otp: Vec<String>,
    pub expires_at: Option<ChronoUtc>,
    pub created_at: ChronoUtc,
}


// ------- Implementations ------- //


impl From<Row> for InternalSharedResource {
    fn from(row: Row) -> Self {
        InternalSharedResource {
            is_resource_folder: row.get("is_resource_folder"),

            user_id: row.get("user_id"),

            resource_id: row.get("resource_id"),
            resource_name: row.get("resource_name"),
            resource_info: row.get("resource_info"),

            total_resource_size: row.get("total_resource_size"),
            permission_set: PermissionSet {
                can_preview: row.get("can_preview"),
                can_download: row.get("can_download"),
                can_create: row.get("can_create"),
                can_update: row.get("can_update"),
                can_delete: row.get("can_delete"),
            },

            created_at: row.get::<_, ChronoUtc>("created_at").to_rfc3339(),
            updated_at: row.get::<_, ChronoUtc>("updated_at").to_rfc3339(),
        }
    }
}


impl InternalSharedResource {
    pub fn from_rows(rows: Vec<Row>) -> Vec<Self> {
        rows.into_iter().map(Self::from).collect()
    }
}


impl From<Row> for InternalShareRequest {
    fn from(row: Row) -> Self {
        InternalShareRequest {
            shared_with_user_id: row.get("shared_with_user_id"),
            folder_id: row.get("folder_id"),

            can_preview: row.get("can_preview"),
            can_download: row.get("can_download"),
            can_create: row.get("can_create"),
            can_update: row.get("can_update"),
            can_delete: row.get("can_delete"),
        }
    }
}


impl InternalShareRequest {
    pub fn from_rows(rows: Vec<Row>) -> Vec<Self> {
        rows.into_iter().map(Self::from).collect()
    }
}


impl From<Row> for ExternalShare {
    fn from(row: Row) -> Self {
        ExternalShare {
            share_id: row.get("share_id"),
            created_by: row.get("created_by"),
            organization_id: row.get("organization_id"),
            share_file_target_id: row.get("share_file_target_id"),
            share_folder_target_id: row.get("share_folder_target_id"),
            permission_set: PermissionSet {
                can_preview: row.get("can_preview"),
                can_download: row.get("can_download"),
                can_create: row.get("can_create"),
                can_update: row.get("can_update"),
                can_delete: row.get("can_delete"),
            },
            share_info: row.get("share_info"),
            password_hash: row.get("password_hash"),
            phones_for_otp: row.get("phones_for_otp"),
            emails_for_otp: row.get("emails_for_otp"),
            expires_at: row.get("expires_at"),
            created_at: row.get("created_at"),
        }
    }
}


impl ExternalShare {
    pub fn from_rows(rows: Vec<Row>) -> Vec<Self> {
        rows.into_iter().map(Self::from).collect()
    }

    pub fn has_restrictions(&self) -> bool {
        self.password_hash.is_some() || !self.phones_for_otp.is_empty() || !self.emails_for_otp.is_empty()
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    pub fn is_file_share(&self) -> bool {
        // Determine if the share is for a file based on the presence of a file target ID or folder target ID
        self.share_file_target_id.is_some() && self.share_folder_target_id.is_none()
    }
}
