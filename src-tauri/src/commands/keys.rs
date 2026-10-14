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

const LUA_SCAN: &str = r#"
local cur = ARGV[1]
local pat = ARGV[2]
local target = tonumber(ARGV[3]) or 500
local hint = tonumber(ARGV[4]) or 10000
local max_iter = tonumber(ARGV[5]) or 60
local filter_type = ARGV[6] or ''
local keys = {}
local types = {}
local seen = {}
local iter = 0

repeat
    iter = iter + 1
    local res
    local used_scan_type = false
    if filter_type ~= '' then
        local ok, r = pcall(redis.call, 'SCAN', cur, 'MATCH', pat, 'COUNT', hint, 'TYPE', filter_type)
        if ok then
            res = r
            used_scan_type = true
        else
            res = redis.call('SCAN', cur, 'MATCH', pat, 'COUNT', hint)
        end
    else
        res = redis.call('SCAN', cur, 'MATCH', pat, 'COUNT', hint)
    end
    cur = tostring(res[1])
    local batch = res[2]
    for i = 1, #batch do
        local k = batch[i]
        if not seen[k] then
            seen[k] = true
            local ts
            if used_scan_type then
                ts = filter_type
            else
                local t = redis.call('TYPE', k)
                ts = (type(t) == 'table' and (t.ok or t[1])) or tostring(t)
            end
            if filter_type == '' or ts == filter_type then
                table.insert(keys, k)
                table.insert(types, ts)
                if #keys >= target then
                    break
                end
            end
        end
    end
until cur == '0' or #keys >= target or iter >= max_iter

return {cur, keys, types}
"#;

const LUA_KEY_DETAIL: &str = r#"
local key = KEYS[1]
local limit = tonumber(ARGV[1]) or 500
local t = redis.call('TYPE', key)
local ktype = (type(t) == 'table' and (t.ok or t[1])) or tostring(t)
if ktype == 'none' then
    return {'none', -2, 0, {}}
end
local ttl = redis.call('TTL', key)
if ktype == 'string' then
    local val = redis.call('GET', key)
    local size = val and #val or 0
    return {'string', ttl, size, {val or ''}}
elseif ktype == 'hash' then
    local size = redis.call('HLEN', key)
    local raw = {}
    if size > 0 then
        if size <= limit then
            raw = redis.call('HGETALL', key)
        else
            raw = redis.call('HSCAN', key, '0', 'COUNT', limit)[2]
        end
    end
    return {'hash', ttl, size, raw}
elseif ktype == 'list' then
    local size = redis.call('LLEN', key)
    local raw = {}
    if size > 0 then
        raw = redis.call('LRANGE', key, 0, limit - 1)
    end
    return {'list', ttl, size, raw}
elseif ktype == 'set' then
    local size = redis.call('SCARD', key)
    local raw = {}
    if size > 0 then
        if size <= limit then
            raw = redis.call('SMEMBERS', key)
        else
            raw = redis.call('SSCAN', key, '0', 'COUNT', limit)[2]
        end
    end
    return {'set', ttl, size, raw}
elseif ktype == 'zset' then
    local size = redis.call('ZCARD', key)
    local raw = {}
    if size > 0 then
        raw = redis.call('ZRANGE', key, 0, limit - 1, 'WITHSCORES')
    end
    return {'zset', ttl, size, raw}
else
    return {ktype, ttl, 0, {}}
end
"#;

fn bytes_to_str(b: Vec<u8>) -> String {
    match String::from_utf8(b) {
        Ok(s) => s,
        Err(err) => String::from_utf8_lossy(&err.into_bytes()).to_string(),
    }
}

