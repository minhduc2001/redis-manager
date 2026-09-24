use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tauri::State;
use redis::AsyncCommands;
use crate::redis_client::{RedisState, RedisConnection};

#[derive(Serialize)]
pub struct ScanResult {
    pub cursor: String,
    pub keys: Vec<KeyEntry>,
}

#[derive(Serialize)]
pub struct KeyEntry {
    pub name: String,
    pub key_type: String,
}

#[derive(Serialize)]
pub struct KeyDetail {
    pub key: String,
    pub key_type: String,
    pub ttl: i64,
    pub value: KeyValue,
    pub size: usize,
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum KeyValue {
    String(String),
    Hash(Vec<HashField>),
    List(Vec<String>),
    Set(Vec<String>),
    ZSet(Vec<ZSetMember>),
    Unknown(String),
}

#[derive(Serialize)]
pub struct HashField {
    pub field: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct ZSetMember {
    pub member: String,
    pub score: f64,
}

#[tauri::command]
pub async fn scan_keys(
    state: State<'_, RedisState>,
    pattern: String,
    cursor: String,
    count: u64,
) -> Result<ScanResult, String> {
    let pattern = if pattern.trim().is_empty() { "*".to_string() } else { pattern };
    let master_conns = state.get_master_connections().await?;

    if master_conns.len() <= 1 {
        // Standalone or single node
        let mut con = master_conns.into_iter().next().map(|(_, c)| c).ok_or("No connection")?;
        let mut cur: u64 = cursor.parse().unwrap_or(0);
        let target = count.max(500) as usize;
        let scan_hint = 2000u64.max(count);
        let mut raw_keys = Vec::new();
        let mut seen = HashSet::new();
        let mut iterations = 0;
        const MAX_ITER: usize = 60;

        loop {
            iterations += 1;
            let (new_cur, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cur)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(scan_hint)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;

            for k in batch {
                if seen.insert(k.clone()) {
                    raw_keys.push(k);
                }
            }

            cur = new_cur;

            if cur == 0 || raw_keys.len() >= target || iterations >= MAX_ITER {
                break;
            }
        }

        let mut entries = Vec::with_capacity(raw_keys.len());
        if !raw_keys.is_empty() {
            for chunk in raw_keys.chunks(500) {
                let mut pipe = redis::pipe();
                for key in chunk {
                    pipe.cmd("TYPE").arg(key);
                }
                let types: Vec<String> = pipe
                    .query_async(&mut con)
                    .await
                    .unwrap_or_else(|_| vec!["unknown".to_string(); chunk.len()]);

                for (name, key_type) in chunk.iter().cloned().zip(types) {
                    entries.push(KeyEntry { name, key_type });
                }
            }
        }

        Ok(ScanResult {
            cursor: cur.to_string(),
            keys: entries,
        })
    } else {
        // Cluster: parse compound cursor "node0:cur0;node1:cur1..."
        let num_nodes = master_conns.len();
        let mut node_cursors: Vec<u64> = vec![0; num_nodes];

        let is_first_run = cursor == "0" || cursor.trim().is_empty();

        if !is_first_run {
            for part in cursor.split(';') {
                let kv: Vec<&str> = part.split(':').collect();
                if kv.len() == 2 {
                    if let (Ok(idx), Ok(c)) = (kv[0].parse::<usize>(), kv[1].parse::<u64>()) {
                        if idx < num_nodes {
                            node_cursors[idx] = c;
                        }
                    }
                }
            }
        }

        let target_total = count.max(500) as usize;
        let per_node_target = (target_total / num_nodes).max(100);
        let scan_hint = 2000u64.max(count / (num_nodes as u64));
        let mut all_entries = Vec::new();
        let mut seen = HashSet::new();

        for (idx, (_addr, mut con)) in master_conns.into_iter().enumerate() {
            let mut cur = node_cursors[idx];
            if !is_first_run && cur == 0 {
                continue;
            }

            let mut node_raw_keys = Vec::new();
            let mut node_iterations = 0;
            const NODE_MAX_ITER: usize = 60;

            loop {
                node_iterations += 1;
                let (new_cur, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                    .arg(cur)
                    .arg("MATCH")
                    .arg(&pattern)
                    .arg("COUNT")
                    .arg(scan_hint)
                    .query_async(&mut con)
                    .await
                    .map_err(|e| e.to_string())?;

                for k in batch {
                    if seen.insert(k.clone()) {
                        node_raw_keys.push(k);
                    }
                }

                cur = new_cur;

                if cur == 0 || node_raw_keys.len() >= per_node_target || node_iterations >= NODE_MAX_ITER {
                    break;
                }
            }

            node_cursors[idx] = cur;

            if !node_raw_keys.is_empty() {
                for chunk in node_raw_keys.chunks(500) {
                    let mut pipe = redis::pipe();
                    for key in chunk {
                        pipe.cmd("TYPE").arg(key);
                    }
                    let types: Vec<String> = pipe
                        .query_async(&mut con)
                        .await
                        .unwrap_or_else(|_| vec!["unknown".to_string(); chunk.len()]);

                    for (name, key_type) in chunk.iter().cloned().zip(types) {
                        all_entries.push(KeyEntry { name, key_type });
                    }
                }
            }
        }

        let all_done = node_cursors.iter().all(|&c| c == 0);
        let next_cursor = if all_done {
            "0".to_string()
        } else {
            node_cursors
                .iter()
                .enumerate()
                .map(|(i, c)| format!("{}:{}", i, c))
                .collect::<Vec<_>>()
                .join(";")
        };

        Ok(ScanResult {
            cursor: next_cursor,
            keys: all_entries,
        })
    }
}

#[tauri::command]
pub async fn search_keys(
    state: State<'_, RedisState>,
    pattern: String,
    max_results: Option<u64>,
) -> Result<ScanResult, String> {
    scan_keys(state, pattern, "0".to_string(), max_results.unwrap_or(1000)).await
}

#[tauri::command]
pub async fn get_key_detail(
    state: State<'_, RedisState>,
    key: String,
) -> Result<KeyDetail, String> {
    let conn = state.get_active_connection().await?;

    match conn {
        RedisConnection::Standalone(mut con) => get_key_detail_impl(&mut con, &key).await,
        RedisConnection::Cluster { mut cluster, .. } => get_key_detail_cluster(&mut cluster, &key).await,
    }
}

async fn get_key_detail_impl(
    con: &mut redis::aio::MultiplexedConnection,
    key: &str,
) -> Result<KeyDetail, String> {
    let key_type: String = redis::cmd("TYPE")
        .arg(key)
        .query_async(con)
        .await
        .map_err(|e| e.to_string())?;

    if key_type == "none" {
        return Err(format!("Key '{}' does not exist or has expired", key));
    }

    let ttl: i64 = con.ttl(key).await.map_err(|e| e.to_string())?;

    let (value, size) = match key_type.as_str() {
        "string" => {
            let raw: Vec<u8> = con.get(key).await.map_err(|e| e.to_string())?;
            let s = raw.len();
            let val_str = match String::from_utf8(raw) {
                Ok(str_val) => str_val,
                Err(err) => String::from_utf8_lossy(&err.into_bytes()).to_string(),
            };
            (KeyValue::String(val_str), s)
        }
        "hash" => {
            let raw_map: Vec<(Vec<u8>, Vec<u8>)> = con.hgetall(key).await.map_err(|e| e.to_string())?;
            let s = raw_map.len();
            let fields: Vec<HashField> = raw_map
                .into_iter()
                .map(|(f, v)| HashField {
                    field: String::from_utf8(f.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&f).to_string()),
                    value: String::from_utf8(v.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&v).to_string()),
                })
                .collect();
            (KeyValue::Hash(fields), s)
        }
        "list" => {
            let len: isize = con.llen(key).await.map_err(|e| e.to_string())?;
            let limit = std::cmp::min(len, 500);
            let items: Vec<String> = if limit > 0 {
                let raw_items: Vec<Vec<u8>> = con.lrange(key, 0, limit - 1).await.map_err(|e| e.to_string())?;
                raw_items
                    .into_iter()
                    .map(|b| String::from_utf8(b.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&b).to_string()))
                    .collect()
            } else {
                Vec::new()
            };
            (KeyValue::List(items), len as usize)
        }
        "set" => {
            let raw_members: Vec<Vec<u8>> = con.smembers(key).await.map_err(|e| e.to_string())?;
            let s = raw_members.len();
            let members: Vec<String> = raw_members
                .into_iter()
                .map(|b| String::from_utf8(b.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&b).to_string()))
                .collect();
            (KeyValue::Set(members), s)
        }
        "zset" => {
            let members: Vec<(Vec<u8>, f64)> = con
                .zrange_withscores(key, 0isize, 499isize)
                .await
                .map_err(|e| e.to_string())?;
            let total: usize = con.zcard(key).await.map_err(|e| e.to_string())?;
            let zset: Vec<ZSetMember> = members
                .into_iter()
                .map(|(m, s)| ZSetMember {
                    member: String::from_utf8(m.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&m).to_string()),
                    score: s,
                })
                .collect();
            (KeyValue::ZSet(zset), total)
        }
        _ => (KeyValue::Unknown(format!("Unsupported type: {}", key_type)), 0),
    };

    Ok(KeyDetail {
        key: key.to_string(),
        key_type,
        ttl,
        value,
        size,
    })
}

