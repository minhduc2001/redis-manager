# Changelog

## v0.1.1

### Bug Fixes
- **Fix Redis Cluster connection**: Sử dụng `ClusterClient::builder()` với retry (3 lần) và timeout (5s) thay vì `ClusterClient::new()`
- **Fallback thông minh**: Nếu cluster connect thất bại → tự thử standalone, và ngược lại
- **Fix URL auth**: Xử lý đúng trường hợp URL đã chứa auth khi thêm password

## v0.1.0

### Features
- Multi-connection support (DEV, UAT, PROD) với tab switching
- Hỗ trợ Redis Standalone và Cluster (auto-detect qua dấu `,`)
- Tree View: gom nhóm key theo prefix (`:` và `.`)
- Flat View: phân trang 50 keys/page
- CRUD cho tất cả data types: String, Hash, List, Set, ZSet
- JSON syntax highlighting với Raw/Formatted toggle
- Redis CLI Console tích hợp
- Confirmation dialog cho disconnect và delete
- Search với debounce và wildcard `*`
- Saved Connections (localStorage)
