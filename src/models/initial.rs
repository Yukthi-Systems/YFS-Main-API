use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use std::env::var as env_var;


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


pub struct RedisSettings {
    pub url: String,
}


#[derive(Clone)]
pub struct RmqSettings {
    pub domain: String,
    pub auth_token: String, // Base64 encoded string of "username:password"
    pub virtual_host: String,
    pub exchange_name: String,
    pub routing_key: String,
}


pub struct AppSettings {
    pub pg_settings: PgSettings,
    pub redis_settings: RedisSettings,
    pub rmq_settings: RmqSettings,
    pub enable_logging: bool,
}


pub struct ApiSettings {
    pub allowed_origins: Vec<String>,
    pub self_api_key: String,
    pub sso_api_url: String,
    pub sso_api_key: String,
    pub file_store_api_key: String, // Same for all file store hosts
    pub file_store_host: String,    // TODO: Temp. only we will later make this via SSO API (available servers per org)
    pub max_instant_download_size: i64,
    pub download_manager_api_url: String,
    pub download_manager_api_key: String,
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


impl RedisSettings {
    fn from_env() -> Self {
        let url = env_var("REDIS_URL").expect("REDIS_URL must be set");

        RedisSettings {
            url,
        }
    }
}


impl RmqSettings {
    pub fn from_env() -> Self {
        let domain = env_var("RABBITMQ_DOMAIN").expect("RABBITMQ_DOMAIN must be set");
        let user_name = env_var("RABBITMQ_USER_NAME").expect("RABBITMQ_USER_NAME must be set");
        let password = env_var("RABBITMQ_PASSWORD").expect("RABBITMQ_PASSWORD must be set");
        let virtual_host = env_var("RABBITMQ_VIRTUAL_HOST").expect("RABBITMQ_VIRTUAL_HOST must be set");
        let exchange_name = env_var("RABBITMQ_EXCHANGE_NAME").expect("RABBITMQ_EXCHANGE_NAME must be set");
        let routing_key = env_var("RABBITMQ_ROUTING_KEY").expect("RABBITMQ_ROUTING_KEY must be set");

        let auth_token = BASE64_STANDARD.encode(format!("{}:{}", user_name, password));

        RmqSettings {
            domain,
            auth_token,
            virtual_host,
            exchange_name,
            routing_key,
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
            redis_settings: RedisSettings::from_env(),
            rmq_settings: RmqSettings::from_env(),
            enable_logging,
        }
    }
}


impl ApiSettings {
    pub fn from_env() -> Self {
        let allowed_origins = env_var("ALLOWED_ORIGINS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|entry| entry.trim())
                    .filter(|entry| !entry.is_empty())
                    .map(|entry| entry.to_string())
                    .collect::<Vec<String>>()
            })
            .filter(|items| !items.is_empty())
            .expect("ALLOWED_ORIGINS must be set as a comma-separated list of allowed origins or '*' for allowing all origins");

        let sso_api_url = env_var("SSO_API_URL").expect("SSO_API_URL must be set");
        let sso_api_key = env_var("SSO_API_KEY").expect("SSO_API_KEY must be set");

        let self_api_key = env_var("SELF_API_KEY").expect("SELF_API_KEY must be set");

        let file_store_api_key = env_var("FILE_STORE_API_KEY").expect("FILE_STORE_API_KEY must be set");
        let file_store_host = env_var("FILE_STORE_HOST").expect("FILE_STORE_HOST must be set");

        let max_instant_download_size = env_var("MAX_INSTANT_DOWNLOAD_SIZE")
            .ok()
            .map(|s| s.parse::<i64>().expect("MAX_INSTANT_DOWNLOAD_SIZE must be a valid integer"))
            .expect("MAX_INSTANT_DOWNLOAD_SIZE must be set");

        // Ensure that the maximum instant download size is a positive integer
        if max_instant_download_size <= 0 {
            panic!("MAX_INSTANT_DOWNLOAD_SIZE must be a positive integer");
        }

        let download_manager_api_url = env_var("DOWNLOAD_MANAGER_API_URL").expect("DOWNLOAD_MANAGER_API_URL must be set");
        let download_manager_api_key = env_var("DOWNLOAD_MANAGER_API_KEY").expect("DOWNLOAD_MANAGER_API_KEY must be set");

        ApiSettings {
            allowed_origins,
            self_api_key,
            file_store_api_key,
            file_store_host,
            sso_api_url,
            sso_api_key,
            download_manager_api_url,
            download_manager_api_key,
            max_instant_download_size,
        }
    }
}
