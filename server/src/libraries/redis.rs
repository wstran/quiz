use std::fmt;
use redis::{Client, RedisResult, Connection, cmd, Pipeline, PubSub};
use redis::cluster::{ClusterClient, ClusterConnection};
use tokio::sync::OnceCell;
use crate::env::REDIS_URI;

#[derive(Clone)]
pub enum RedisConnection {
    Single(Client),
    Cluster(ClusterClient),
}

impl fmt::Debug for RedisConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RedisConnection::Single(client) => write!(f, "RedisConnection::Single({:?})", client),
            RedisConnection::Cluster(_) => write!(f, "RedisConnection::Cluster(ClusterClient)"),
        }
    }
}

pub static REDIS: OnceCell<RedisConnection> = OnceCell::const_new();

pub async fn init_redis() {
    let connection = if REDIS_URI.clone().contains("cluster") {
        let client = ClusterClient::new(vec![REDIS_URI.clone()])
            .expect("Failed to initialize Redis Cluster client");
        RedisConnection::Cluster(client)
    } else {
        let client = Client::open(REDIS_URI.clone())
            .expect("Failed to initialize Redis client");
        RedisConnection::Single(client)
    };
    REDIS.set(connection).expect("Redis already initialized");
}

pub fn get_redis() -> &'static RedisConnection {
    REDIS.get().expect("Redis not initialized")
}

pub enum RedisConn {
    Single(Connection),
    Cluster(ClusterConnection),
}

impl RedisConn {
    pub fn get_connection() -> RedisResult<Self> {
        match get_redis() {
            RedisConnection::Single(client) => {
                let conn = client.get_connection()?;
                Ok(RedisConn::Single(conn))
            }
            RedisConnection::Cluster(client) => {
                let conn = client.get_connection()?;
                Ok(RedisConn::Cluster(conn))
            }
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> RedisResult<()> {
        match self {
            RedisConn::Single(conn) => cmd("SET").arg(key).arg(value).query(conn),
            RedisConn::Cluster(conn) => cmd("SET").arg(key).arg(value).query(conn),
        }
    }

    pub fn set_ex(&mut self, key: &str, value: &str, seconds: u64) -> RedisResult<()> {
        match self {
            RedisConn::Single(conn) => cmd("SETEX").arg(key).arg(seconds).arg(value).query(conn),
            RedisConn::Cluster(conn) => cmd("SETEX").arg(key).arg(seconds).arg(value).query(conn),
        }
    }

    pub fn get(&mut self, key: &str) -> RedisResult<Option<String>> {
        match self {
            RedisConn::Single(conn) => cmd("GET").arg(key).query(conn),
            RedisConn::Cluster(conn) => cmd("GET").arg(key).query(conn),
        }
    }

    pub fn del(&mut self, key: &str) -> RedisResult<i64> {
        match self {
            RedisConn::Single(conn) => cmd("DEL").arg(key).query(conn),
            RedisConn::Cluster(conn) => cmd("DEL").arg(key).query(conn),
        }
    }

    pub fn exists(&mut self, key: &str) -> RedisResult<bool> {
        match self {
            RedisConn::Single(conn) => cmd("EXISTS").arg(key).query(conn),
            RedisConn::Cluster(conn) => cmd("EXISTS").arg(key).query(conn),
        }
    }

    pub fn set_nx(&mut self, key: &str, value: &str) -> RedisResult<bool> {
        match self {
            RedisConn::Single(conn) => cmd("SETNX").arg(key).arg(value).query(conn),
            RedisConn::Cluster(conn) => cmd("SETNX").arg(key).arg(value).query(conn),
        }
    }

    pub fn expire(&mut self, key: &str, seconds: i64) -> RedisResult<()> {
        match self {
            RedisConn::Single(conn) => cmd("EXPIRE").arg(key).arg(seconds).query(conn),
            RedisConn::Cluster(conn) => cmd("EXPIRE").arg(key).arg(seconds).query(conn),
        }
    }

    pub fn incr(&mut self, key: &str, increment: i64) -> RedisResult<i64> {
        match self {
            RedisConn::Single(conn) => cmd("INCRBY").arg(key).arg(increment).query(conn),
            RedisConn::Cluster(conn) => cmd("INCRBY").arg(key).arg(increment).query(conn),
        }
    }

    pub fn publish(&mut self, channel: &str, message: &str) -> RedisResult<i32> {
        match self {
            RedisConn::Single(conn) => cmd("PUBLISH").arg(channel).arg(message).query(conn),
            RedisConn::Cluster(conn) => cmd("PUBLISH").arg(channel).arg(message).query(conn),
        }
    }

    pub fn pubsub(&mut self) -> RedisResult<PubSub> {
        match self {
            RedisConn::Single(conn) => Ok(conn.as_pubsub()),
            RedisConn::Cluster(_) => Err(redis::RedisError::from((
                redis::ErrorKind::InvalidClientConfig,
                "Pub/Sub not supported in cluster mode with this implementation",
            ))),
        }
    }
}

pub fn with_transaction<F, T>(func: F) -> RedisResult<T>
where
    F: FnOnce(&mut Pipeline) -> RedisResult<()>,
    T: redis::FromRedisValue,
{
    let mut conn = RedisConn::get_connection()?;
    let mut pipe = redis::pipe();
    pipe.atomic();
    func(&mut pipe)?;
    match conn {
        RedisConn::Single(ref mut c) => pipe.query(c),
        RedisConn::Cluster(ref mut c) => pipe.query(c),
    }
}