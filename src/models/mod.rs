pub mod initial;
pub mod folders;
pub mod shares;
pub mod errors;
pub mod files;
pub mod user;

use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize)]
pub struct PermissionSet {
    pub can_preview: bool,
    pub can_download: bool,
    pub can_create: bool,
    pub can_update: bool,
    pub can_delete: bool,
}


#[derive(Deserialize)]
pub struct PageQuery {
    // TODO: Have a check so that it does not exceed a maximum limit and crash the DB
    pub limit: i64,
    pub offset: i64,
}
