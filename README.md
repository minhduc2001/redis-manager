# Redis Manager

A blazing fast, lightweight Redis GUI client for Windows. Designed to be a low-memory alternative, offering essential management features with a sleek, native-feeling dark UI.

## 📥 Download & Install

**[Download Redis Manager for Windows (v0.1.0)](https://github.com/minhduc2001/redis-manager/releases/download/v0.1.0/RedisManager_x64_en-US.msi)**

1. Download the `.msi` file from the link above.
2. Run the installer and follow the setup wizard.
3. Launch **Redis Manager** from your Start Menu.

---

## ✨ Features

- **Multi-Connection Support**: Manage multiple environments (DEV, UAT, PROD) simultaneously with tabbed navigation.
- **Standalone & Cluster Ready**: Seamlessly connect to single instances or Redis Clusters (auto-detected via comma-separated hosts).
- **Smart Key Browsing**:
  - **Tree View**: Automatically groups keys into folders based on prefixes (`:` and `.`).
  - **Flat View**: Paginated list for quick scrolling.
  - **Search**: Fast, debounced pattern matching (supports `*`).
- **Full CRUD Operations**: View, edit, add, and delete keys. Supports all core data types: `String`, `Hash`, `List`, `Set`, `ZSet`.
- **Advanced JSON Viewer**: Auto-detects JSON strings, providing syntax highlighting and pretty-printing with a Raw/Formatted toggle.
- **Integrated Redis CLI**: A built-in terminal to execute raw Redis commands (e.g., `PING`, `HGETALL`, `KEYS`) with auto-suggestions and formatted history.
- **Low Memory Usage**: Consumes minimal RAM (~15-30MB).

## 📖 Quick Guide

### 1. Connecting to Redis
- **Connection Name**: Give your connection a memorable name (e.g., `DEV Cluster`).
- **Host**: 
  - For standalone: `127.0.0.1:6379`
  - For cluster: `host1:6379,host2:6379,host3:6380`
- Click **Connect**. Connections are automatically saved for next time.

### 2. Browsing Keys
- Use the **🗂 Tree View** to navigate keys logically by their prefix. Folders with more than 50 keys will have a "Show more" button.
- Switch to **≡ Flat View** for a paginated list of all keys.

### 3. CLI Console
- Click the **⌨ CLI Console** tab on the right panel.
- Type any Redis command (e.g., `GET mykey`, `INFO server`).
- Use the UP and DOWN arrow keys to navigate your command history.
