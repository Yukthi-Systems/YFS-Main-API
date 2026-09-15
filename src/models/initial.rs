use std::env::var as env_var;
use std::time::Duration;


pub struct PgSettings {
    pub url: String,
    pub conn_timeout: u64,
    pub max_pool_size: usize,
    pub wait_timeout: u64,
    pub new_connection_timeout: u64,
    pub recycle_timeout: u64,
    pub warm_pool: bool,
    pub warm_pool_size: usize,
}


pub struct MokaSettings {
    pub cache_size: u64,
    pub expiration_time: Duration,
}


pub struct AppSettings {
    pub pg_settings: PgSettings,
    pub cache_settings: MokaSettings,
    pub enable_logging: bool,
}


// ------- Implementations ------- //


impl PgSettings {
    fn from_env() -> Self {
        let url = env_var("POSTGRES_DB_URL").expect("POSTGRES_DB_URL must be set");
        let conn_timeout = env_var("PG_CONN_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("PG_CONN_TIMEOUT must be a positive integer of type u64");
        let max_pool_size = env_var("PG_POOL_MAX_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("PG_POOL_MAX_SIZE must be a positive integer of type usize");
        let wait_timeout = env_var("PG_POOL_WAIT_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("PG_POOL_WAIT_TIMEOUT must be a positive integer of type u64");
        let new_connection_timeout = env_var("PG_POOL_NEW_CONNECTION_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("PG_POOL_NEW_CONNECTION_TIMEOUT must be a positive integer of type u64");
        let recycle_timeout = env_var("PG_POOL_RECYCLE_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("PG_POOL_RECYCLE_TIMEOUT must be a positive integer of type u64");
        let warm_pool = env_var("PG_POOL_WARM_POOL").expect("PG_POOL_WARM_POOL must be set as true or false");
        let warm_pool = match warm_pool.to_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => panic!("PG_POOL_WARM_POOL must be set as true or false"),
        };
        let warm_pool_size = env_var("PG_POOL_WARM_POOL_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("PG_POOL_WARM_POOL_SIZE must be a positive integer of type usize");

        // Warm pool size can not go above 128 (if warm pool is enabled)
        if warm_pool_size > max_pool_size {
            panic!("PG_POOL_WARM_POOL_SIZE must be at most PG_POOL_MAX_SIZE, it can not go more than {}", max_pool_size);
        }
        if warm_pool && warm_pool_size > 128 {
            panic!("PG_POOL_WARM_POOL_SIZE must be at most 128, and the optimal size is 64");
        }

        PgSettings {
            url,
            conn_timeout,
            max_pool_size,
            wait_timeout,
            new_connection_timeout,
            recycle_timeout,
            warm_pool,
            warm_pool_size,
        }
    }
}


impl MokaSettings {
    fn from_env() -> Self {
        let cache_size = env_var("CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("CACHE_SIZE must be a positive integer of type u64");
        let expiration_time = env_var("CACHE_EXPIRATION_TIME")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("CACHE_EXPIRATION_TIME must be a positive integer of type u64");

        MokaSettings {
            cache_size,
            expiration_time: Duration::from_secs(expiration_time),
        }
    }
}


impl AppSettings {
    pub fn from_env() -> Self {
        let enable_logging = env_var("ENABLE_LOGGING").expect("ENABLE_LOGGING must be set as true or false");
        let enable_logging = match enable_logging.to_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => panic!("ENABLE_LOGGING must be set as true or false"),
        };

        AppSettings {
            pg_settings: PgSettings::from_env(),
            cache_settings: MokaSettings::from_env(),
            enable_logging,
        }
    }
}
