use serde::Serialize;
use tauri::State;
use redis::AsyncCommands;
use crate::redis_client::{RedisState, RedisConnection};

#[derive(Serialize)]
pub struct ScanResult {
    pub cursor: u64,
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
    cursor: u64,
    count: u64,
) -> Result<ScanResult, String> {
    let conn = state.get_active_connection().await?;
    let pattern = if pattern.is_empty() { "*".to_string() } else { pattern };

    match conn {
        RedisConnection::Standalone(mut con) => {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(count)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;

            let mut entries = Vec::new();
            for key in &keys {
                let kt: String = redis::cmd("TYPE")
                    .arg(key)
                    .query_async(&mut con)
                    .await
                    .unwrap_or_else(|_| "unknown".to_string());
                entries.push(KeyEntry {
                    name: key.clone(),
                    key_type: kt,
                });
            }

            Ok(ScanResult {
                cursor: new_cursor,
                keys: entries,
            })
        }
        RedisConnection::Cluster(mut con) => {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(count)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;

            let mut entries = Vec::new();
            for key in &keys {
                let kt: String = redis::cmd("TYPE")
                    .arg(key)
                    .query_async(&mut con)
                    .await
                    .unwrap_or_else(|_| "unknown".to_string());
                entries.push(KeyEntry {
                    name: key.clone(),
                    key_type: kt,
                });
            }

            Ok(ScanResult {
                cursor: new_cursor,
                keys: entries,
            })
        }
    }
}

#[tauri::command]
pub async fn get_key_detail(
    state: State<'_, RedisState>,
    key: String,
) -> Result<KeyDetail, String> {
    let conn = state.get_active_connection().await?;

    match conn {
        RedisConnection::Standalone(mut con) => get_key_detail_impl(&mut con, &key).await,
        RedisConnection::Cluster(mut con) => get_key_detail_cluster(&mut con, &key).await,
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

    let ttl: i64 = con.ttl(key).await.map_err(|e| e.to_string())?;

    let (value, size) = match key_type.as_str() {
        "string" => {
            let v: String = con.get(key).await.map_err(|e| e.to_string())?;
            let s = v.len();
            (KeyValue::String(v), s)
        }
        "hash" => {
            let map: Vec<(String, String)> = con.hgetall(key).await.map_err(|e| e.to_string())?;
            let s = map.len();
            let fields: Vec<HashField> = map
                .into_iter()
                .map(|(f, v)| HashField { field: f, value: v })
                .collect();
            (KeyValue::Hash(fields), s)
        }
        "list" => {
            let len: isize = con.llen(key).await.map_err(|e| e.to_string())?;
            let limit = std::cmp::min(len, 500);
            let items: Vec<String> = con.lrange(key, 0, limit - 1).await.map_err(|e| e.to_string())?;
            (KeyValue::List(items), len as usize)
        }
        "set" => {
            let members: Vec<String> = con.smembers(key).await.map_err(|e| e.to_string())?;
            let s = members.len();
            (KeyValue::Set(members), s)
        }
        "zset" => {
            let members: Vec<(String, f64)> = con
                .zrange_withscores(key, 0isize, 499isize)
                .await
                .map_err(|e| e.to_string())?;
            let total: usize = con.zcard(key).await.map_err(|e| e.to_string())?;
            let zset: Vec<ZSetMember> = members
                .into_iter()
                .map(|(m, s)| ZSetMember { member: m, score: s })
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

    let ttl: i64 = redis::cmd("TTL")
        .arg(key)
        .query_async(con)
        .await
        .map_err(|e| e.to_string())?;

    let (value, size) = match key_type.as_str() {
        "string" => {
            let v: String = redis::cmd("GET")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let s = v.len();
            (KeyValue::String(v), s)
        }
        "hash" => {
            let map: Vec<(String, String)> = redis::cmd("HGETALL")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let s = map.len();
            let fields: Vec<HashField> = map
                .into_iter()
                .map(|(f, v)| HashField { field: f, value: v })
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
            let items: Vec<String> = redis::cmd("LRANGE")
                .arg(key)
                .arg(0)
                .arg(limit - 1)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            (KeyValue::List(items), len as usize)
        }
        "set" => {
            let members: Vec<String> = redis::cmd("SMEMBERS")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let s = members.len();
            (KeyValue::Set(members), s)
        }
        "zset" => {
            let members: Vec<(String, f64)> = redis::cmd("ZRANGE")
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
                .map(|(m, s)| ZSetMember { member: m, score: s })
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
        RedisConnection::Cluster(mut con) => {
            let _: () = redis::cmd("SET")
                .arg(&key)
                .arg(&value)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;
            if let Some(t) = ttl {
                if t > 0 {
                    let _: () = redis::cmd("EXPIRE")
                        .arg(&key)
                        .arg(t)
                        .query_async(&mut con)
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
        RedisConnection::Cluster(mut con) => {
            let _: () = redis::cmd("HSET")
                .arg(&key)
                .arg(&field)
                .arg(&value)
                .query_async(&mut con)
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
            let deleted: u64 = con.del(&keys).await.map_err(|e| e.to_string())?;
            Ok(deleted)
        }
        RedisConnection::Cluster(mut con) => {
            let mut total = 0u64;
            for key in &keys {
                let d: u64 = redis::cmd("DEL")
                    .arg(key)
                    .query_async(&mut con)
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
        RedisConnection::Cluster(mut con) => {
            let _: () = redis::cmd("RENAME")
                .arg(&old_key)
                .arg(&new_key)
                .query_async(&mut con)
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
        RedisConnection::Cluster(mut con) => {
            if ttl < 0 {
                let _: () = redis::cmd("PERSIST")
                    .arg(&key)
                    .query_async(&mut con)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                let _: () = redis::cmd("EXPIRE")
                    .arg(&key)
                    .arg(ttl)
                    .query_async(&mut con)
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
        RedisConnection::Cluster(mut con) => {
            let _: () = redis::cmd("HDEL")
                .arg(&key)
                .arg(&field)
                .query_async(&mut con)
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
        RedisConnection::Cluster(mut con) => {
            let _: () = redis::cmd("RPUSH")
                .arg(&key)
                .arg(&value)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
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
        RedisConnection::Cluster(mut con) => {
            let _: () = redis::cmd("SADD")
                .arg(&key)
                .arg(&value)
                .query_async(&mut con)
                .await
                .map_err(|e| e.to_string())?;
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

    // Parse command string into parts (handle quoted strings)
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
        RedisConnection::Cluster(mut con) => cmd.query_async(&mut con).await,
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
