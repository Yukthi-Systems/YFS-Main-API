use crate::models::errors::AppError;
use serde::{Serialize, Deserialize};
use tokio_postgres::row::Row;
use uuid::Uuid;


type ChronoUtc = chrono::DateTime<chrono::Utc>;

// TODO: Call this from the SSO API later and add it to user session, not hardcoded here
const MAX_SINGLE_FILE_SIZE: u64 = 50 * 1024 * 1024 * 1024; // 50 GB


#[derive(Deserialize)]
pub enum FileOpsType {
    Upload,
    Download,
    Delete,
    Replace
}


#[derive(Deserialize)]
pub struct FileOpsRequest {
    pub folder_id: Uuid,
    pub file_id: Option<Uuid>,
    pub shared_folder_id: Option<Uuid>,

    pub file_name: String,
    pub file_info: serde_json::Value,
    pub file_type: String,  // TODO: Check if this is allowed/blocked by organization-level constraints
    pub file_version: i32,
    pub expected_file_size: u64,
}


#[derive(Deserialize)]
pub struct FileOpsCallBack {
    pub folder_id: Uuid,
    pub file_id: Uuid,
    pub owner_id: Uuid,
    pub file_version: i32,
    pub hosted_at: String,
    pub file_location: String,
    pub file_size: i64,
    pub metadata: serde_json::Value,
    pub file_hash: String,
}


#[derive(Serialize)]
pub struct BasicFileInfo {
    pub file_id: Uuid,
    pub folder_id: Uuid,
    pub user_id: Uuid,
    pub file_name: String,
    pub file_info: serde_json::Value,
    pub is_locked: bool,
    pub available_versions: Vec<i32>,
    pub created_at: ChronoUtc,
    pub updated_at: ChronoUtc,
}


#[derive(Serialize)]
pub struct FileLocation {
    pub file_location: String,
    pub hosted_at: String,
}


#[derive(Serialize)]
pub struct FileStorageAPI {
    pub file_location: String,
    pub hosted_at: String,
    pub file_name: String,
    pub folder_id: Uuid,
    pub file_id: Uuid,
    pub owner_id: Uuid,
    pub file_version: i32,
    pub expected_file_size: u64,
    pub max_operation_time: u16,
}


#[derive(Serialize)]
pub struct FileVersionInfo {
    pub file_version: i32,
    pub file_size: i64,
    pub hosted_at: String,
    pub file_location: String,
}


// ------- Implementations ------- //


