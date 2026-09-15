use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use deadpool_postgres::PoolError;
use serde::Serialize;
use std::fmt;



#[derive(Debug)]
pub enum AppError {
    DbPool(PoolError),
    Pg(tokio_postgres::Error),
    // Unprocessable(String),
    // NotFound(String),
    // Conflict(String),
    // Gone(String),
}


#[derive(Serialize)]
struct ErrorResp {
    error: String
}


// ------- Implementations ------- //


impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DbPool(e) => write!(f, "DB: {}", e),
            AppError::Pg(e) => write!(f, "PostgreSQL: {}", e),
            // AppError::NotFound(s) => write!(f, "Resource not found: {}", s),
            // AppError::Conflict(s) => write!(f, "Conflict: {}", s),
            // AppError::Gone(s) => write!(f, "It's gone: {}", s),
            // AppError::Unprocessable(s) => write!(f, "Unprocessable: {}", s),
        }
    }
}


impl From<PoolError> for AppError {
    fn from(e: PoolError) -> Self {
        AppError::DbPool(e)
    }
}


impl From<tokio_postgres::Error> for AppError {
    fn from(e: tokio_postgres::Error) -> Self {
        AppError::Pg(e)
    }
}


impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::DbPool(_) => StatusCode::FAILED_DEPENDENCY,
            AppError::Pg(_) => StatusCode::EXPECTATION_FAILED,
            // AppError::NotFound(_) => StatusCode::NOT_FOUND,
            // AppError::Conflict(_) => StatusCode::CONFLICT,
            // AppError::Unprocessable(_) => StatusCode::UNPROCESSABLE_ENTITY,
            // AppError::Gone(_) => StatusCode::GONE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        log::error!("Error occurred: {}", self);

        HttpResponse::build(self.status_code())
            .json(ErrorResp { error: self.to_string() })
    }
}


impl std::error::Error for AppError {}
