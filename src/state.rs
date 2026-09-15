use crate::models::initial::{AppSettings, MokaSettings, PgSettings};
use deadpool_postgres::{Manager, RecyclingMethod, Pool as PgPool};
use crate::utils::{process_channel, AppCache};
use deadpool::{managed::Timeouts, Runtime};
use actix_web::web::Data as webData;
use tokio_postgres::{Config, NoTls};
use std::sync::mpsc::Sender;
use std::time::Duration;
use log::{info, warn};



async fn warm_pool(pool: &PgPool, pg: &PgSettings) {
    // Warm pool to avoid first-hit latency
    if !pg.warm_pool {
        // Return early if warm pool is not enabled
        return;
    }

    let warm_n = pg.max_pool_size.min(pg.warm_pool_size);
    let mut ok = 0;

    for _ in 0..warm_n {
        match pool.get().await {
            Ok(client) => {
                let _ = client.simple_query("SELECT 1").await;
                ok += 1;
            }
            Err(_) => {
                warn!("Pool warm-up: failed to get a connection");
            }
        }
    }

    // Log the warm-up results
    if ok == 0 {
        warn!("Pool warm-up failed, all attempts to get a connection were unsuccessful: {warm_n}");
    } else {
        info!("Pool warm-up: {ok} conns warmed up out of {warm_n}. Success rate: {:.2}%", ok as f64 / warm_n as f64 * 100.0);
    }
}


fn build_pg_config(settings: &PgSettings) -> Config {
    // Initialize the Postgres configuration
    let mut cfg: Config = settings.url.parse::<Config>().expect("invalid POSTGRES_DB_URL");
    cfg.application_name("rust-api");
    cfg.connect_timeout(Duration::from_secs(settings.conn_timeout));

    cfg
}


fn init_pg_pool(pg_settings: &PgSettings) -> PgPool {
    // Get the Postgres base configuration
    let cfg: Config = build_pg_config(pg_settings);
    let mgr = Manager::from_config(
        cfg,
        NoTls,
        deadpool_postgres::ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        },
    );

    let pool = PgPool::builder(mgr)
        .max_size(pg_settings.max_pool_size)
        .runtime(Runtime::Tokio1)
        .timeouts(Timeouts {
            // how long to wait for an idle connection from the pool
            wait: Some(Duration::from_secs(pg_settings.wait_timeout)),
            // how long to spend creating a new connection (if pool can grow)
            create: Some(Duration::from_secs(pg_settings.new_connection_timeout)),
            // how long to spend recycling/validating a connection
            recycle: Some(Duration::from_secs(pg_settings.recycle_timeout)),
        })
        .build()
        .expect("failed to build pg pool");

    info!("Postgres pool initialized (max_pool_size={})", pg_settings.max_pool_size);
    pool
}


fn init_cache(cache_settings: &MokaSettings) -> AppCache {
    // Build the AppCache
    let cache: AppCache = AppCache::builder()
        .max_capacity(cache_settings.cache_size)
        .time_to_live(cache_settings.expiration_time)
        .build();

    info!("In-memory cache initialized (max_capacity={})", cache_settings.cache_size);
    cache
}


pub async fn initialize() -> (webData<PgPool>, webData<AppCache>, webData<Sender<u8>>) {
    // Preparing to start the server by collecting environment variables
    let app_settings: AppSettings = AppSettings::from_env();

    if app_settings.enable_logging {
        let _ = env_logger::try_init(); // Initialize the logger to log all the logs
        info!("Starting the server by initializing the application state");
    }

    // Initialize the Postgres client
    let postgres_state = init_pg_pool(&app_settings.pg_settings);

    // Warm up the connection pool if enabled
    warm_pool(&postgres_state, &app_settings.pg_settings).await;

    // Initialize the in-memory cache (Moka)
    let in_mem_cache = init_cache(&app_settings.cache_settings);

    // Initialize the channel
    let (tx, rx) = std::sync::mpsc::channel::<u8>();
    process_channel(rx);

    // Wrap the state of the application and share it
    (webData::new(postgres_state), webData::new(in_mem_cache), webData::new(tx))
}
