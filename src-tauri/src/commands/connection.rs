use serde::Serialize;
use tauri::State;
use crate::redis_client::{RedisState, test_redis_connection, RedisConnection};

#[derive(Serialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub version: String,
    pub mode: String,
    pub connected_clients: String,
    pub used_memory_human: String,
    pub total_keys: String,
    pub uptime_in_seconds: String,
}

#[derive(Serialize)]
pub struct ConnectionTab {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub is_active: bool,
}

#[tauri::command]
pub async fn connect_redis(
    state: State<'_, RedisState>,
    id: String,
    name: String,
    url: String,
    password: Option<String>,
) -> Result<ConnectionInfo, String> {
    let pwd = password.as_deref();
    let mode = state.connect(&id, &name, &url, pwd).await?;
    Ok(ConnectionInfo {
        id,
        name,
        mode,
        url,
    })
}

#[tauri::command]
pub async fn disconnect_redis(
    state: State<'_, RedisState>,
    id: String,
) -> Result<(), String> {
    state.disconnect(&id).await
}

#[tauri::command]
pub async fn set_active_connection(
    state: State<'_, RedisState>,
    id: String,
) -> Result<(), String> {
    state.set_active(&id).await
}

#[tauri::command]
pub async fn get_connections(
    state: State<'_, RedisState>,
) -> Result<Vec<ConnectionTab>, String> {
    let list = state.get_connection_list().await;
    Ok(list.into_iter().map(|(id, name, mode, is_active)| ConnectionTab {
        id,
        name,
        mode,
        is_active: is_active == "true",
    }).collect())
}

#[tauri::command]
pub async fn test_connection(
    url: String,
    password: Option<String>,
) -> Result<bool, String> {
    test_redis_connection(&url, password.as_deref()).await
}

#[tauri::command]
pub async fn get_server_info(state: State<'_, RedisState>) -> Result<ServerInfo, String> {
    let conn = state.get_active_connection().await?;

    let info_str: String = match conn {
        RedisConnection::Standalone(mut con) => {
            redis::cmd("INFO").query_async(&mut con).await.map_err(|e| e.to_string())?
        }
        RedisConnection::Cluster(mut con) => {
            redis::cmd("INFO").query_async(&mut con).await.map_err(|e| e.to_string())?
        }
    };

    let get_field = |field: &str| -> String {
        info_str
            .lines()
            .find(|line| line.starts_with(field))
            .map(|line| line.split(':').nth(1).unwrap_or("").trim().to_string())
            .unwrap_or_else(|| "N/A".to_string())
    };

    let db_keys: String = info_str
        .lines()
        .filter(|line| line.starts_with("db"))
        .map(|line| {
            let keys_part = line.split(':').nth(1).unwrap_or("");
            keys_part
                .split(',')
                .find(|p| p.starts_with("keys="))
                .and_then(|p| p.strip_prefix("keys="))
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
        })
        .sum::<u64>()
        .to_string();

    Ok(ServerInfo {
        version: get_field("redis_version"),
        mode: get_field("redis_mode"),
        connected_clients: get_field("connected_clients"),
        used_memory_human: get_field("used_memory_human"),
        total_keys: db_keys,
        uptime_in_seconds: get_field("uptime_in_seconds"),
    })
}
