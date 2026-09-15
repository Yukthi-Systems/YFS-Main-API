use crate::database::user::{check_user_session, create_user_session, delete_user_session, update_user_last_seen};
use crate::cache::handler::{set_redis_cache, delete_redis_cache, get_redis_cache};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, post, web};
use crate::models::user::{SessionUser, PublicSessionUser};
use crate::database::shares::get_external_share_by_id;
use crate::models::errors::{ApiResponse, AppError};
use crate::handlers::auth::verify_password_hash;
use crate::state::AppState;


#[derive(serde::Deserialize)]
struct TokensRequest {
    refresh_token: uuid::Uuid,
    access_token: uuid::Uuid,
    user_id: uuid::Uuid,
}


#[post("/login")]
async fn user_login(request: HttpRequest, state: web::Data<AppState>) -> ApiResponse {
    // Get the SSO Cookie from the request
    let sso_session_id_cookie = request.cookie("SSO-Session-ID").map(|c| c.value().to_string());
    if sso_session_id_cookie.is_none() {
        return Ok(HttpResponse::BadRequest().body("SSO Session undefined, please login to SSO first"));
    }
    let sso_session_id_cookie: String = sso_session_id_cookie.unwrap();

    let refresh_token = uuid::Uuid::new_v4();
    let access_token = uuid::Uuid::new_v4().to_string();

    // We create a new session for the user, by using the SSO session ID and the newly generated refresh token
    let session_user = SessionUser::new(&sso_session_id_cookie, &refresh_token).await?;

    // Create a PgSQL session entry (Will have Refresh Token and User Info) - Long lived (like 30 days or so)
    create_user_session(&state.pg_pool, &session_user).await?;

    // Create a Redis cache entry (Will have Access Token linked with Refresh Token and User Info too) - Short lived (like 3 Hrs or so)
    set_redis_cache(state.redis_cache.clone(), &access_token, &session_user, 60 * 60 * 3).await?;

    // Return a X-Sesson-Refresh-ID along with X-Sesson-Access-ID
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Refresh-ID-Token", refresh_token.to_string()))
        .insert_header(("X-Session-Expiry", (60 * 60 * 3).to_string())) // Inform client about access token expiry time in seconds
        .insert_header(("Access-Control-Expose-Headers", "X-Refresh-ID-Token, X-Session-Expiry"))
        .json(serde_json::json!({
            "access_token": access_token,
            "user_info": {
                "email": session_user.email,
                "user_id": session_user.user_id,
                "domain_name": session_user.domain_name,
                "organization_id": session_user.organization_id,
                "organization_name": session_user.organization_name,
                "is_file_versioning_enabled": session_user.is_file_versioning_enabled,
                "is_sharing_enabled": session_user.is_sharing_enabled,
                "quota_allocated": session_user.quota_allocated,
                "quota_utilized": session_user.quota_utilized
            }
        })))
}


#[get("/session")]
pub async fn get_session(request: HttpRequest, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Check session, if it is no longer valid, delete it from both PgSQL and Redis
    if !check_user_session(&state.pg_pool, &session_user.refresh_token, &session_user.sso_token, &session_user.user_id).await? {
        // Delete the old session from the database as it is no longer valid
        delete_user_session(&state.pg_pool, &session_user.user_id, &session_user.refresh_token).await?;

        // Look for X-Session-Access-Token-ID
        let session_access_token = request
            .headers()
            .get("x-session-access-id")
            .and_then(|hv| hv.to_str().ok())
            .map(|s| s.to_string()).unwrap();

        // Delete the Access Token from Redis cache to invalidate the Short lived session
        delete_redis_cache(state.redis_cache.clone(), &session_access_token).await?;

        return Ok(HttpResponse::Unauthorized().body("Invalid session"));
    }

    // Remove refresh token from the response for security reasons
    let response_user_info = serde_json::json!({
        "email": session_user.email,
        "user_id": session_user.user_id,
        "domain_name": session_user.domain_name,
        "organization_id": session_user.organization_id,
        "organization_name": session_user.organization_name,
        "is_file_versioning_enabled": session_user.is_file_versioning_enabled,
        "is_sharing_enabled": session_user.is_sharing_enabled,

        // TODO: Check the quota live and not rely solely on cached values
        "quota_allocated": session_user.quota_allocated,
        "quota_utilized": session_user.quota_utilized
    });

    // Update the user's last seen timestamp in the database
    update_user_last_seen(&state.pg_pool, &session_user.user_id).await?;

    Ok(HttpResponse::Ok().json(response_user_info))
}


