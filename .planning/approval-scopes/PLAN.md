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
4. Phạm vi khớp LỆNH CHÍNH XÁC (so khớp từng token), không khớp tiền tố. Lý do: tiền tố của "git push origin main" khớp cả "git push origin main --force". Thêm cờ `exact` vào `Lease` (serde default false, tương thích ngược).
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

## Chưa biết
- `ChatCapabilityExecutor` có tự kiểm lại lệnh bằng guard riêng sau khi authority cho phép không (đọc trước bước 3).
- App Swift hiện dùng JSON nào để gửi quyết định (đọc `ChatEngine.decide`, dòng 286-296).
