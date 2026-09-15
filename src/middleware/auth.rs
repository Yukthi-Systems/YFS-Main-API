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
use crate::{
    models::user::SessionUser,
    utils::{
        AppCache,
        make_key
    }
};



// fn api_key_check(req: &ServiceRequest) -> bool {
//     const HEADER_NAME: &str = "x-api-key";
//     const API_KEY: &str = "Neko-Nik";

//     let header_check = req
//         .headers()
//         .get(HEADER_NAME)
//         .and_then(|hv| hv.to_str().ok())
//         .map(|val| val == API_KEY)
//         .unwrap_or(false);

//     header_check
// }


/// Check for valid session based on Session-ID cookie and x-csrf-token header
/// Inserts SessionUser into request extensions if valid
/// Returns true if valid, false otherwise
async fn session_check(req: &ServiceRequest) -> bool {
    // Look for Session-ID cookie and x-csrf-token header
    let session_id = req
        .cookie("Session-ID")
        .map(|c| c.value().to_string());
    let csrf_token = req
        .headers()
        .get("x-csrf-token")
        .and_then(|hv| hv.to_str().ok())
        .map(|s| s.to_string());

    // If either is missing, fail
    if session_id.is_none() || csrf_token.is_none() {
        return false;
    }

    // Check cache for session
    let cache = req.app_data::<web::Data<AppCache>>().unwrap();
    let key = make_key(session_id.unwrap());

    if let Some(user) = cache.get(&key).await {
        // Convert the JSON string back to SessionUser
        let user: SessionUser = serde_json::from_str(&user).unwrap();
        
        // Verify CSRF token
        if user.csrf_token != csrf_token.unwrap() {
            return false;
        }

        // Insert user into request extensions for further use
        req.extensions_mut().insert(user);

        return true;
    } else {
        return false;
    }
}


/// Authentication middleware
/// Checks for valid session and optionally API key
/// Short-circuits with 401 Unauthorized if checks fail
/// Otherwise calls the next service in the chain
pub async fn auth_check<B>(req: ServiceRequest, next: Next<B>) -> Result<ServiceResponse, Error>
    where B: MessageBody + 'static
{
    // If You need API-Key authentication, uncomment below
    // if !api_key_check(&req) {
    //     log::warn!("API key check failed for request {} {}", req.method(), req.path());

    //     // Short-circuit and return 401 Unauthorized
    //     let resp = HttpResponse::Unauthorized()
    //         .append_header(("content-type", "text/plain; charset=utf-8"))
    //         .body("Unauthorized: missing or invalid API key");

    //     // Convert into a ServiceResponse with a boxed body to satisfy types
    //     return Ok(req.into_response(resp).map_into_boxed_body());
    // }

    // See if session is valid
    if !session_check(&req).await {
        log::warn!("Session check failed for request {} {}", req.method(), req.path());

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