async fn get_key_detail_cluster(
    con: &mut redis::cluster_async::ClusterConnection,
    key: &str,
) -> Result<KeyDetail, String> {
    let key_type: String = redis::cmd("TYPE")
        .arg(key)
        .query_async(con)
        .await
        .map_err(|e| e.to_string())?;

    if key_type == "none" {
        return Err(format!("Key '{}' does not exist or has expired", key));
    }

    let ttl: i64 = redis::cmd("TTL")
        .arg(key)
        .query_async(con)
        .await
        .map_err(|e| e.to_string())?;

    let (value, size) = match key_type.as_str() {
        "string" => {
            let raw: Vec<u8> = redis::cmd("GET")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let s = raw.len();
            let val_str = match String::from_utf8(raw) {
                Ok(str_val) => str_val,
                Err(err) => String::from_utf8_lossy(&err.into_bytes()).to_string(),
            };
            (KeyValue::String(val_str), s)
        }
        "hash" => {
            let raw_map: Vec<(Vec<u8>, Vec<u8>)> = redis::cmd("HGETALL")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let s = raw_map.len();
            let fields: Vec<HashField> = raw_map
                .into_iter()
                .map(|(f, v)| HashField {
                    field: String::from_utf8(f.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&f).to_string()),
                    value: String::from_utf8(v.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&v).to_string()),
                })
                .collect();
            (KeyValue::Hash(fields), s)
        }
        "list" => {
            let len: i64 = redis::cmd("LLEN")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let limit = std::cmp::min(len, 500);
            let items: Vec<String> = if limit > 0 {
                let raw_items: Vec<Vec<u8>> = redis::cmd("LRANGE")
                    .arg(key)
                    .arg(0)
                    .arg(limit - 1)
                    .query_async(con)
                    .await
                    .map_err(|e| e.to_string())?;
                raw_items
                    .into_iter()
                    .map(|b| String::from_utf8(b.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&b).to_string()))
                    .collect()
            } else {
                Vec::new()
            };
            (KeyValue::List(items), len as usize)
        }
        "set" => {
            let raw_members: Vec<Vec<u8>> = redis::cmd("SMEMBERS")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let s = raw_members.len();
            let members: Vec<String> = raw_members
                .into_iter()
                .map(|b| String::from_utf8(b.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&b).to_string()))
                .collect();
            (KeyValue::Set(members), s)
        }
        "zset" => {
            let members: Vec<(Vec<u8>, f64)> = redis::cmd("ZRANGE")
                .arg(key)
                .arg(0)
                .arg(499)
                .arg("WITHSCORES")
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let total: i64 = redis::cmd("ZCARD")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let zset: Vec<ZSetMember> = members
                .into_iter()
                .map(|(m, s)| ZSetMember {
                    member: String::from_utf8(m.clone()).unwrap_or_else(|_| String::from_utf8_lossy(&m).to_string()),
                    score: s,
                })
                .collect();
            (KeyValue::ZSet(zset), total as usize)
        }
        _ => (KeyValue::Unknown(format!("Unsupported type: {}", key_type)), 0),
    };

    Ok(KeyDetail {
        key: key.to_string(),
        key_type,
        ttl,
        value,
        size,
    })
}