#[post("/refresh")]
pub async fn refresh_session(request: HttpRequest, user_tokens: web::Json<TokensRequest>, state: web::Data<AppState>) -> ApiResponse {
    // Get the SSO Cookie from the request
    let sso_session_id_cookie = request.cookie("SSO-Session-ID").map(|c| c.value().to_string());
    if sso_session_id_cookie.is_none() {
        return Ok(HttpResponse::BadRequest().body("SSO Session undefined, please login to SSO first"));
    }
    let sso_session_id_cookie: String = sso_session_id_cookie.unwrap();

    // Check if the SSO session is valid before proceeding with the refresh
    if !check_user_session(&state.pg_pool, &user_tokens.refresh_token, &sso_session_id_cookie, &user_tokens.user_id).await? {
        return Ok(HttpResponse::Unauthorized().body("Invalid SSO session"));
    }

    // Check if the SSO is valid or not (By using the same NewUserLogin)
    let new_session = SessionUser::new(&sso_session_id_cookie, &user_tokens.refresh_token).await?;

    // Delete the old Redis cache entry (if exists) with the old Access Token
    delete_redis_cache(state.redis_cache.clone(), &user_tokens.access_token.to_string()).await?;

    // Generate a new Access Token for the session
    let new_access_token = uuid::Uuid::new_v4().to_string();

    // Create a new Redis cache entry with the new Access Token and same user info, linked with the new Refresh Token
    set_redis_cache(state.redis_cache.clone(), &new_access_token, &new_session, 60 * 60 * 3).await?;

    // Update the user's last seen timestamp in the database
    update_user_last_seen(&state.pg_pool, &new_session.user_id).await?;

    // Return the new Access Token along with some basic user info
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Session-Expiry", (60 * 60 * 3).to_string())) // Inform client about access token expiry time in seconds
        .insert_header(("Access-Control-Expose-Headers", "X-Session-Expiry"))
        .json(serde_json::json!({
            "access_token": new_access_token,
            "user_info": {
                "email": new_session.email,
                "user_id": new_session.user_id,
                "domain_name": new_session.domain_name,
                "organization_id": new_session.organization_id,
                "organization_name": new_session.organization_name,
                "is_file_versioning_enabled": new_session.is_file_versioning_enabled,
                "is_sharing_enabled": new_session.is_sharing_enabled,
                "quota_allocated": new_session.quota_allocated,
                "quota_utilized": new_session.quota_utilized
            }
        }))
    )
}


#[delete("/logout")]
pub async fn user_logout(request: HttpRequest, state: web::Data<AppState>) -> ApiResponse {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    // Look for X-Session-Access-Token-ID
    let session_access_token = request
        .headers()
        .get("x-session-access-id")
        .and_then(|hv| hv.to_str().ok())
        .map(|s| s.to_string()).unwrap();

    // Check and delete the Refresh Token from PgSQL DB to invalidate the Long lived session
    delete_user_session(&state.pg_pool, &session_user.user_id, &session_user.refresh_token).await?;

    // Delete the Access Token from Redis cache to invalidate the Short lived session
    delete_redis_cache(state.redis_cache.clone(), &session_access_token).await?;

    // Update the user's last seen timestamp in the database
    update_user_last_seen(&state.pg_pool, &session_user.user_id).await?;

    Ok(HttpResponse::Ok().body("Logged out successfully"))
}


