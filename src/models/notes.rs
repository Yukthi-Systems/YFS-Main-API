use serde::{Deserialize, Serialize};
use tokio_postgres::row::Row;
use std::fmt;



#[derive(Serialize, Deserialize)]
pub struct Notes {
    pub id: Option<i32>,    // If user wants to create we can use same struct
    pub title: String,
    pub content: String,
}


// ------- Implementations ------- //


impl fmt::Display for Notes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Note Title: {}, Content: {}", self.title, self.content)
    }
}


impl From<Row> for Notes {
    fn from(row: Row) -> Self {
        // Use explicit type for id to handle SQL NULLs
        let id: Option<i32> = row.get::<_, Option<i32>>("id");
        let title: String = row.get("title");
        let content: String = row.get("content");

        Notes { id, title, content }
    }
}


impl Notes {
    pub fn from_rows(rows: Vec<Row>) -> Vec<Self> {
        rows.into_iter().map(Notes::from).collect()
    }
}
