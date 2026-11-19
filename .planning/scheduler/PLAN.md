# Lịch chạy (scheduler) cho yana-rt

Học nguyên tắc từ Hermes (MIT), không chép mã. Xem `.claude/assistant/hermes-learnings.md` ở checkout chính.

## Hiện trạng
Chưa có lịch chạy cho người dùng. Chỉ có `TurnOrigin::Scheduler` (nguồn của một lượt) và dịch vụ launchd/systemd cho Giám Thị. Không có thư viện cron trong `Cargo.toml`; thêm phụ thuộc cần người duyệt (luật 44), nên tự viết bộ phân tích đơn giản.

## Sáu thứ làm trước (từ phân tích Hermes)
1. Bản ghi job: phần người viết tách khỏi phần lịch tự cập nhật. Lưu nguyên tử.
2. Bộ phân tích lịch + `next_after`, LUÔN nhận múi giờ làm tham số (Hermes ghi đây là loại lỗi hay gặp).
3. Vòng tick một chủ: tăng `next_run` TRƯỚC khi chạy (chạy tối đa một lần), tập đang chạy chống chồng.
4. Sổ thực thi bền, chỉ mục duy nhất theo (job, thời điểm dự kiến).
5. Chính sách chạy sót: một lần bù, hết hạn thì bỏ, ghi "missed xN".
6. Chính sách chạy không người: không tool tương tác, không tool hẹn giờ, việc cần duyệt thì từ chối.

## Lát đầu (bước này): mục 2
`src/schedule/spec.rs`: các dạng lịch, phân tích, `next_after`. Hàm thuần, không I/O, kiểm thử hết.

Dạng được hỗ trợ (cố ý hẹp, không có cron 5 trường ở bản đầu):
- một lần: `in 30m`, `in 2h`, hoặc thời điểm ISO (`2026-12-01T09:00:00+07:00`, `2026-12-01 09:00` theo giờ máy)
- lặp: `every 30m`, `every 2h`, `every 1d` (mốc neo là lúc tạo, không trôi)
- hằng ngày: `daily 09:00`, `weekdays 09:00`
- hằng tuần: `weekly mon,thu 18:30`

Giới hạn: chuỗi tối đa 200 ký tự, không ký tự điều khiển, khoảng lặp từ 1 phút đến 366 ngày.
Giờ địa phương mơ hồ (cuối giờ mùa hè): lấy lần xảy ra đầu. Giờ không tồn tại (đầu giờ mùa hè): dịch tới giờ hợp lệ kế tiếp trong ngày đó.

## Chưa làm (các lát sau)
Mục 1, 3, 4, 5, 6; lệnh `yana-rt schedule ...`; nối với dịch vụ nền; trang Scheduler trong app.
Chưa kiểm DST bằng múi giờ thật vì không có `chrono-tz`; phần chọn giờ được tách ra thành hàm thuần để test trực tiếp.