async fn get_key_detail_lua<C: redis::aio::ConnectionLike>(
    con: &mut C,
    key: &str,
) -> Result<KeyDetail, String> {
    let (key_type, ttl, size, raw_items): (String, i64, usize, Vec<Vec<u8>>) = redis::cmd("EVAL")
        .arg(LUA_KEY_DETAIL)
        .arg(1)
        .arg(key)
        .arg(500)
        .query_async(con)
        .await
        .map_err(|e| e.to_string())?;

    if key_type == "none" {
        return Err(format!("Key '{}' does not exist or has expired", key));
    }

    let value = match key_type.as_str() {
        "string" => {
            let val_str = raw_items
                .into_iter()
                .next()
                .map(bytes_to_str)
                .unwrap_or_default();
            KeyValue::String(val_str)
        }
        "hash" => {
            let mut fields = Vec::with_capacity(raw_items.len() / 2);
            let mut iter = raw_items.into_iter();
            while let (Some(f), Some(v)) = (iter.next(), iter.next()) {
                fields.push(HashField {
                    field: bytes_to_str(f),
                    value: bytes_to_str(v),
                });
            }
            KeyValue::Hash(fields)
        }
        "list" => {
            let items = raw_items.into_iter().map(bytes_to_str).collect();
            KeyValue::List(items)
        }
        "set" => {
            let members = raw_items.into_iter().map(bytes_to_str).collect();
            KeyValue::Set(members)
        }
        "zset" => {
            let mut zset = Vec::with_capacity(raw_items.len() / 2);
            let mut iter = raw_items.into_iter();
            while let (Some(m), Some(s)) = (iter.next(), iter.next()) {
                let score = bytes_to_str(s).parse::<f64>().unwrap_or(0.0);
                zset.push(ZSetMember {
                    member: bytes_to_str(m),
                    score,
                });
            }
            KeyValue::ZSet(zset)
        }
        _ => KeyValue::Unknown(format!("Unsupported type: {}", key_type)),
    };

    Ok(KeyDetail {
        key: key.to_string(),
        key_type,
        ttl,
        value,
        size,
    })
}

async fn scan_node_lua(
    con: &mut redis::aio::MultiplexedConnection,
    cursor: u64,
    pattern: &str,
    target: usize,
    scan_hint: u64,
    max_iter: usize,
    filter_type: &str,
) -> Result<(u64, Vec<KeyEntry>), String> {
    let (next_cur_str, raw_keys, raw_types): (String, Vec<String>, Vec<String>) = redis::cmd("EVAL")
        .arg(LUA_SCAN)
        .arg(0)
        .arg(cursor.to_string())
        .arg(pattern)
        .arg(target)
        .arg(scan_hint)
        .arg(max_iter)
        .arg(filter_type)
        .query_async(con)
        .await
        .map_err(|e| e.to_string())?;

    let next_cursor = next_cur_str.parse::<u64>().unwrap_or(0);
    let entries = raw_keys
        .into_iter()
        .zip(raw_types)
        .map(|(name, key_type)| KeyEntry { name, key_type })
        .collect();

    Ok((next_cursor, entries))
}

async fn scan_node_native(
    con: &mut redis::aio::MultiplexedConnection,
    cursor: u64,
    pattern: &str,
    target: usize,
    scan_hint: u64,
    max_iter: usize,
    filter_type: &str,
) -> Result<(u64, Vec<KeyEntry>), String> {
    let mut cur = cursor;
    let mut raw_keys = Vec::new();
    let mut seen = HashSet::new();
    let mut iterations = 0;

    loop {
        iterations += 1;
        let mut cmd = redis::cmd("SCAN");
        cmd.arg(cur).arg("MATCH").arg(pattern).arg("COUNT").arg(scan_hint);
        if !filter_type.is_empty() {
            cmd.arg("TYPE").arg(filter_type);
        }

        let (new_cur, batch): (u64, Vec<String>) = match cmd.query_async(con).await {
            Ok(res) => res,
            Err(_) if !filter_type.is_empty() => {
                redis::cmd("SCAN")
                    .arg(cur)
                    .arg("MATCH")
                    .arg(pattern)
                    .arg("COUNT")
                    .arg(scan_hint)
                    .query_async(con)
                    .await
                    .map_err(|e| e.to_string())?
            }
            Err(e) => return Err(e.to_string()),
        };

        for k in batch {
            if seen.insert(k.clone()) {
                raw_keys.push(k);
            }
        }

        cur = new_cur;

        if cur == 0 || raw_keys.len() >= target || iterations >= max_iter {
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
                .query_async(con)
                .await
                .unwrap_or_else(|_| vec!["unknown".to_string(); chunk.len()]);

            for (name, key_type) in chunk.iter().cloned().zip(types) {
                if filter_type.is_empty() || key_type == filter_type {
                    entries.push(KeyEntry { name, key_type });
                }
            }
        }
    }

    Ok((cur, entries))
}

