# API key dùng chung: bản thiết kế

Trạng thái (2026-11-26): đã duyệt và đã viết mã cho runtime và hai app. Việc còn lại trước khi phát hành:
thử quyền truy cập Keychain khi chương trình khác đọc key do `yana-rt` tạo, build trên Linux, và phát
hành runtime trước hai app. Chuyển key sang kho chung xóa bản cũ của app, nên bản app cũ không còn thấy.
Mục tiêu: người dùng nhập key **một lần**, app chat và Yana Studio cùng dùng.

## Hiện trạng (đọc mã)

| Nơi | Lưu ở đâu | Gửi cho runtime |
|---|---|---|
| App chat | Keychain, service `ai.yana.app.provider-key`, account = id nhà cung cấp | trường `api_key` qua stdin |
| Yana Studio | SecretStorage của IDE, tên `yana.key.<id>` | trường `api_key` qua stdin |
| `yana-rt` | đã có quy ước Keychain service `yana-ai`, account = tên biến môi trường; **chỉ kiểm có hay không, cố ý không đọc giá trị** (`src/os/platform/macos/secrets.rs`) | nhận `api_key` hoặc biến môi trường |

## Đề xuất

1. Một kho duy nhất: Keychain service `yana-ai` (quy ước runtime đã dùng để kiểm tra).
2. Chỉ `yana-rt` ghi, xóa và đọc. Hai app gọi lệnh, key đi qua stdin, không qua đối số hay file:
   - `yana-rt credential set <provider>` (đọc key từ stdin)
   - `yana-rt credential remove <provider>`
   - `yana-rt credential status --json` (chỉ có hay không)
3. Khi chat, nếu không có `api_key` và không có biến môi trường, runtime lấy từ Keychain, chỉ dùng cho đúng
   nhà cung cấp đó, gửi tới địa chỉ cố định trong catalog. Với `custom` không tự lấy key, vì địa chỉ do
   người gọi đặt: nếu tự lấy thì một lời gọi có thể chuyển key tới máy chủ lạ.
4. Chuyển key cũ: lần mở đầu, mỗi app hỏi một câu "đưa key đã lưu vào kho chung?", ghi bằng lệnh `set`,
   kiểm `status`, rồi mới xóa bản cũ.
5. Cầu nối chỉ báo "danh sách key đã đổi", không mang giá trị.

## Rủi ro phải xử lý trước khi viết mã

| Rủi ro | Cách chặn đề xuất | Còn lại |
|---|---|---|
| Đọc bằng lệnh `security -w` cho phép mọi chương trình gọi lại lệnh đó | Đọc bằng Security.framework trong chính `yana-rt` | Cần thư viện mới, phải qua kiểm duyệt phụ thuộc |
| Agent gọi `yana-rt` để tiêu tiền bằng key | Chặn lệnh chat và credential trong công cụ `run_command` | Chặn theo mẫu lệnh không kín hoàn toàn |
| Agent đọc key | Không có lệnh nào in giá trị key | Mã độc chạy cùng tài khoản vẫn là giới hạn đã biết |
| `yana-rt` ký tạm, mỗi bản mới macOS hỏi lại quyền Keychain | Ký bằng Developer ID khi có | Trước đó người dùng phải bấm cho phép sau mỗi lần cập nhật |
| Đổi ranh giới "không bao giờ đọc giá trị" của runtime | Hai người soát theo luật của repo | Cần anh duyệt |

Chưa kiểm: công cụ `run_command` có truyền biến môi trường chứa key xuống tiến trình con không; Linux và Windows.

## Cũng chỉ cài một lần

Cùng cách này áp cho model, nhà cung cấp đang chọn, địa chỉ custom (đợt 2 trong `SPEC.md`).

## Điều kiện xong

- Nhập key ở app chat, Studio chat được mà không nhập lại, và ngược lại. Có test cho cả hai chiều.
- Không lệnh nào in giá trị key (có test). `custom` không nhận key tự động (có test).
- Chuyển key cũ thất bại thì bản cũ còn nguyên.
