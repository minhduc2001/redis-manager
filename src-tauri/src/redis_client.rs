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
    Cluster {
        cluster: ClusterConnection,
        masters: Arc<Mutex<Vec<(String, MultiplexedConnection)>>>,
    },
}

#[derive(Clone)]
pub struct ConnectionEntry {
    pub connection: RedisConnection,
    pub url: String,
    pub mode: String,
    pub name: String,
    pub password: Option<String>,
    pub master_nodes: Vec<String>,
    pub seed_hosts: Vec<String>,
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
        if urls.is_empty() {
            return Err("No connection URLs provided".to_string());
        }
        let is_cluster = urls.len() > 1;

        let seed_hosts: Vec<String> = urls.iter().map(|u| extract_host(u)).collect();

        let (conn, mode, master_nodes) = if is_cluster {
            // Try cluster connection
            match self.connect_cluster(&urls, password).await {
                Ok(mut con) => {
                    let masters = discover_cluster_masters(&mut con, &urls).await;
                    (
                        RedisConnection::Cluster {
                            cluster: con,
                            masters: Arc::new(Mutex::new(Vec::new())),
                        },
                        "cluster".to_string(),
                        masters,
                    )
                }
                Err(cluster_err) => {
                    // If cluster fails, try connecting to the first node as standalone
                    match self.connect_standalone(urls[0], password).await {
                        Ok(con) => (
                            RedisConnection::Standalone(con),
                            "standalone".to_string(),
                            vec![urls[0].to_string()],
                        ),
                        Err(_) => return Err(format!("Cluster connection failed: {}. Also failed as standalone.", cluster_err)),
                    }
                }
            }
        } else {
            // Try standalone first, if it fails try as single-node cluster
            match self.connect_standalone(urls[0], password).await {
                Ok(con) => (
                    RedisConnection::Standalone(con),
                    "standalone".to_string(),
                    vec![urls[0].to_string()],
                ),
                Err(standalone_err) => {
                    match self.connect_cluster(&urls, password).await {
                        Ok(mut con) => {
                            let masters = discover_cluster_masters(&mut con, &urls).await;
                            (
                                RedisConnection::Cluster {
                                    cluster: con,
                                    masters: Arc::new(Mutex::new(Vec::new())),
                                },
                                "cluster".to_string(),
                                masters,
                            )
                        }
                        Err(_) => return Err(format!("Connection failed: {}", standalone_err)),
                    }
                }
            }
        };

        let entry = ConnectionEntry {
            connection: conn,
            url: url.to_string(),
            mode: mode.clone(),
            name: name.to_string(),
            password: password.map(|p| p.to_string()),
            master_nodes,
            seed_hosts,
        };

        let mut conns = self.connections.lock().await;
        conns.insert(id.to_string(), entry);
        let mut active = self.active_id.lock().await;
        *active = Some(id.to_string());

        Ok(mode)
    }

    async fn connect_standalone(&self, host: &str, password: Option<&str>) -> Result<MultiplexedConnection, String> {
        let connection_url = build_redis_url(host, password);
        let client = Client::open(connection_url.as_str())
            .map_err(|e| format!("Failed to create client: {}", e))?;
        let con = client.get_multiplexed_async_connection().await
            .map_err(|e| format!("Failed to connect: {}", e))?;
        Ok(con)
    }

    async fn connect_cluster(&self, urls: &[&str], password: Option<&str>) -> Result<ClusterConnection, String> {
        let connection_urls: Vec<String> = urls.iter().map(|u| build_redis_url(u, password)).collect();
        let url_refs: Vec<&str> = connection_urls.iter().map(|s| s.as_str()).collect();

        let client = ClusterClient::builder(url_refs)
            .retries(3)
            .connection_timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Failed to build cluster client: {}", e))?;

        let con = client.get_async_connection().await
            .map_err(|e| format!("Failed to connect to cluster: {}", e))?;
        Ok(con)
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

    pub async fn get_active_entry(&self) -> Result<ConnectionEntry, String> {
        let active = self.active_id.lock().await;
        let id = active.as_ref().ok_or_else(|| "No active connection".to_string())?;
        let conns = self.connections.lock().await;
        let entry = conns.get(id).ok_or_else(|| "Connection not found".to_string())?;
        Ok(entry.clone())
    }

    pub async fn get_master_connections(&self) -> Result<Vec<(String, MultiplexedConnection)>, String> {
        let entry = self.get_active_entry().await?;
        match &entry.connection {
            RedisConnection::Standalone(con) => {
                Ok(vec![(entry.url.clone(), con.clone())])
            }
            RedisConnection::Cluster { masters, .. } => {
                let mut guard = masters.lock().await;
                // Test if cached master connections are still alive
                let mut valid_conns = Vec::new();
                for (addr, mut con) in guard.iter().cloned() {
                    let ping: Result<String, _> = redis::cmd("PING").query_async(&mut con).await;
                    if ping.is_ok() {
                        valid_conns.push((addr, con));
                    }
                }

                if !valid_conns.is_empty() && valid_conns.len() >= entry.master_nodes.len().max(1) {
                    *guard = valid_conns.clone();
                    return Ok(valid_conns);
                }

                // Reconnect missing or all master nodes
                let mut conns = Vec::new();
                for node in &entry.master_nodes {
                    if let Ok(con) = connect_master_node(node, &entry.seed_hosts, entry.password.as_deref()).await {
                        conns.push((node.clone(), con));
                    }
                }

                // If no master connected from announced list, fallback to user seed URLs
                if conns.is_empty() {
                    for seed in entry.url.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        if let Ok(con) = connect_master_node(seed, &entry.seed_hosts, entry.password.as_deref()).await {
                            conns.push((seed.to_string(), con));
                        }
                    }
                }

                if conns.is_empty() {
                    return Err("Failed to connect to any cluster master nodes".to_string());
                }

                *guard = conns.clone();
                Ok(conns)
            }
        }
    }

    pub async fn get_connection_list(&self) -> Vec<(String, String, String, String, String)> {
        let conns = self.connections.lock().await;
        let active = self.active_id.lock().await;
        let active_id = active.as_deref().unwrap_or("");
        conns.iter().map(|(id, entry)| {
            (id.clone(), entry.name.clone(), entry.mode.clone(), (id == active_id).to_string(), entry.url.clone())
        }).collect()
    }
}

