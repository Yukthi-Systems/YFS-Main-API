use actix_web::{
    dev::{
        ServiceRequest,
        ServiceResponse
    },
    body::MessageBody,
    middleware::Next,
    HttpResponse,
    HttpMessage,
    Error,
    web,
};
use crate::models::user::{SessionUser, PublicSessionUser};
use crate::cache::handler::get_redis_cache;
use crate::state::{AppState, API_SETTINGS};


/// Check for valid session based on x-session-access-id header
/// Inserts SessionUser into request extensions if valid
/// Returns true if valid, false otherwise
async fn session_check(req: &ServiceRequest) -> bool {
    // Look for X-Session-Access-Token-ID
    let session_access_token = req
        .headers()
        .get("x-session-access-id")
        .and_then(|hv| hv.to_str().ok())
        .map(|s| s.to_string());

    // If no session access token, return false
    if session_access_token.is_none() {
        return false;
    }

    // Check cache for session
    let state = req.app_data::<web::Data<AppState>>().unwrap();
    let access_token = session_access_token.unwrap();

    // See if the access token exists in Redis cache and get the associated SessionUser
    let session_user: Option<SessionUser> = get_redis_cache(state.redis_cache.clone(), &access_token).await.unwrap();
    if session_user.is_none() {
        return false;
    }

    // Insert user into request extensions for further use
    req.extensions_mut().insert(session_user.unwrap());
    return true;
}


/// Authentication middleware
/// Short-circuits with 401 Unauthorized if checks fail
/// Otherwise calls the next service in the chain
pub async fn auth_check<B>(req: ServiceRequest, next: Next<B>) -> Result<ServiceResponse, Error>
    where B: MessageBody + 'static
{
    // See if session is valid
    if !session_check(&req).await {
        // Short-circuit and return 401 Unauthorized
        let resp = HttpResponse::Unauthorized()
            .append_header(("content-type", "text/plain; charset=utf-8"))
            .body("Unauthorized: invalid session");

        // Convert into a ServiceResponse with a boxed body to satisfy types
        return Ok(req.into_response(resp).map_into_boxed_body());
    }

    // authorized -> call the next service
    let res = next.call(req).await?;
    Ok(res.map_into_boxed_body())
}


/// Check for valid session based on x-public-session-id header
/// Inserts PublicSessionUser into request extensions if valid
/// Returns true if valid, false otherwise
async fn public_session_check(req: &ServiceRequest) -> bool {
    // Look for X-Session-Access-Token-ID
    let session_access_token = req
        .headers()
        .get("x-public-session-id")
        .and_then(|hv| hv.to_str().ok())
        .map(|s| s.to_string());

    // If no session access token, return false
    if session_access_token.is_none() {
        return false;
    }

    // Check cache for session
    let state = req.app_data::<web::Data<AppState>>().unwrap();
    let access_token = format!("public:{}", session_access_token.unwrap());

    // See if the access token exists in Redis cache and get the associated PublicSessionUser
    let session_user: Option<PublicSessionUser> = get_redis_cache(state.redis_cache.clone(), &access_token).await.unwrap();
    if session_user.is_none() {
        return false;
    }
    let session_user = session_user.unwrap();

    // Make sure that the session is active
    if !session_user.is_session_active {
        return false;
    }

    // Insert user into request extensions for further use
    req.extensions_mut().insert(session_user);
    return true;
}


/// Authentication middleware
/// Short-circuits with 401 Unauthorized if checks fail
/// Otherwise calls the next service in the chain
pub async fn public_auth_check<B>(req: ServiceRequest, next: Next<B>) -> Result<ServiceResponse, Error>
    where B: MessageBody + 'static
{
    // See if session is valid
    if !public_session_check(&req).await {
        // Short-circuit and return 401 Unauthorized
        let resp = HttpResponse::Unauthorized()
            .append_header(("content-type", "text/plain; charset=utf-8"))
            .body("Unauthorized: invalid public session");

        // Convert into a ServiceResponse with a boxed body to satisfy types
        return Ok(req.into_response(resp).map_into_boxed_body());
    }

    // authorized -> call the next service
    let res = next.call(req).await?;
    Ok(res.map_into_boxed_body())
}


/// API Key header check middleware for internal API routes (Normal Internal API)
pub async fn api_key_check<B>(req: ServiceRequest, next: Next<B>) -> Result<ServiceResponse, Error>
    where B: MessageBody + 'static
{
    // Check for x-api-key header
    let api_key = req.headers().get("x-api-key").and_then(|h| h.to_str().ok());

    if api_key != Some(&API_SETTINGS.self_api_key) {
        log::warn!("API key check failed for request {} {}", req.method(), req.path());

        // Short-circuit and return 401 Unauthorized
        let resp = HttpResponse::Unauthorized()
            .append_header(("content-type", "text/plain; charset=utf-8"))
            .body("Unauthorized: Invalid API key");

        // Convert into a ServiceResponse with a boxed body to satisfy types
        return Ok(req.into_response(resp).map_into_boxed_body());
    }

    // authorized -> call the next service
    let res = next.call(req).await?;
    Ok(res.map_into_boxed_body())
}
