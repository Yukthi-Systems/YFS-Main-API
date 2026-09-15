use std::sync::mpsc::Receiver;
use moka::future::Cache;
use std::sync::Arc;


// Cache key and value types
type Key = Arc<str>;
type Value = String;
pub type AppCache = Cache<Key, Value>;


/// Create a cache key from a string-like value
pub fn make_key<S>(s: S) -> Key
    where S: Into<Value>
{
    Arc::<str>::from(s.into())
}


/// Cache data into the cache
pub async fn cache_data<T, S>(key: S, value: &T, cache_conn: &AppCache)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
        S: Into<String>
{
    // Note: No need to spawn to add the cache, just call the function directly (Tested by Neko Nik)
    // There differences are tiny (~0.5–1% variation), so why do all the clone and stuff

    // Serialize the value to a JSON string
    let json = serde_json::to_string(value).unwrap();

    // Insert the JSON string into the cache
    cache_conn.insert(make_key(key), json).await;
}


/// Process the channel
pub fn process_channel(rx: Receiver<u8>) {
    std::thread::spawn(move || {
        // Keep reading
        loop {
            match rx.recv() {
                Ok(val) => {
                    log::info!("Received: {}", val);
                }
                Err(_) => {
                    log::warn!("Channel closed");
                    break;
                }
            }
        }
    });
}