#[post("/session/{share_id}")]
pub async fn create_public_session(share_id: web::Path<String>, state: web::Data<AppState>) -> ApiResponse {
    // Fetch the share details from the database using the share_id
    let share_details = get_external_share_by_id(&state.pg_pool, &share_id).await?;
    if share_details.is_none() {
        return Err(AppError::BadRequest("The external share does not exist".into()));
    }
    let share_details = share_details.unwrap();

    // At this point, `share_details` contains the valid external share information.
    // See if the share has expired
    if share_details.is_expired() {
        return Err(AppError::BadRequest("The external share has expired".into()));
    }

    // Check if the share has any restrictions (e.g., password, OTP)
    let has_restrictions = share_details.has_restrictions();
    let share_info = share_details.share_info.clone();
    let expires_at = share_details.expires_at.clone();
    let is_password_protected = share_details.password_hash.is_some();
    let is_email_otp_protected = share_details.emails_for_otp.len() > 0;
    let is_phone_otp_protected = share_details.phones_for_otp.len() > 0;
    let is_file_share = share_details.is_file_share();

    // Create a new public session
    let public_session = PublicSessionUser::new(share_details, !has_restrictions);

    // Create a Redis cache entry (Will have Access Token linked with Refresh Token and User Info too) - Short lived (like 3 Hrs or so)
    set_redis_cache(state.redis_cache.clone(), &public_session.cache_key(), &public_session, 60 * 60 * 3).await?;

    // Return a X-Sesson-Refresh-ID along with X-Sesson-Access-ID
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Session-Expiry", (60 * 60 * 3).to_string())) // Inform client about access token expiry time in seconds
        .insert_header(("Access-Control-Expose-Headers", "X-Session-Expiry"))
        .json(serde_json::json!({
            "public_session_token": public_session.access_token,
            "share_info": share_info,
            "is_password_protected": is_password_protected,
            "is_email_otp_protected": is_email_otp_protected,
            "is_phone_otp_protected": is_phone_otp_protected,
            "is_file_share": is_file_share, // If its of type file rather than folder
            "expires_at": expires_at,
        })))
}


#[post("/validate/password")]
pub async fn public_session_validate_password(request: HttpRequest, raw_password: web::Json<String>, state: web::Data<AppState>) -> ApiResponse {
    // Fetch the public session from the request header
    let public_session_id = request
        .headers()
        .get("x-public-session-id")
        .and_then(|hv| hv.to_str().ok())
        .map(|s| s.to_string());

    if public_session_id.is_none() {
        return Err(AppError::BadRequest("Missing public session ID".into()));
    }

    let public_session_id = public_session_id.unwrap();
    let session_cache_key = format!("public:{}", public_session_id);

    // Get the share_id from the public session cache
    let public_session: Option<PublicSessionUser> = get_redis_cache(state.redis_cache.clone(), &session_cache_key).await?;
    if public_session.is_none() {
        return Err(AppError::BadRequest("The public session does not exist or has expired".into()));
    }
    let public_session = public_session.unwrap();

    // Fetch the share details from the database using the share_id
    let share_details  = get_external_share_by_id(&state.pg_pool, &public_session.share_id).await?;
    if share_details.is_none() {
        return Err(AppError::BadRequest("The external share does not exist".into()));
    }
    let share_details = share_details.unwrap();
    let password_hash = share_details.password_hash.clone();

    // Ensure the external share is still valid and has restrictions
    if share_details.is_expired() || !share_details.has_restrictions() || password_hash.is_none() {
        return Err(AppError::BadRequest("The external share is either expired or does not have restrictions or does not have a password set".into()));
    }

    // Validate the provided password against the share details
    if !verify_password_hash(&raw_password, &password_hash.unwrap()) {
        return Err(AppError::BadRequest("Invalid password".into()));
    }

    // At this point, the password has been validated successfully
    // We remove the old session from the cache as the password has been validated successfully
    delete_redis_cache(state.redis_cache.clone(), &session_cache_key).await?;

    // Create a new valid public session for the public session user
    let new_session = PublicSessionUser::new(share_details, true);

    // Store the new session in the cache with a TTL of 3 hours
    set_redis_cache(state.redis_cache.clone(), &session_cache_key, &new_session, 60 * 60 * 3).await?;

    Ok(HttpResponse::Ok().finish())
}


#[get("/session")]
pub async fn get_public_session(request: HttpRequest) -> ApiResponse {
    // Get PublicSessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<PublicSessionUser>().unwrap();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "share_id": session_user.share_id,
        "created_by": session_user.created_by,
        "share_folder_target_id": session_user.share_folder_target_id,
        "share_file_target_id": session_user.share_file_target_id,
        "permission_set": session_user.permission_set,
    })))
}


#[delete("/logout")]
pub async fn public_user_logout(request: HttpRequest, state: web::Data<AppState>) -> ApiResponse {
    // Get PublicSessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<PublicSessionUser>().unwrap();

    // Delete the public session from the cache
    let session_cache_key = format!("public:{}", session_user.access_token);
    delete_redis_cache(state.redis_cache.clone(), &session_cache_key).await?;

    Ok(HttpResponse::Ok().finish())
}
