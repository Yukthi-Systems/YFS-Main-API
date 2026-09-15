use crate::{
    database::health_check as db_health_check_pgsql,
    models::errors::ApiResponse,
    state::AppState
};
use crate::cache::handler::get_redis_cache;
use actix_web::{get, web, HttpResponse};


// Health check endpoint
#[get("/api")]
async fn api_health_check(state: web::Data<AppState>) -> ApiResponse {
    // Check PostgreSQL health
    db_health_check_pgsql(&state.pg_pool).await?;

    // Check Redis health (It won't be present but a good way to check if the connection is healthy)
    let _: Option<String> = get_redis_cache(state.redis_cache.clone(), "health_check").await?;

    Ok(HttpResponse::Ok().body("API is healthy!"))
}
