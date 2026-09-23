# Changelog

## v0.1.5

### Features & Improvements
- **Hỗ trợ Redis Cluster hoàn chỉnh**:
  - Tự động phát hiện và quét đa Master Nodes qua `CLUSTER NODES`, giải quyết triệt để vấn đề tìm kiếm bị sót key do phân vùng slot hash.
  - Tăng tốc truy vấn loại dữ liệu (`TYPE`) gấp 10x-50x bằng Redis Pipelining.
  - Hiển thị chính xác tổng số keys toàn cụm cluster qua `DBSIZE` và tổng dung lượng RAM tiêu thụ.
- **Nâng cấp Bộ tìm kiếm (Search Bar)**:
  - Bổ sung 3 chế độ tìm kiếm: **Contains** (mặc định), **Prefix**, **Exact**.
  - Tự động nhận diện wildcard `*` và `?`.
  - Khắc phục lỗi trả về rỗng khi không tìm thấy exact key.
- **Bộ công cụ Select All & Batch Actions**:
  - Master checkbox 3 trạng thái (Unchecked, Indeterminate `—`, Checked `✓`).
  - Checkbox chọn theo từng thư mục (Folder) trong Tree View.
  - Thanh thao tác hàng loạt: chọn All, None, Invert, **Copy Key Names** vào clipboard, và xóa hàng loạt với modal xác nhận.
  - Phím tắt `Ctrl+A` / `Cmd+A` để chọn toàn bộ key nhanh.
- **Tạo Key Mới (+ New Key)**:
  - Modal tạo nhanh key trực tiếp cho các kiểu dữ liệu: String, Hash, List, Set, ZSet với TTL tùy chọn.
- **Giao diện người dùng hiện đại (Modern Dark UI)**:
  - Thiết kế lại theme với phong cách neon dark tương phản cao, badge dữ liệu phát sáng, thanh trạng thái máy chủ rõ ràng.

## v0.1.4

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
