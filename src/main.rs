use actix_web::web::scope as actix_scope;
use routes::{health, sample_db, auth};
use actix_web::middleware::from_fn;
use actix_web::{App, HttpServer};
use std::env::var as env_var;
use actix_cors::Cors;

mod middleware;
mod database;
mod models;
mod routes;
mod state;
mod utils;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (pg_pool, in_mem_cache, tx) = state::initialize().await;

    // Start the Actix web server
    HttpServer::new(move || {
        App::new()
            .app_data(pg_pool.clone())
            .app_data(in_mem_cache.clone())
            .app_data(tx.clone())
            .wrap(Cors::default()
                .allow_any_origin()
                .allowed_methods(vec!["GET", "POST", "DELETE"])
                .allow_any_header()
                .max_age(60)
            )
            .service(
                actix_scope("/health")
                .service(health::api_health_check)
                .service(health::db_health_check)
                .service(health::cache_health_check)
                .service(health::channel_health_check)
            )
            .service(
                actix_scope("/sample_db")
                .wrap(from_fn(middleware::auth::auth_check))
                .service(sample_db::create_note_handler)
                .service(sample_db::list_notes_handler)
            )
            .service(
                actix_scope("/auth")
                .service(auth::create_session_handler)
                .service(
                    actix_scope("")
                    .wrap(from_fn(middleware::auth::auth_check))
                    .service(auth::delete_session_handler)
                    .service(auth::get_session_handler)
                )
            )
    })
    .bind(("0.0.0.0", 8686))?
    .workers(env_var("API_WORKERS_COUNT").unwrap_or("4".to_string()).parse().unwrap())
    .run().await
}
