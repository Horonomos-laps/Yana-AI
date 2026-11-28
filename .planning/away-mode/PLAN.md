# Chế độ vắng mặt (away mode)

Người dùng bật trước khi rời đi; công việc tiếp tục trong một cái hộp người dùng vẽ ra.

## Hộp
- Thời gian: 5 phút đến 24 giờ. Ngân sách: số hành động nó được tự duyệt, 1 đến 1000. Cấp: 3 đến 5 (cấp 1 và 2 sẽ không chạy gì mà không hỏi).
- Bản ghi nằm ở kho riêng của người dùng (`<trust store>/away/<hash repo>.json`), ngoài repo, cùng chỗ với các quyền đã nhớ và cấp kiểm soát. Bản ghi không đọc được thì coi như không vắng mặt (hỏi như thường), không bao giờ tự duyệt nhầm.
- Chỉ một hộp tại một thời điểm; bật lần hai khi đang bật bị từ chối, để việc kéo dài không diễn ra trong im lặng.
- Agent không chạy được `yana-rt away` cho chính nó ở bất kỳ cấp nào (`level_rules`).

## Trong hộp
- Việc thuộc cấp của hộp và còn ngân sách: chạy, tiêu 1 đơn vị (có khóa, nên nhiều việc cùng lúc không tiêu quá), biên nhận ghi bằng chứng `away:<id>`.
- Việc nguy hiểm, hoặc ngoài cấp, hoặc khi hết ngân sách: KHÔNG duyệt và KHÔNG từ chối hẳn. Chuyển thành "xếp hàng chờ": agent nhận một kết quả báo lỗi đọc được ("đã xếp hàng cho người dùng, đừng thử lại, làm việc không cần nó"), lượt tiếp tục, và mục đó nằm trong hàng chờ (tối đa 200, giữ mục mới nhất).
- Chỉ lượt của chính người dùng được hưởng. Sub-agent vẫn bị từ chối như cũ. Câu trả lời người dùng vừa đưa cho một lệnh cụ thể thì ghi là duyệt của người dùng, không tiêu ngân sách.
- Hết giờ, hết ngân sách hoặc người dùng dừng: quay về hỏi như thường.

## Khi quay về
`yana-rt away report [--clear] [--json]`: hộp, đã tiêu bao nhiêu, và hàng chờ (mô tả đã bỏ ký tự điều khiển, tối đa 200 ký tự). Mỗi lần tự duyệt đã có biên nhận trong `authority-receipts.jsonl`.

## Chưa làm
- Giới hạn tốc độ và chờ thử lại khi nhà cung cấp bị giới hạn: thuộc lớp provider, chưa nối với hộp. Hiện hộp chỉ dừng ở ngân sách và thời gian.
- Duyệt từng mục trong hàng chờ ngay từ app (hiện chỉ xem và xóa hàng chờ). Một mục trong hàng chờ chưa thể "duyệt" bằng một cú bấm vì lượt đã đi tiếp.
- Hàng chờ chỉ có mô tả để người đọc, không phải thứ chạy được.
- Chưa có UI trong app.
- Danh sách nguy hiểm và danh sách an toàn là phán đoán của người viết, chưa được kiểm bằng dữ liệu chạy bóng.
