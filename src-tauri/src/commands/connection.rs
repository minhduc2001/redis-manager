use serde::Serialize;
use tauri::State;
use crate::redis_client::{RedisState, test_redis_connection};

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
    pub url: String,
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
    Ok(list.into_iter().map(|(id, name, mode, is_active, url)| ConnectionTab {
        id,
        name,
        mode,
        is_active: is_active == "true",
        url,
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
    let entry = state.get_active_entry().await?;
    let master_conns = state.get_master_connections().await?;

    let mut total_keys_count: u64 = 0;
    let mut total_used_memory_bytes: u64 = 0;
    let mut total_clients: u64 = 0;
    let mut first_info_str = String::new();

    let tasks = master_conns.into_iter().map(|(_addr, mut con)| async move {
        // Pipeline DBSIZE and INFO in a single round-trip
        let mut pipe = redis::pipe();
        pipe.cmd("DBSIZE").cmd("INFO");
        if let Ok((count, info)) = pipe.query_async::<(u64, String)>(&mut con).await {
            return (count, Some(info));
        }

        // Fallback if pipeline fails
        let count = redis::cmd("DBSIZE").query_async::<u64>(&mut con).await.unwrap_or(0);
        let info = redis::cmd("INFO").query_async::<String>(&mut con).await.ok();
        (count, info)
    });

    let results = futures::future::join_all(tasks).await;

    for (count, info_opt) in results {
        total_keys_count += count;
        if let Some(info) = info_opt {
            if first_info_str.is_empty() {
                first_info_str = info.clone();
            }

            for line in info.lines() {
                if line.starts_with("used_memory:") {
                    if let Ok(bytes) = line.split(':').nth(1).unwrap_or("0").trim().parse::<u64>() {
                        total_used_memory_bytes += bytes;
                    }
                } else if line.starts_with("connected_clients:") {
                    if let Ok(clients) = line.split(':').nth(1).unwrap_or("0").trim().parse::<u64>() {
                        total_clients += clients;
                    }
                }
            }
        }
    }

    let get_field = |field: &str| -> String {
        first_info_str
            .lines()
            .find(|line| line.starts_with(field))
            .map(|line| line.split(':').nth(1).unwrap_or("").trim().to_string())
            .unwrap_or_else(|| "N/A".to_string())
    };

    let human_mem = if total_used_memory_bytes > 0 {
        format_bytes_human(total_used_memory_bytes)
    } else {
        get_field("used_memory_human")
    };

    let clients_str = if total_clients > 0 {
        total_clients.to_string()
    } else {
        get_field("connected_clients")
    };

    Ok(ServerInfo {
        version: get_field("redis_version"),
        mode: entry.mode.clone(),
        connected_clients: clients_str,
        used_memory_human: human_mem,
        total_keys: total_keys_count.to_string(),
        uptime_in_seconds: get_field("uptime_in_seconds"),
    })
}

fn format_bytes_human(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