impl From<Row> for BasicFileInfo {
    fn from(row: Row) -> Self {
        BasicFileInfo {
            file_id: row.get("file_id"),
            folder_id: row.get("folder_id"),
            user_id: row.get("user_id"),
            file_name: row.get("file_name"),
            file_info: row.get("file_info"),
            is_locked: row.get("is_locked"),
            available_versions: row.get("available_versions"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}


impl From<Row> for FileLocation {
    fn from(row: Row) -> Self {
        FileLocation {
            file_location: row.get("file_location"),
            hosted_at: row.get("hosted_at"),
        }
    }
}


impl From<Row> for FileVersionInfo {
    fn from(row: Row) -> Self {
        FileVersionInfo {
            file_version: row.get("file_version"),
            file_size: row.get("file_size"),
            hosted_at: row.get("hosted_at"),
            file_location: row.get("file_location"),
        }
    }
}


impl FileVersionInfo {
    pub fn from_rows(rows: Vec<Row>) -> Vec<Self> {
        rows.into_iter().map(Self::from).collect()
    }
}


impl FileOpsType {
    fn is_upload(&self) -> bool {
        matches!(self, FileOpsType::Upload)
    }

    fn is_delete(&self) -> bool {
        matches!(self, FileOpsType::Delete)
    }

    fn is_download(&self) -> bool {
        matches!(self, FileOpsType::Download)
    }

    fn is_replace(&self) -> bool {
        matches!(self, FileOpsType::Replace)
    }
}


impl FileOpsRequest {
    fn check_file_size(&self) -> Result<(), AppError> {
        if self.expected_file_size > MAX_SINGLE_FILE_SIZE {
            return Err(AppError::BadRequest(format!("File size {} exceeds the maximum allowed size of {} bytes", self.expected_file_size, MAX_SINGLE_FILE_SIZE)));
        }
        Ok(())
    }

    fn file_version_check(&self, operation_type: &FileOpsType, is_versioning_enabled: bool) -> Result<(), AppError> {
        if self.file_version < 1 {
            return Err(AppError::BadRequest("File version must be greater than or equal to 1".into()));
        }

        // File Replace should not work if versioning is enabled
        if operation_type.is_replace() && is_versioning_enabled {
            return Err(AppError::BadRequest("File replace operations are not allowed when versioning is enabled".into()));
        }

        if self.file_version > 1 && self.file_id.is_none() {
            return Err(AppError::BadRequest("File ID must be provided for file versions greater than 1".into()));
        }

        // If its first version and no file ID is provided
        // It can only do upload and without a file ID
        if self.file_version == 1 && self.file_id.is_none() && !operation_type.is_upload() {
            return Err(AppError::BadRequest("First version of the file must be uploaded without a file ID".into()));
        }

        // If its replace then the file ID must be provided
        if operation_type.is_replace() && self.file_id.is_none() {
            return Err(AppError::BadRequest("File ID must be provided for replace operations".into()));
        }

        // Replace operations should only be allowed for the first version
        if operation_type.is_replace() && self.file_version != 1 {
            return Err(AppError::BadRequest("Replace operations can only be performed on the first version".into()));
        }

        // Versioning should be enabled for file versions greater than 1
        if self.file_version > 1 && !is_versioning_enabled {
            // But its allowed if the file is being Deleted or Downloaded
            if !operation_type.is_delete() && !operation_type.is_download() {
                return Err(AppError::BadRequest("File versioning must be enabled for versions greater than 1".into()));
            }
        }

        Ok(())
    }

    pub fn validate(&self, operation_type: &FileOpsType, is_versioning_enabled: bool) -> Result<(), AppError> {
        // Validate the file size against the maximum allowed size
        self.check_file_size()?;

        // Validate the file version against the minimum allowed value
        self.file_version_check(operation_type, is_versioning_enabled)?;

        // If it is download the file ID must be provided
        if (operation_type.is_download()) && self.file_id.is_none() {
            return Err(AppError::BadRequest("File ID must be provided for download operations".into()));
        }

        // If it is delete the file ID must be provided
        if (operation_type.is_delete()) && self.file_id.is_none() {
            return Err(AppError::BadRequest("File ID must be provided for delete operations".into()));
        }

        Ok(())
    }


    pub fn validate_against_info(&self, operation_type: &FileOpsType, file_info: &BasicFileInfo) -> Result<(), AppError> {
        // If its locked, we do not allow any operations on it including downloads and deletions
        if file_info.is_locked {
            return Err(AppError::BadRequest("Current file is undergoing modifications and is locked".into()));
        }

        // If its download / delete / replace, check if the requested version exists in the file info
        if operation_type.is_download() || operation_type.is_delete() {
            if !file_info.available_versions.contains(&self.file_version) {
                return Err(AppError::BadRequest("Requested file version is not available".into()));
            }
        }

        // Replace should be only done if its the first version
        if operation_type.is_replace() {
            if self.file_version != 1 {
                return Err(AppError::BadRequest("Replace operations can only be performed on the first version".into()));
            }
        }

        // If its upload then it should be exactly +1 of the latest version
        if operation_type.is_upload() {
            let latest_version = file_info.available_versions.iter().max().cloned().unwrap_or(0);
            if self.file_version != latest_version + 1 {
                return Err(AppError::BadRequest("Uploaded file version must be exactly one greater than the latest version".into()));
            }
        }

        Ok(())
    }

    fn get_ttl(&self) -> u16 {
        // Get the file size in MegaBytes (1024 * 1024 = 1,048,576)
        let size_mb = self.expected_file_size as f64 / 1_048_576.0;

        // Square root of the file size in MB multiplied by 180 to get the TTL in seconds
        // It will max out at u16::MAX seconds (approximately 18.2 hours)
        (180.0 * size_mb.sqrt())
            .round()
            .clamp(7.0, u16::MAX as f64) as u16
    }

    pub fn generate_api_struct(&self, file_location: FileLocation, owner_id: Uuid, file_id: Uuid) -> FileStorageAPI {
        FileStorageAPI {
            file_location: file_location.file_location,
            hosted_at: file_location.hosted_at,
            file_name: self.file_name.clone(),
            file_id,
            owner_id,
            folder_id: self.folder_id,
            file_version: self.file_version,
            expected_file_size: self.expected_file_size,
            max_operation_time: self.get_ttl(),
        }
    }
}
