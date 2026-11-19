# Duyệt lệnh 3 mức: once / session / always

Nhánh: `feat/approval-scopes` (từ origin/main cc1193ce). Chỉ commit nội bộ, chưa push.
Học nguyên tắc từ Hermes (MIT), không chép mã. Xem `.claude/assistant/hermes-learnings.md` ở checkout chính.

## Hiện trạng (đã đọc mã)
- `PendingApproval` ghi đúng một quyết định bool (`decision`, `decided_by`), resolve đúng một lần (`pending_approval.rs`).
- Luồng nối: app gọi `yana-rt chat --resume-approval` với JSON `{approval_id, decision, decided_by}` (`chat/headless.rs`, `HeadlessResumeInput`, `deny_unknown_fields`).
- `capability_decision` (`runtime/authority.rs`): HALT, availability, rồi lease, rồi `HumanApprovalPerCall`.
- Lease (`capability/lease.rs`) đã có: chủ thể, năng lực, phạm vi lệnh, hạn, ngân sách, thu hồi, ủy quyền cha-con. Nhưng chỉ được dùng khi `context.agent_id` có giá trị (sub-agent). Lượt chat do người dùng khởi tạo chưa dùng lease.

## Quyết định thiết kế
1. `once` (mặc định, giữ nguyên hành vi cũ): không lưu gì.
2. `session` và `always` chỉ có nghĩa khi `decision = true`. Từ chối luôn là `once`; `scope` kèm `decision=false` bị từ chối là dữ liệu sai.
3. Cấp quyền thực chất là cấp một lease. Không tạo cơ chế mới.
4. Phạm vi khớp LỆNH CHÍNH XÁC (so khớp từng token), không khớp tiền tố. Lý do: tiền tố của một lệnh push khớp cả lệnh đó kèm cờ thêm. Cách làm thực tế: mục `allow` có tiền tố `exact:` (KHÔNG thêm trường vào `Lease`). Bản cũ đọc mục này như tiền tố có token đầu `exact:git`, không khớp lệnh thật nào, nên hạ cấp chỉ làm mất quyền, không mở rộng quyền. Trường bool mới sẽ bị serde bỏ qua và âm thầm mở rộng.
5. Chủ thể lease của người dùng: `human-grant:<session_id>` cho `session`, `human-grant:workspace` cho `always`. Authority tra lease này cho lượt do người dùng khởi tạo.
6. Hạn: `session` = 8 giờ và chỉ khớp đúng session_id. `always` = 90 ngày (lease được thiết kế có hạn, nên "luôn luôn" là 90 ngày, thu hồi được bằng `yana-rt lease revoke`). Cả hai là hằng số có tên.
7. Sàn không bị lease bỏ qua: HALT, availability, `narrow_by_intent` vẫn chạy trước/sau như hiện tại. Lease không áp dụng cho capability có `ApprovalRequirement` khác `HumanApprovalPerCall`.
8. Lệnh quá dài hoặc không tokenize được thì không cấp (fail-closed), nhận lỗi rõ ràng.
9. Mọi lần cấp ghi receipt, có `issued_by = decided_by`.

## Các bước
1. [Rust] `Lease.exact` + khớp chính xác + test.
2. [Rust] `PendingApprovalStore::resolve` nhận `scope`; cấp lease khi `decision=true` và scope khác once + test (biên: rỗng, 65.536 ký tự, byte NUL, CRLF, `../..`, scope lạ, deny kèm scope).
3. [Rust] `capability_decision` tra lease người dùng cho lượt `human_initiated`; test không vượt HALT/availability/intent.
4. [Rust] `HeadlessResumeInput.scope` (mặc định once) + test serde.
5. [Rust] review độc lập (security-auditor + code-auditor) vì đụng chuỗi authority.
6. [Swift] 3 nút trong trang duyệt + trang xem/thu hồi quyền đã cấp. Làm sau, cần repo `Yana-studio`.

## Đã sửa sau review độc lập (2 người review, chỉ đọc)
- Không gian tên `human-grant:` được giữ riêng: `LeaseStore::grant` từ chối, chỉ `grant_exact` cấp được; đường lease của agent bỏ qua chủ thể này. Có test bắt được lỗi (đã thử tắt dòng sửa, test đỏ).
- Cấp lease theo `resolved.context.session.repo_root`, cùng giá trị lúc khớp (trước đó dùng cwd, có thể lệch).
- Không cấp trùng lease; ghi log khi tra lease lỗi; từ chối nhớ shell và trình thông dịch (bash, sh, python*, node, env, sudo...); cảnh báo khi nhớ lệnh build/test vì chúng chạy mã trong thư mục dự án.

## Giới hạn đã biết, CHƯA giải quyết (cần người quyết)
1. `leases.json` là file thường. Một lệnh đã được duyệt (hoặc đã được nhớ) mà ghi file được thì có thể ghi một lease `human-grant:workspace` giả, và lease đó có hiệu lực với mọi lượt của người dùng. Ghi file qua tool của agent thì bị chặn (`file_mutation.rs` PROTECTED_CONFIGS), nhưng `run_command` thì không. Cách giảm: (B) đặt lease của người dùng ngoài repo, (C) ký bằng khóa trong Keychain chỉ yana-rt đọc được. Đến khi chọn, app KHÔNG nên đưa nút "session/always".
2. Một lệnh được nhớ chỉ ghim chữ của lệnh, không ghim thứ nó chạy (`cargo test` chạy mã mà agent có thể sửa).
3. `decided_by` và `scope` đến từ JSON stdin không xác thực; `issued_by` là chữ tự do.
4. Chưa gửi tín hiệu có cấu trúc cho app khi lời nhớ bị từ chối (hiện chỉ stderr).
5. `capability_decision` vẫn dài hơn giới hạn 50 dòng; `grant_for_decision` có 6 tham số (giới hạn 5). Chưa tách.

## Chưa biết
- `ChatCapabilityExecutor` có tự kiểm lại lệnh bằng guard riêng sau khi authority cho phép không (đọc trước bước 3).
- App Swift hiện dùng JSON nào để gửi quyết định (đọc `ChatEngine.decide`, dòng 286-296).
