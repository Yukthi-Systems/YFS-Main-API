use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fmt;



#[derive(Serialize, Deserialize)]
pub struct SessionUser {
    // Session ID and CSRF token is required for session validation
    pub session_id: String,
    pub csrf_token: String,

    // Additional user info
    pub user_name: String,
    // Add more fields as necessary
}


// ------- Implementations ------- //


impl fmt::Display for SessionUser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Session's User Name: {}", self.user_name)
    }
}


impl SessionUser {
    pub fn create(user_name: String) -> Self {
        SessionUser {
            user_name,
            csrf_token: Uuid::new_v4().to_string(),
            session_id: Uuid::new_v4().to_string(),
        }
    }
}
