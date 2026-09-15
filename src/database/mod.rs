use deadpool_postgres::{
    PoolError as PgError,
    Pool as PgPool
};

pub mod folders;
pub mod shares;
pub mod files;
pub mod user;


// DB working state Check
pub async fn health_check(db_pool: &PgPool) -> Result<(), PgError> {
    // Simple query to check if the database is responsive
    let client = db_pool.get().await?;
    let _ = client.query("SELECT 1", &[]).await?;
    Ok(())
}
