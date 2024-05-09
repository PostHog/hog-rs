use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use redis::{AsyncCommands, RedisError};
use tokio::time::timeout;

// average for all commands is <10ms, check grafana
const REDIS_TIMEOUT_MILLISECS: u64 = 10;

/// A simple redis wrapper
/// Copied from capture/src/redis.rs.
/// TODO: Modify this to support hincrby

#[async_trait]
pub trait Client {
    // A very simplified wrapper, but works for our usage
    async fn zrangebyscore(&self, k: String, min: String, max: String) -> Result<Vec<String>>;

    async fn get(&self, k: String) -> Result<String>;
    async fn set(&self, k: String, v: String) -> Result<()>;
}

pub struct RedisClient {
    client: redis::Client,
}

impl RedisClient {
    pub fn new(addr: String) -> Result<RedisClient> {
        let client = redis::Client::open(addr)?;

        Ok(RedisClient { client })
    }
}

#[async_trait]
impl Client for RedisClient {
    async fn zrangebyscore(&self, k: String, min: String, max: String) -> Result<Vec<String>> {
        let mut conn = self.client.get_async_connection().await?;

        let results = conn.zrangebyscore(k, min, max);
        let fut = timeout(Duration::from_secs(REDIS_TIMEOUT_MILLISECS), results).await?;

        Ok(fut?)
    }

    async fn get(&self, k: String) -> Result<String> {
        let mut conn = self.client.get_async_connection().await?;

        let results = conn.get(k.clone());
        // TODO: Is this safe? Should we be doing something else for error handling here?
        let fut: Result<Vec<u8>, RedisError> =
            timeout(Duration::from_secs(REDIS_TIMEOUT_MILLISECS), results).await?;

        // TRICKY: We serialise data to json, then django pickles it.
        // Here we deserialize the bytes using serde_pickle, to get the json string.
        let string_response: String = serde_pickle::from_slice(&fut?, Default::default())?;

        Ok(string_response)
    }

    async fn set(&self, k: String, v: String) -> Result<()> {
        // TRICKY: We serialise data to json, then django pickles it.
        // Here we serialize the json string to bytes using serde_pickle.
        let bytes = serde_pickle::to_vec(&v, Default::default())?;

        let mut conn = self.client.get_async_connection().await?;

        let results = conn.set(k, bytes);
        let fut = timeout(Duration::from_secs(REDIS_TIMEOUT_MILLISECS), results).await?;

        Ok(fut?)
    }
}
