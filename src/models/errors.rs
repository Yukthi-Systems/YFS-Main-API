use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use deadpool_postgres::PoolError;
use redis::RedisError;
use serde::Serialize;
use std::fmt;


pub type ApiResponse = Result<HttpResponse, AppError>;


#[derive(Debug)]
pub enum AppError {
    DbPool(PoolError),
    Pg(tokio_postgres::Error),
    Redis(RedisError),
    SerDe(serde_json::Error),
    Reqwest(reqwest::Error),

    Unauthorized(String),
    BadRequest(String),
    NotImplemented(String),
    Unprocessable(String),
    NotFound(String),
    Conflict(String),
    Gone(String),
}


#[derive(Serialize)]
struct ErrorResp {
    error: String
}


// ------- Implementations ------- //



fn pg_error(db_error: &tokio_postgres::Error, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if let Some(db_err) = db_error.as_db_error() {
        write!(
            f,
            "PostgreSQL error: {} | code={} | severity={} | detail={} | hint={} | table={} | column={} | constraint={}",
            db_err.message(),
            db_err.code().code(),
            db_err.severity(),
            db_err.detail().unwrap_or("-"),
            db_err.hint().unwrap_or("-"),
            db_err.table().unwrap_or("-"),
            db_err.column().unwrap_or("-"),
            db_err.constraint().unwrap_or("-"),
        )
    } else {
        write!(f, "PostgreSQL client error: {}", db_error)
    }
}


impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DbPool(e) => write!(f, "DB: {}", e),
            AppError::Pg(e) => pg_error(e, f),
            AppError::Redis(e) => write!(f, "Redis: {}", e),
            AppError::SerDe(e) => write!(f, "JSON: {}", e),
            AppError::Unauthorized(s) => write!(f, "Unauthorized: {}", s),
            AppError::Reqwest(e) => write!(f, "Reqwest: {}", e),
            AppError::BadRequest(s) => write!(f, "Bad Request: {}", s),
            AppError::NotImplemented(s) => write!(f, "Not Implemented: {}", s),
            AppError::NotFound(s) => write!(f, "Resource not found: {}", s),
            AppError::Conflict(s) => write!(f, "Conflict: {}", s),
            AppError::Gone(s) => write!(f, "It's gone: {}", s),
            AppError::Unprocessable(s) => write!(f, "Unprocessable: {}", s),
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


impl From<RedisError> for AppError {
    fn from(e: RedisError) -> Self {
        AppError::Redis(e)
    }
}


impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::SerDe(e)
    }
}


impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Reqwest(e)
    }
}


impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::DbPool(_) => StatusCode::FAILED_DEPENDENCY,
            AppError::Pg(_) => StatusCode::EXPECTATION_FAILED,
            AppError::Redis(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::SerDe(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Reqwest(_) => StatusCode::BAD_GATEWAY,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Unprocessable(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Gone(_) => StatusCode::GONE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        log::error!("Error occurred: {}", self);

        HttpResponse::build(self.status_code())
            .json(ErrorResp { error: self.to_string() })
    }
}


impl std::error::Error for AppError {}