async fn scan_single_node(
    con: &mut redis::aio::MultiplexedConnection,
    cursor: u64,
    pattern: &str,
    target: usize,
    scan_hint: u64,
    max_iter: usize,
    filter_type: &str,
) -> Result<(u64, Vec<KeyEntry>), String> {
    // Try ultra-fast server-side Lua scan first (1 network round-trip instead of 60)
    match scan_node_lua(con, cursor, pattern, target, scan_hint, max_iter, filter_type).await {
        Ok(res) => Ok(res),
        Err(_) => {
            // Graceful fallback to native SCAN pipeline
            scan_node_native(con, cursor, pattern, target, scan_hint, max_iter, filter_type).await
        }
    }
}

#[tauri::command]
pub async fn scan_keys(
    state: State<'_, RedisState>,
    pattern: String,
    cursor: String,
    count: u64,
    key_type: Option<String>,
) -> Result<ScanResult, String> {
    let pattern = if pattern.trim().is_empty() { "*".to_string() } else { pattern };
    let filter_type = key_type
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let filter_type = if filter_type == "all" { String::new() } else { filter_type };
    let is_first_run = cursor == "0" || cursor.trim().is_empty();

    // Fast path: if pattern has no wildcards (* or ?), perform instant O(1) lookup
    if is_first_run && !pattern.contains('*') && !pattern.contains('?') {
        if let Ok(conn) = state.get_active_connection().await {
            let type_check: Result<String, _> = match conn {
                RedisConnection::Standalone(mut con) => {
                    redis::cmd("TYPE").arg(&pattern).query_async(&mut con).await
                }
                RedisConnection::Cluster { mut cluster, .. } => {
                    redis::cmd("TYPE").arg(&pattern).query_async(&mut cluster).await
                }
            };
            if let Ok(ktype) = type_check {
                if ktype != "none" && (filter_type.is_empty() || ktype == filter_type) {
                    return Ok(ScanResult {
                        cursor: "0".to_string(),
                        keys: vec![KeyEntry {
                            name: pattern,
                            key_type: ktype,
                        }],
                    });
                }
            }
        }
    }

    let master_conns = state.get_master_connections().await?;

    if master_conns.len() <= 1 {
        // Standalone or single node
        let mut con = master_conns.into_iter().next().map(|(_, c)| c).ok_or("No connection")?;
        let cur: u64 = cursor.parse().unwrap_or(0);
        let target = count.max(500) as usize;
        let scan_hint = 10000u64.max(count * 5);

        let (next_cursor, entries) = scan_single_node(&mut con, cur, &pattern, target, scan_hint, 60, &filter_type).await?;

        Ok(ScanResult {
            cursor: next_cursor.to_string(),
            keys: entries,
        })
    } else {
        // Cluster: parse compound cursor "node0:cur0;node1:cur1..." and query master nodes concurrently
        let num_nodes = master_conns.len();
        let mut node_cursors: Vec<u64> = vec![0; num_nodes];

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
        let scan_hint = 10000u64.max((count * 5) / (num_nodes as u64));

        let mut node_tasks = Vec::with_capacity(num_nodes);
        for (idx, (_addr, mut con)) in master_conns.into_iter().enumerate() {
            let cur = node_cursors[idx];
            let pat = pattern.clone();
            let ft = filter_type.clone();
            node_tasks.push(async move {
                if !is_first_run && cur == 0 {
                    return (idx, 0u64, Vec::new(), false);
                }
                match scan_single_node(&mut con, cur, &pat, per_node_target, scan_hint, 60, &ft).await {
                    Ok((next_cur, entries)) => (idx, next_cur, entries, false),
                    Err(_) => (idx, cur, Vec::new(), true),
                }
            });
        }

        // Parallel scan across all master nodes
        let results = futures::future::join_all(node_tasks).await;

        let mut all_entries = Vec::new();
        let mut seen = HashSet::new();
        let mut had_node_err = false;

        for (idx, next_cur, entries, is_err) in results {
            if is_err {
                had_node_err = true;
            }
            node_cursors[idx] = next_cur;
            for entry in entries {
                if seen.insert(entry.name.clone()) {
                    all_entries.push(entry);
                }
            }
        }

        if had_node_err {
            state.invalidate_master_connections().await;
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
    key_type: Option<String>,
) -> Result<ScanResult, String> {
    scan_keys(state, pattern, "0".to_string(), max_results.unwrap_or(1000), key_type).await
}


#[tauri::command]
pub async fn get_key_detail(
    state: State<'_, RedisState>,
    key: String,
) -> Result<KeyDetail, String> {
    let conn = state.get_active_connection().await?;

    match conn {
        RedisConnection::Standalone(mut con) => {
            match get_key_detail_lua(&mut con, &key).await {
                Ok(detail) => Ok(detail),
                Err(e) if e.contains("does not exist") => Err(e),
                Err(_) => get_key_detail_impl(&mut con, &key).await,
            }
        }
        RedisConnection::Cluster { mut cluster, .. } => {
            match get_key_detail_lua(&mut cluster, &key).await {
                Ok(detail) => Ok(detail),
                Err(e) if e.contains("does not exist") => Err(e),
                Err(_) => get_key_detail_cluster(&mut cluster, &key).await,
            }
        }
    }
}

async fn get_key_detail_impl(
    con: &mut redis::aio::MultiplexedConnection,
    key: &str,
) -> Result<KeyDetail, String> {
    let mut pipe = redis::pipe();
    pipe.cmd("TYPE").arg(key).cmd("TTL").arg(key);
    let (key_type, ttl): (String, i64) = pipe
        .query_async(con)
        .await
        .map_err(|e| e.to_string())?;

    if key_type == "none" {
        return Err(format!("Key '{}' does not exist or has expired", key));
    }

    let (value, size) = match key_type.as_str() {
        "string" => {
            let raw: Vec<u8> = con.get(key).await.map_err(|e| e.to_string())?;
            let s = raw.len();
            (KeyValue::String(bytes_to_str(raw)), s)
        }
        "hash" => {
            let s: usize = con.hlen(key).await.map_err(|e| e.to_string())?;
            let raw_map: Vec<(Vec<u8>, Vec<u8>)> = if s <= 500 {
                con.hgetall(key).await.map_err(|e| e.to_string())?
            } else {
                let (_cur, flat): (u64, Vec<Vec<u8>>) = redis::cmd("HSCAN")
                    .arg(key)
                    .arg(0)
                    .arg("COUNT")
                    .arg(500)
                    .query_async(con)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut pairs = Vec::with_capacity(flat.len() / 2);
                let mut iter = flat.into_iter();
                while let (Some(f), Some(v)) = (iter.next(), iter.next()) {
                    pairs.push((f, v));
                }
                pairs
            };
            let fields: Vec<HashField> = raw_map
                .into_iter()
                .map(|(f, v)| HashField {
                    field: bytes_to_str(f),
                    value: bytes_to_str(v),
                })
                .collect();
            (KeyValue::Hash(fields), s)
        }
        "list" => {
            let len: isize = con.llen(key).await.map_err(|e| e.to_string())?;
            let limit = std::cmp::min(len, 500);
            let items: Vec<String> = if limit > 0 {
                let raw_items: Vec<Vec<u8>> = con.lrange(key, 0, limit - 1).await.map_err(|e| e.to_string())?;
                raw_items.into_iter().map(bytes_to_str).collect()
            } else {
                Vec::new()
            };
            (KeyValue::List(items), len as usize)
        }
        "set" => {
            let s: usize = con.scard(key).await.map_err(|e| e.to_string())?;
            let raw_members: Vec<Vec<u8>> = if s <= 500 {
                con.smembers(key).await.map_err(|e| e.to_string())?
            } else {
                let (_cur, items): (u64, Vec<Vec<u8>>) = redis::cmd("SSCAN")
                    .arg(key)
                    .arg(0)
                    .arg("COUNT")
                    .arg(500)
                    .query_async(con)
                    .await
                    .map_err(|e| e.to_string())?;
                items
            };
            let members: Vec<String> = raw_members.into_iter().map(bytes_to_str).collect();
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
                    member: bytes_to_str(m),
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
            (KeyValue::String(bytes_to_str(raw)), s)
        }
        "hash" => {
            let s: usize = redis::cmd("HLEN")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let raw_map: Vec<(Vec<u8>, Vec<u8>)> = if s <= 500 {
                redis::cmd("HGETALL")
                    .arg(key)
                    .query_async(con)
                    .await
                    .map_err(|e| e.to_string())?
            } else {
                let (_cur, flat): (u64, Vec<Vec<u8>>) = redis::cmd("HSCAN")
                    .arg(key)
                    .arg(0)
                    .arg("COUNT")
                    .arg(500)
                    .query_async(con)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut pairs = Vec::with_capacity(flat.len() / 2);
                let mut iter = flat.into_iter();
                while let (Some(f), Some(v)) = (iter.next(), iter.next()) {
                    pairs.push((f, v));
                }
                pairs
            };
            let fields: Vec<HashField> = raw_map
                .into_iter()
                .map(|(f, v)| HashField {
                    field: bytes_to_str(f),
                    value: bytes_to_str(v),
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
                raw_items.into_iter().map(bytes_to_str).collect()
            } else {
                Vec::new()
            };
            (KeyValue::List(items), len as usize)
        }
        "set" => {
            let s: usize = redis::cmd("SCARD")
                .arg(key)
                .query_async(con)
                .await
                .map_err(|e| e.to_string())?;
            let raw_members: Vec<Vec<u8>> = if s <= 500 {
                redis::cmd("SMEMBERS")
                    .arg(key)
                    .query_async(con)
                    .await
                    .map_err(|e| e.to_string())?
            } else {
                let (_cur, items): (u64, Vec<Vec<u8>>) = redis::cmd("SSCAN")
                    .arg(key)
                    .arg(0)
                    .arg("COUNT")
                    .arg(500)
                    .query_async(con)
                    .await
                    .map_err(|e| e.to_string())?;
                items
            };
            let members: Vec<String> = raw_members.into_iter().map(bytes_to_str).collect();
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
                    member: bytes_to_str(m),
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
    if keys.is_empty() {
        return Ok(0);
    }
    let conn = state.get_active_connection().await?;
    match conn {
        RedisConnection::Standalone(mut con) => {
            let mut total = 0u64;
            for chunk in keys.chunks(500) {
                let deleted: u64 = match redis::cmd("UNLINK").arg(chunk).query_async(&mut con).await {
                    Ok(c) => c,
                    Err(_) => con.del(chunk).await.map_err(|e| e.to_string())?,
                };
                total += deleted;
            }
            Ok(total)
        }
        RedisConnection::Cluster { cluster, .. } => {
            let mut total = 0u64;
            for chunk in keys.chunks(200) {
                let tasks = chunk.iter().map(|key| {
                    let mut cl = cluster.clone();
                    let k = key.clone();
                    async move {
                        match redis::cmd("UNLINK").arg(&k).query_async::<u64>(&mut cl).await {
                            Ok(d) => Ok(d),
                            Err(_) => redis::cmd("DEL").arg(&k).query_async::<u64>(&mut cl).await,
                        }
                    }
                });
                let results = futures::future::join_all(tasks).await;
                for res in results {
                    total += res.map_err(|e| e.to_string())?;
                }
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
