use redis::aio::MultiplexedConnection;
use redis::cluster::ClusterClient;
use redis::cluster_async::ClusterConnection;
use redis::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum RedisConnection {
    Standalone(MultiplexedConnection),
    Cluster(ClusterConnection),
}

pub struct ConnectionEntry {
    pub connection: RedisConnection,
    pub url: String,
    pub mode: String,
    pub name: String,
}

pub struct RedisState {
    pub connections: Arc<Mutex<HashMap<String, ConnectionEntry>>>,
    pub active_id: Arc<Mutex<Option<String>>>,
}

impl RedisState {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            active_id: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self, id: &str, name: &str, url: &str, password: Option<&str>) -> Result<String, String> {
        let urls: Vec<&str> = url.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let is_cluster = urls.len() > 1;

        let (conn, mode) = if is_cluster {
            let connection_urls: Vec<String> = urls.iter().map(|u| build_redis_url(u, password)).collect();
            let url_refs: Vec<&str> = connection_urls.iter().map(|s| s.as_str()).collect();
            let client = ClusterClient::new(url_refs).map_err(|e| format!("Failed to create cluster client: {}", e))?;
            let con = client.get_async_connection().await.map_err(|e| format!("Failed to connect to cluster: {}", e))?;
            (RedisConnection::Cluster(con), "cluster".to_string())
        } else {
            let connection_url = build_redis_url(urls[0], password);
            let client = Client::open(connection_url.as_str()).map_err(|e| format!("Failed to create client: {}", e))?;
            let con = client.get_multiplexed_async_connection().await.map_err(|e| format!("Failed to connect: {}", e))?;
            (RedisConnection::Standalone(con), "standalone".to_string())
        };

        let entry = ConnectionEntry {
            connection: conn,
            url: url.to_string(),
            mode: mode.clone(),
            name: name.to_string(),
        };

        let mut conns = self.connections.lock().await;
        conns.insert(id.to_string(), entry);
        let mut active = self.active_id.lock().await;
        *active = Some(id.to_string());

        Ok(mode)
    }

    pub async fn disconnect(&self, id: &str) -> Result<(), String> {
        let mut conns = self.connections.lock().await;
        conns.remove(id);
        let mut active = self.active_id.lock().await;
        if active.as_deref() == Some(id) {
            *active = conns.keys().next().cloned();
        }
        Ok(())
    }

    pub async fn set_active(&self, id: &str) -> Result<(), String> {
        let conns = self.connections.lock().await;
        if !conns.contains_key(id) {
            return Err("Connection not found".to_string());
        }
        drop(conns);
        let mut active = self.active_id.lock().await;
        *active = Some(id.to_string());
        Ok(())
    }

    pub async fn get_active_connection(&self) -> Result<RedisConnection, String> {
        let active = self.active_id.lock().await;
        let id = active.as_ref().ok_or_else(|| "No active connection".to_string())?;
        let conns = self.connections.lock().await;
        let entry = conns.get(id).ok_or_else(|| "Connection not found".to_string())?;
        Ok(entry.connection.clone())
    }

    pub async fn get_connection_list(&self) -> Vec<(String, String, String, String)> {
        let conns = self.connections.lock().await;
        let active = self.active_id.lock().await;
        let active_id = active.as_deref().unwrap_or("");
        conns.iter().map(|(id, entry)| {
            (id.clone(), entry.name.clone(), entry.mode.clone(), (id == active_id).to_string())
        }).collect()
    }
}

fn build_redis_url(host_port: &str, password: Option<&str>) -> String {
    let clean = host_port
        .trim()
        .trim_start_matches("redis://")
        .trim_start_matches("rediss://");

    match password {
        Some(pwd) if !pwd.is_empty() => format!("redis://:{}@{}", pwd, clean),
        _ => format!("redis://{}", clean),
    }
}

pub async fn test_redis_connection(url: &str, password: Option<&str>) -> Result<bool, String> {
    let urls: Vec<&str> = url.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    if urls.len() > 1 {
        let connection_urls: Vec<String> = urls.iter().map(|u| build_redis_url(u, password)).collect();
        let url_refs: Vec<&str> = connection_urls.iter().map(|s| s.as_str()).collect();
        let client = ClusterClient::new(url_refs).map_err(|e| format!("Cluster error: {}", e))?;
        let mut con = client.get_async_connection().await.map_err(|e| format!("Connection failed: {}", e))?;
        let _: String = redis::cmd("PING").query_async(&mut con).await.map_err(|e| format!("Ping failed: {}", e))?;
        Ok(true)
    } else {
        let connection_url = build_redis_url(urls[0], password);
        let client = Client::open(connection_url.as_str()).map_err(|e| format!("Client error: {}", e))?;
        let mut con = client.get_multiplexed_async_connection().await.map_err(|e| format!("Connection failed: {}", e))?;
        let _: String = redis::cmd("PING").query_async(&mut con).await.map_err(|e| format!("Ping failed: {}", e))?;
        Ok(true)
    }
}
