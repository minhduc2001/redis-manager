# Changelog

## v0.1.3

### Features
- **Auto-Update**: Tích hợp `tauri-plugin-updater` giúp ứng dụng tự động kiểm tra và cập nhật khi có bản mới. Hiển thị thanh tiến trình tải xuống trực quan.
- **Exact & Like Mode**: Chuyển đổi linh hoạt giữa tìm chính xác và tìm kiếm theo Full SCAN (chống quá tải server).
- **Resizable Panels**: Cho phép kéo dãn Sidebar và danh sách Key giống VS Code, lưu trạng thái kích thước vào bộ nhớ.
- **Sidebar thông minh hơn**: Phân loại màu sắc theo môi trường (PROD/UAT/DEV), thẻ trạng thái kết nối trực quan.

### Bug Fixes
- **Fix tìm kiếm bị sót Key**: Thay vì dùng `SCAN` 1 batch như cũ, giờ đã chuyển sang Full SCAN cho đến `cursor = 0` đảm bảo không bị lặp hay sót key.
- **Sửa lỗi UI/UX**: Tự động ẩn nút "Scan more" khi đang trong chế độ tìm kiếm để tránh nhầm lẫn.
- **Fix đường dẫn import**: Sửa lại các đường dẫn type bị sai (`Cannot find module`).

## v0.1.2

### Bug Fixes
- **Fix duplicate saved connections**: Deduplicate theo URL thay vì ID
- **Auto-reconnect khi mở app**: Tự kết nối lại connection cuối cùng, vào thẳng giao diện chính
- **Click saved = auto-connect**: Click vào saved connection sẽ connect luôn thay vì chỉ fill form
- **Always save connection**: Mọi kết nối thành công đều được lưu lại
- **Fix folder load-more button**: Button "Show more" trong tree view folder hoạt động đúng

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