#[tauri::command]
pub async fn set_key_value(
    state: State<'_, RedisState>,
    key: String,
    value: String,
    ttl: Option<i64>,
) -> Result<(), String> {
    let conn = state.get_active_connection().await?;

    match conn {
        RedisConnection::Standalone(mut con) => {
            let _: () = con.set(&key, &value).await.map_err(|e| e.to_string())?;
            if let Some(t) = ttl {
                if t > 0 {
                    let _: () = con.expire(&key, t).await.map_err(|e| e.to_string())?;
                }
            }
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let _: () = redis::cmd("SET")
                .arg(&key)
                .arg(&value)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
            if let Some(t) = ttl {
                if t > 0 {
                    let _: () = redis::cmd("EXPIRE")
                        .arg(&key)
                        .arg(t)
                        .query_async(&mut cluster)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_hash_field(
    state: State<'_, RedisState>,
    key: String,
    field: String,
    value: String,
) -> Result<(), String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            let _: () = con.hset(&key, &field, &value).await.map_err(|e| e.to_string())?;
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let _: () = redis::cmd("HSET")
                .arg(&key)
                .arg(&field)
                .arg(&value)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_keys(
    state: State<'_, RedisState>,
    keys: Vec<String>,
) -> Result<u64, String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            if keys.is_empty() {
                return Ok(0);
            }
            let deleted: u64 = con.del(&keys).await.map_err(|e| e.to_string())?;
            Ok(deleted)
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let mut total = 0u64;
            for key in &keys {
                let d: u64 = redis::cmd("DEL")
                    .arg(key)
                    .query_async(&mut cluster)
                    .await
                    .map_err(|e| e.to_string())?;
                total += d;
            }
            Ok(total)
        }
    }
}

#[tauri::command]
pub async fn rename_key(
    state: State<'_, RedisState>,
    old_key: String,
    new_key: String,
) -> Result<(), String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            let _: () = redis::cmd("RENAME")
                .arg(&old_key)
                .arg(&new_key)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let _: () = redis::cmd("RENAME")
                .arg(&old_key)
                .arg(&new_key)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_key_ttl(
    state: State<'_, RedisState>,
    key: String,
    ttl: i64,
) -> Result<(), String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            if ttl < 0 {
                let _: () = redis::cmd("PERSIST")
                    .arg(&key)
                    .query_async(&mut con)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                let _: () = con.expire(&key, ttl).await.map_err(|e| e.to_string())?;
            }
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            if ttl < 0 {
                let _: () = redis::cmd("PERSIST")
                    .arg(&key)
                    .query_async(&mut cluster)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                let _: () = redis::cmd("EXPIRE")
                    .arg(&key)
                    .arg(ttl)
                    .query_async(&mut cluster)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_hash_field(
    state: State<'_, RedisState>,
    key: String,
    field: String,
) -> Result<(), String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            let _: () = con.hdel(&key, &field).await.map_err(|e| e.to_string())?;
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let _: () = redis::cmd("HDEL")
                .arg(&key)
                .arg(&field)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn add_list_item(
    state: State<'_, RedisState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            let _: () = con.rpush(&key, &value).await.map_err(|e| e.to_string())?;
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let _: () = redis::cmd("RPUSH")
                .arg(&key)
                .arg(&value)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_list_item(
    state: State<'_, RedisState>,
    key: String,
    value: String,
    count: Option<i64>,
) -> Result<u64, String> {
    let conn = state.get_active_connection().await?;
    let cnt = count.unwrap_or(1);
    match conn {
        RedisConnection::Standalone(mut con) => {
            let removed: u64 = redis::cmd("LREM")
                .arg(&key)
                .arg(cnt)
                .arg(&value)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;
            Ok(removed)
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let removed: u64 = redis::cmd("LREM")
                .arg(&key)
                .arg(cnt)
                .arg(&value)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
            Ok(removed)
        }
    }
}

#[tauri::command]
pub async fn add_set_member(
    state: State<'_, RedisState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            let _: () = con.sadd(&key, &value).await.map_err(|e| e.to_string())?;
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let _: () = redis::cmd("SADD")
                .arg(&key)
                .arg(&value)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_set_member(
    state: State<'_, RedisState>,
    key: String,
    member: String,
) -> Result<u64, String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            let removed: u64 = con.srem(&key, &member).await.map_err(|e| e.to_string())?;
            Ok(removed)
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let removed: u64 = redis::cmd("SREM")
                .arg(&key)
                .arg(&member)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
            Ok(removed)
        }
    }
}

#[tauri::command]
pub async fn add_zset_member(
    state: State<'_, RedisState>,
    key: String,
    score: f64,
    member: String,
) -> Result<(), String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            let _: () = redis::cmd("ZADD")
                .arg(&key)
                .arg(score)
                .arg(&member)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let _: () = redis::cmd("ZADD")
                .arg(&key)
                .arg(score)
                .arg(&member)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_zset_member(
    state: State<'_, RedisState>,
    key: String,
    member: String,
) -> Result<u64, String> {
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            let removed: u64 = redis::cmd("ZREM")
                .arg(&key)
                .arg(&member)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;
            Ok(removed)
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            let removed: u64 = redis::cmd("ZREM")
                .arg(&key)
                .arg(&member)
                .query_async(&mut cluster)
                .await
                .map_err(|e| e.to_string())?;
            Ok(removed)
        }
    }
}

