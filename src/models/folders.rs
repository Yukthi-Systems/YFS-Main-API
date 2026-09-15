use serde::{Serialize, Deserialize};
use tokio_postgres::row::Row;
use uuid::Uuid;


type ChronoUtc = chrono::DateTime<chrono::Utc>;


#[derive(Serialize)]
pub struct Resource {
    pub is_resource_folder: bool,
    pub parent_folder_id: Option<Uuid>,

    pub resource_id: Uuid,
    pub resource_name: String,
    pub resource_info: serde_json::Value,

    pub total_resource_size: i64,

    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}


#[derive(Deserialize)]
pub struct NewFolderRequest {
    pub parent_folder_id: Option<Uuid>,
    pub shared_folder_id: Option<Uuid>,
    pub folder_name: String,
    pub folder_info: serde_json::Value,
}


#[derive(Deserialize)]
pub struct EditFolderRequest {
    pub folder_id: Uuid,
    pub shared_folder_id: Option<Uuid>,
    pub folder_name: String,
    pub folder_info: serde_json::Value,
}


#[derive(Deserialize)]
pub struct MoveFolderRequest {
    pub folder_id: Uuid,
    pub new_parent_folder_id: Option<Uuid>,
    pub shared_folder_id: Option<Uuid>,
}


// ------- Implementations ------- //


impl From<Row> for Resource {
    fn from(row: Row) -> Self {
        Resource {
            is_resource_folder: row.get("is_resource_folder"),
            parent_folder_id: row.get("parent_folder_id"),

            resource_id: row.get("resource_id"),
            resource_name: row.get("resource_name"),
            resource_info: row.get("resource_info"),

            total_resource_size: row.get("total_resource_size"),

            created_at: row.get::<_, ChronoUtc>("created_at").to_rfc3339(),
            updated_at: row.get::<_, ChronoUtc>("updated_at").to_rfc3339(),
            deleted_at: row.get::<_, Option<ChronoUtc>>("deleted_at").map(|dt| dt.to_rfc3339()),
        }
    }
}


impl Resource {
    pub fn from_rows(rows: Vec<Row>) -> Vec<Self> {
        rows.into_iter().map(Self::from).collect()
    }
}
