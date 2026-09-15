use actix_web::{cookie::Cookie, delete, get, post, web, HttpResponse, Responder, HttpMessage, HttpRequest};
use crate::utils::{AppCache, make_key, cache_data};
use crate::models::user::SessionUser;



#[post("/session")]
pub async fn create_session_handler(
    user_name: String,         // The request body (For now accept anything)
    state: web::Data<AppCache>, // The state containing the Cache
) -> impl Responder {
    // Generate a new session ID
    let session = SessionUser::create(user_name);

    cache_data(&session.session_id, &session, &state).await;
    log::info!("Created new session: {}", session);

    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-CSRF-Token", session.csrf_token))
        .cookie(
            Cookie::build("Session-ID", &session.session_id)
                .path("/")
                .http_only(true)
                .finish(),
        )
        .body("Session created successfully!")
}


#[get("/session")]
pub async fn get_session_handler(request: HttpRequest) -> impl Responder {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    HttpResponse::Ok().body(format!("Hello, {}", session_user.user_name))
}


#[delete("/session")]
pub async fn delete_session_handler(request: HttpRequest, state: web::Data<AppCache>) -> impl Responder {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    let key = make_key(session_user.session_id.clone());
    state.remove(&key).await;

    let mut cookie = Cookie::build("Session-ID", "")
        .path("/")
        .http_only(true)
        .finish();

    cookie.make_removal();

    HttpResponse::Ok()
        .cookie(cookie)
        .body("Session deleted successfully!")
}
