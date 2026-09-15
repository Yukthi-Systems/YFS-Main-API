use crate::database::user::{get_user_by_id, search_user_by_email, update_user_session_fcm_token, update_user_public_info, update_user_private_info};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, get, patch, web};
use crate::models::errors::{ApiResponse, AppError};
use crate::models::user::SessionUser;
use crate::state::AppState;



#[patch("/update-fcm-token")]
pub async fn update_fcm_token(request: HttpRequest, fcm_token: web::Json<String>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Update the FCM token in the database
    update_user_session_fcm_token(&state.pg_pool, &session_user.user_id, &fcm_token).await?;

    Ok(HttpResponse::Ok().body("FCM token updated successfully"))
}


#[get("/user-by-id/{user_id}")]
pub async fn get_user_info_by_id(request: HttpRequest, user_id: web::Path<uuid::Uuid>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let user_info = get_user_by_id(&state.pg_pool, &user_id, &session_user.organization_id).await?;
    if user_info.is_none() {
        return Err(AppError::NotFound("User not found".into()));
    }
    let mut user_info = user_info.unwrap();

    // If its self, include private_info; otherwise, only public_info
    if user_info.user_id != session_user.user_id {
        user_info.private_info = serde_json::Value::Null;
    }

    Ok(HttpResponse::Ok().json(user_info))
}


#[get("/search/user-by-email/{email}")]
pub async fn dropdown_search_user_by_email(request: HttpRequest, email: web::Path<String>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let user_info = search_user_by_email(&state.pg_pool, &email, &session_user.organization_id).await?;

    Ok(HttpResponse::Ok().json(user_info))
}


#[patch("/update-user-info/{update_public_info}")]
pub async fn update_user_info(request: HttpRequest, update_public_info: web::Path<bool>, user_info: web::Json<serde_json::Value>, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    if update_public_info.into_inner() {
        update_user_public_info(&state.pg_pool, &session_user.user_id, &user_info).await?;
    } else {
        update_user_private_info(&state.pg_pool, &session_user.user_id, &user_info).await?;
    }

    Ok(HttpResponse::Ok().body("User info updated successfully"))
}