pub fn extract_host(host_port: &str) -> String {
    let clean = clean_host_port(host_port);
    clean.split(':').next().unwrap_or(&clean).to_string()
}

pub fn clean_host_port(host_port: &str) -> String {
    let clean = host_port
        .trim()
        .trim_start_matches("redis://")
        .trim_start_matches("rediss://");
    if clean.contains('@') {
        clean.split('@').last().unwrap_or(clean).to_string()
    } else {
        clean.to_string()
    }
}

pub fn build_redis_url(host_port: &str, password: Option<&str>) -> String {
    let clean = clean_host_port(host_port);

    match password {
        Some(pwd) if !pwd.is_empty() => format!("redis://:{}@{}", pwd, clean),
        _ => format!("redis://{}", clean),
    }
}

async fn discover_cluster_masters(con: &mut ClusterConnection, seed_urls: &[&str]) -> Vec<String> {
    let mut masters = Vec::new();
    let nodes_info: Result<String, _> = redis::cmd("CLUSTER").arg("NODES").query_async(con).await;
    if let Ok(info) = nodes_info {
        for line in info.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let addr_raw = parts[1];
            let flags = parts[2];

            let is_master = flags.split(',').any(|f| f == "master");
            let is_fail = flags.contains("fail");

            if is_master && !is_fail {
                let addr_clean = addr_raw.split('@').next().unwrap_or(addr_raw);
                let host_port = addr_clean.split(',').next().unwrap_or(addr_clean);
                if !host_port.is_empty() && !masters.contains(&host_port.to_string()) {
                    masters.push(host_port.to_string());
                }
            }
        }
    }

    if masters.is_empty() {
        for u in seed_urls {
            let host_port = clean_host_port(u);
            if !host_port.is_empty() && !masters.contains(&host_port) {
                masters.push(host_port);
            }
        }
    }

    masters
}

pub async fn connect_master_node(
    node_addr: &str,
    seed_hosts: &[String],
    password: Option<&str>,
) -> Result<MultiplexedConnection, String> {
    // 1. Try direct node_addr
    let direct_url = build_redis_url(node_addr, password);
    if let Ok(client) = Client::open(direct_url.as_str()) {
        if let Ok(con) = client.get_multiplexed_async_connection().await {
            return Ok(con);
        }
    }

    // 2. Fallback: try using announced port with seed hosts (for Docker NAT/port-forwarding)
    let port = node_addr.split(':').last().unwrap_or("");
    if !port.is_empty() {
        for host in seed_hosts {
            let alt_addr = format!("{}:{}", host, port);
            if alt_addr != node_addr {
                let alt_url = build_redis_url(&alt_addr, password);
                if let Ok(client) = Client::open(alt_url.as_str()) {
                    if let Ok(con) = client.get_multiplexed_async_connection().await {
                        return Ok(con);
                    }
                }
            }
        }
    }

    Err(format!("Failed to connect to cluster master node: {}", node_addr))
}

pub async fn test_redis_connection(url: &str, password: Option<&str>) -> Result<bool, String> {
    let urls: Vec<&str> = url.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    if urls.len() > 1 {
        // Cluster
        let connection_urls: Vec<String> = urls.iter().map(|u| build_redis_url(u, password)).collect();
        let url_refs: Vec<&str> = connection_urls.iter().map(|s| s.as_str()).collect();
        let client = ClusterClient::builder(url_refs)
            .retries(2)
            .connection_timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Cluster error: {}", e))?;
        let mut con = client.get_async_connection().await
            .map_err(|e| format!("Connection failed: {}", e))?;
        let _: String = redis::cmd("PING").query_async(&mut con).await
            .map_err(|e| format!("Ping failed: {}", e))?;
        Ok(true)
    } else {
        // Standalone
        let connection_url = build_redis_url(urls[0], password);
        let client = Client::open(connection_url.as_str())
            .map_err(|e| format!("Client error: {}", e))?;
        let mut con = client.get_multiplexed_async_connection().await
            .map_err(|e| format!("Connection failed: {}", e))?;
        let _: String = redis::cmd("PING").query_async(&mut con).await
            .map_err(|e| format!("Ping failed: {}", e))?;
        Ok(true)
    }
}