#[tauri::command]
pub async fn create_key(
    state: State<'_, RedisState>,
    key: String,
    key_type: String,
    value: String,
    ttl: Option<i64>,
) -> Result<(), String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Key name cannot be empty".to_string());
    }

    let conn = state.get_active_connection().await?;

    match conn {
        RedisConnection::Standalone(mut con) => {
            match key_type.to_lowercase().as_str() {
                "string" => {
                    let _: () = con.set(&key, &value).await.map_err(|e| e.to_string())?;
                }
                "hash" => {
                    if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&value) {
                        for (f, v) in map {
                            let _: () = con.hset(&key, f, v).await.map_err(|e| e.to_string())?;
                        }
                    } else {
                        let parts: Vec<&str> = value.splitn(2, ':').collect();
                        let (f, v) = if parts.len() == 2 {
                            (parts[0].trim(), parts[1].trim())
                        } else {
                            ("field", value.as_str())
                        };
                        let _: () = con.hset(&key, f, v).await.map_err(|e| e.to_string())?;
                    }
                }
                "list" => {
                    let _: () = con.rpush(&key, &value).await.map_err(|e| e.to_string())?;
                }
                "set" => {
                    let _: () = con.sadd(&key, &value).await.map_err(|e| e.to_string())?;
                }
                "zset" => {
                    let parts: Vec<&str> = value.split_whitespace().collect();
                    let (score, member) = if parts.len() >= 2 {
                        let s = parts[0].parse::<f64>().unwrap_or(0.0);
                        (s, parts[1..].join(" "))
                    } else {
                        (0.0, value.clone())
                    };
                    let _: () = redis::cmd("ZADD")
                        .arg(&key)
                        .arg(score)
                        .arg(&member)
                        .query_async(&mut con)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                _ => return Err(format!("Unsupported key type: {}", key_type)),
            }

            if let Some(t) = ttl {
                if t > 0 {
                    let _: () = con.expire(&key, t).await.map_err(|e| e.to_string())?;
                }
            }
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            match key_type.to_lowercase().as_str() {
                "string" => {
                    let _: () = redis::cmd("SET")
                        .arg(&key)
                        .arg(&value)
                        .query_async(&mut cluster)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                "hash" => {
                    if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&value) {
                        for (f, v) in map {
                            let _: () = redis::cmd("HSET")
                                .arg(&key)
                                .arg(f)
                                .arg(v)
                                .query_async(&mut cluster)
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                    } else {
                        let parts: Vec<&str> = value.splitn(2, ':').collect();
                        let (f, v) = if parts.len() == 2 {
                            (parts[0].trim(), parts[1].trim())
                        } else {
                            ("field", value.as_str())
                        };
                        let _: () = redis::cmd("HSET")
                            .arg(&key)
                            .arg(f)
                            .arg(v)
                            .query_async(&mut cluster)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                }
                "list" => {
                    let _: () = redis::cmd("RPUSH")
                        .arg(&key)
                        .arg(&value)
                        .query_async(&mut cluster)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                "set" => {
                    let _: () = redis::cmd("SADD")
                        .arg(&key)
                        .arg(&value)
                        .query_async(&mut cluster)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                "zset" => {
                    let parts: Vec<&str> = value.split_whitespace().collect();
                    let (score, member) = if parts.len() >= 2 {
                        let s = parts[0].parse::<f64>().unwrap_or(0.0);
                        (s, parts[1..].join(" "))
                    } else {
                        (0.0, value.clone())
                    };
                    let _: () = redis::cmd("ZADD")
                        .arg(&key)
                        .arg(score)
                        .arg(&member)
                        .query_async(&mut cluster)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                _ => return Err(format!("Unsupported key type: {}", key_type)),
            }

            if let Some(t) = ttl {
                if t > 0 {
                    let _: () = redis::cmd("EXPIRE")
                        .arg(&key)
                        .arg(t)
                        .query_async(&mut cluster)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn execute_command(
    state: State<'_, RedisState>,
    command: String,
) -> Result<String, String> {
    let conn = state.get_active_connection().await?;

    let parts = parse_command_line(&command);
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let cmd_name = parts[0].to_uppercase();
    let mut cmd = redis::cmd(&cmd_name);
    for arg in &parts[1..] {
        cmd.arg(arg.as_str());
    }

    let result: redis::RedisResult<redis::Value> = match conn {
        RedisConnection::Standalone(mut con) => cmd.query_async(&mut con).await,
        RedisConnection::Cluster { mut cluster, .. } => cmd.query_async(&mut cluster).await,
    };

    match result {
        Ok(value) => Ok(format_redis_value(&value, 0)),
        Err(e) => Err(format!("(error) {}", e)),
    }
}

fn parse_command_line(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = '"';

    for ch in input.chars() {
        match ch {
            '"' | '\'' if !in_quote => {
                in_quote = true;
                quote_char = ch;
            }
            c if c == quote_char && in_quote => {
                in_quote = false;
            }
            ' ' if !in_quote => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn format_redis_value(value: &redis::Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match value {
        redis::Value::Nil => "(nil)".to_string(),
        redis::Value::Int(i) => format!("(integer) {}", i),
        redis::Value::BulkString(data) => {
            match String::from_utf8(data.clone()) {
                Ok(s) => format!("\"{}\"", s),
                Err(_) => format!("(binary) {} bytes", data.len()),
            }
        }
        redis::Value::Array(arr) => {
            if arr.is_empty() {
                return "(empty array)".to_string();
            }
            let mut lines = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let formatted = format_redis_value(item, indent + 1);
                lines.push(format!("{}{}) {}", pad, i + 1, formatted));
            }
            lines.join("\n")
        }
        redis::Value::SimpleString(s) => s.clone(),
        redis::Value::Okay => "OK".to_string(),
        redis::Value::Map(pairs) => {
            if pairs.is_empty() {
                return "(empty map)".to_string();
            }
            let mut lines = Vec::new();
            for (i, (k, v)) in pairs.iter().enumerate() {
                let key_str = format_redis_value(k, 0);
                let val_str = format_redis_value(v, indent + 1);
                lines.push(format!("{}{}) {} => {}", pad, i + 1, key_str, val_str));
            }
            lines.join("\n")
        }
        redis::Value::Double(f) => format!("(double) {}", f),
        redis::Value::Boolean(b) => format!("(boolean) {}", b),
        redis::Value::VerbatimString { format: _, text } => format!("\"{}\"", text),
        redis::Value::BigNumber(n) => format!("(big number) {}", n),
        redis::Value::Set(items) => {
            if items.is_empty() {
                return "(empty set)".to_string();
            }
            let mut lines = Vec::new();
            for (i, item) in items.iter().enumerate() {
                let formatted = format_redis_value(item, indent + 1);
                lines.push(format!("{}{}) {}", pad, i + 1, formatted));
            }
            lines.join("\n")
        }
        redis::Value::Attribute { data, attributes: _ } => {
            format_redis_value(data, indent)
        }
        redis::Value::Push { kind: _, data } => {
            let mut lines = Vec::new();
            for (i, item) in data.iter().enumerate() {
                let formatted = format_redis_value(item, indent + 1);
                lines.push(format!("{}{}) {}", pad, i + 1, formatted));
            }
            lines.join("\n")
        }
        redis::Value::ServerError(e) => format!("(error) {}", e.details().unwrap_or("unknown")),
    }
}
