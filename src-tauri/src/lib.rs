mod redis_client;
mod commands;

use redis_client::RedisState;
use commands::connection::*;
use commands::keys::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(RedisState::new())
        .invoke_handler(tauri::generate_handler![
            connect_redis,
            disconnect_redis,
            set_active_connection,
            get_connections,
            test_connection,
            get_server_info,
            scan_keys,
            get_key_detail,
            set_key_value,
            set_hash_field,
            delete_keys,
            rename_key,
            set_key_ttl,
            delete_hash_field,
            add_list_item,
            add_set_member,
            execute_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
