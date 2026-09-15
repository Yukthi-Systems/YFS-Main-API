use actix_web::{get, post, web, HttpRequest, HttpResponse, HttpMessage};
use crate::database::notes::{add_new_notes, fetch_all_notes};
use deadpool_postgres::Pool as PgPool;
use crate::models::{
    user::SessionUser,
    errors::AppError,
    notes::Notes,
};

type ApiResp = Result<HttpResponse, AppError>;



#[post("/create-note")]
pub async fn create_note_handler(request: HttpRequest, body: web::Json<Notes>, pg_pool: web::Data<PgPool>) -> ApiResp {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    log::trace!("{} is creating a new note.", session_user);

    add_new_notes(&pg_pool, vec![body.into_inner()]).await?;

    Ok(HttpResponse::Ok().json("Note created successfully!"))
}


#[get("/notes")]
pub async fn list_notes_handler(request: HttpRequest, pg_pool: web::Data<PgPool>) -> ApiResp {
    // Get SessionUser from request extensions
    let ext = request.extensions();
    let session_user = ext.get::<SessionUser>().unwrap();

    log::trace!("User '{}' is listing notes.", session_user.user_name);

    let notes = fetch_all_notes(&pg_pool).await?;

    Ok(HttpResponse::Ok().json(notes))
}
