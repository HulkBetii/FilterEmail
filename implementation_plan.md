# Đánh giá và Đề xuất UI/UX cho trang "Xác Minh DNS"

Sau khi rà soát kỹ source code (`App.tsx`, `top-dashboard.tsx`) và flow ứng dụng, tôi nhận thấy trang **Xác Minh DNS** đang gặp phải một số vấn đề về khả năng sử dụng (usability) khi nhồi nhét quá nhiều thông tin. Dưới đây là review chi tiết và đề xuất nâng cấp UI (chỉ thuần Frontend).

## 🛑 Các vấn đề UI/UX hiện tại trên tab "Xác Minh DNS"

1. **Left Column (Bảng Cài đặt) quá dài và rối rắm:**
   - Dưới mục chọn folder là phần **DNS Config** hiển thị trực tiếp `Timeout (ms)` và `Max Concurrent Lookups`. Đây là các tham số kỹ thuật, đa số người dùng không cần thay đổi chúng nhưng lại bị phơi ra chiếm chỗ màn hình.
   - Hộp cảnh báo (Amber Review Note) chiếm diện tích cứng ngắc bên trên nút Start.

2. **Right Column (Bảng Kết quả) bị hiệu ứng "Wall of Numbers":**
   - Sự rườm rà lớn nhất ở đây là việc **nhóm thống kê của tab Basic Filter (Public, Edu, Targeted...) vẫn được giữ nguyên layout bệ vệ** trên màn hình Xác Minh.
   - Lọc phân loại cơ bản chỉ là bước "tiền kiểm tra" trước khi gọi DNS. Việc render 6 thẻ này (thậm chí chiếm nhiều diện tích màn hình hơn khối Hero của DNS) làm phân tán sự chú ý vào thông số chính là: DNS có thành công không, có MX không.
   - Right column hiện tại có quá nhiều block: Hero Cards -> Review Required -> SMTP -> Basic Filter StatCards. Cảm giác như nhét cố 17 thẻ số vào nhau.

## 💡 Đề xuất Cải tiến Action Plan (Thuần Frontend)

### 1. Thu gọn Bảng Cài Đặt (Left Column)
- **Cấu hình Nâng cao (Advanced Settings):** Tạo một accordion/toggle "Advanced Options" (VD: `Advanced DNS Configuration`) để giấu `Timeout (ms)` và `Max Concurrent` đi. Mặc định là thu gọn.
- **Giữ nổi bật Persistent Cache:** Mục này là tính năng giá trị, vẫn giữ nằm ngoài hoặc rõ nét.
- **Dọn dẹp Hộp cảnh báo:** Xóa hộp Amber Review Note ở khối bên trái, gộp note đó thành 1 dòng text siêu nhỏ phía dưới khối "Review Required" ở cột bên phải.

### 2. Thu gọn "Basic Filter Stats" khi ở tab Xác Minh DNS
- Lập trình trong `App.tsx`: Nếu `verifyMode = true`, mảng `statCards` chứa (Invalid, Public, Edu...) sẽ **không hiển thị dưới dạng thẻ lớn (cards) nữa**, mà sẽ được thu gọn vào một thanh **"Tiền Xử Lý Phân Loại"** (dạng inline pills hoặc list mini) nằm gọn trên cùng thẻ.
- Sự thay đổi này sẽ giải phóng khoảng 30% diện tích màn hình cột bên phải, nhường Spotlight hoàn toàn cho 4 thẻ MX xanh dương/đỏ và nhóm SMTP.

> [!NOTE]
> **Cam kết:** Mọi tu chỉnh này chỉ là điều chỉnh cách ẩn hiện và CSS layout trên `App.tsx`. Backend (Rust) hoàn toàn không bị thay đổi. 

Bạn thấy hướng điều chỉnh này có giúp app trở nên chuyên nghiệp và clean hơn không? Nếu okay, tôi sẽ tiến hành refactor cục UI trong `App.tsx` ngay nhé!
