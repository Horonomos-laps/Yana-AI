# Cầu nối Yana: bản thiết kế

Trạng thái (2026-11-26): anh Tâm đã bảo bắt đầu theo các đề xuất ở mục 9. Đã có giao thức, bắt tay
phiên bản và máy chủ socket chuyển gói. Chưa có kho chat, luồng control, bộ bắt trong app, soát bảo mật.

## 1. Mục tiêu

Mọi app của Yana (app chat Swift, notch, Yana Studio, sau này web, desktop, công cụ ngoài) cắm vào
**một cầu nối chung** viết bằng Rust trong `yana-rt`, để tính năng chạy đồng bộ giữa các app.

Ý đã chốt với anh:

1. Một cầu chung, không nối riêng từng cặp.
2. **Không có hàng chờ chung.** Việc này không phải đợi việc kia.
3. Mỗi đầu cầu có **bộ bắt hoạt động** để thấy thay đổi và đẩy đi ngay.
4. Các app biết phiên bản và khả năng của nhau.

## 2. Hình dạng

```
App chat ── bộ bắt ─┐
Notch ───── bộ bắt ─┤
Yana Studio ─ bộ bắt ┼── yana-rt bridge ── kho chung (phiên chat, cài đặt, trạng thái)
Web, desktop (sau) ─┤        │
Claude Code, Codex ─┘        └── Authority của runtime (duyệt lệnh, biên nhận)
```

| Phần | Làm | Không làm |
|---|---|---|
| Bộ bắt (trong từng app) | Thấy thay đổi, đóng gói phần vừa đổi, áp gói nhận về | Không gửi thẳng sang app khác |
| Cầu nối | Xác thực, phân luồng, chuyển ngay, giữ trạng thái mới nhất | Không tự quyết định duyệt lệnh |
| Kho chung | Nguồn sự thật cho dữ liệu được đồng bộ | Không chứa API key |

App vẫn chat được khi cầu tắt, bằng cách gọi `yana-rt chat --headless` như hiện nay.

## 3. Luồng, không hàng chờ chung

Mỗi kết nối mang nhiều luồng độc lập. Cầu xử lý các luồng song song.

| Luồng | Nội dung | Khi bên nhận chậm | Thứ tự |
|---|---|---|---|
| `control` | dừng agent, quyết định duyệt | không được bỏ, báo lỗi ngay cho bên gửi | theo từng lệnh |
| `chat` | tin nhắn, chữ đang trả lời | gộp các mẩu liền nhau | theo từng cuộc chat |
| `state` | agent đang chạy, tiến độ | chỉ giữ giá trị mới nhất | không cần |
| `settings` | model, dự án, bộ nhớ, chỉ dẫn | chỉ giữ bản mới nhất của từng mục | theo từng mục |
| `system` | phiên bản `yana-rt`, app vào và ra | chỉ giữ giá trị mới nhất | không cần |

Giới hạn phải nói rõ: bộ đệm của một kết nối vẫn có kích thước hữu hạn. Cầu không dồn vô hạn.
Khi đầy, `state`, `settings`, `system` giữ bản mới nhất, `chat` gộp rồi nếu vẫn đầy thì ngắt kết nối
để bên đó lấy lại từ kho, `control` trả lỗi.

**Mất kết nối:** không phát lại hàng đống gói cũ. App gửi số phiên bản đã có của từng mục, cầu trả phần còn thiếu.

**Chống dội ngược:** mỗi gói mang `origin`. Bộ bắt không phát lại thay đổi vừa áp từ cầu.

**Hai bên cùng sửa:** mỗi mục có số phiên bản. Ghi kèm phiên bản cũ thì bị từ chối, app tải bản mới rồi
hỏi người dùng nếu khác nhau. Chat chỉ nối thêm nên không đè nhau; hai app không được cùng chạy một lượt
trong một cuộc chat (dùng khóa phiên có sẵn trong `src/session_db/`).

## 4. Giao thức

- Vận chuyển: Unix socket trong thư mục chỉ chủ máy đọc được. Không mở cổng mạng.
- Mỗi gói một dòng JSON, giới hạn 1 MiB như giao thức headless đang dùng. Nội dung lớn gửi bằng tham chiếu tới kho.
- Vỏ gói: `v` (phiên bản giao thức), `id`, `stream`, `type`, `origin`, `object`, `rev`, `body`.
- Bắt tay: app gửi tên, phiên bản app, phiên bản giao thức nó hiểu, khả năng, thẻ nhận diện.
  Cầu trả phiên bản `yana-rt`, phiên bản giao thức dùng chung và quyền được cấp.
- Tương thích: trường lạ thì bỏ qua; loại gói lạ trên `control` thì từ chối rõ ràng; không có phiên bản
  chung thì ngắt và báo app nào cần cập nhật.

## 5. Bảo mật

Cầu chuyển được quyết định duyệt lệnh, nên đây là phần rủi ro nhất.

| Mối lo | Cách chặn |
|---|---|
| Chương trình lạ trên máy cắm vào | Kiểm người dùng của tiến trình bên kia socket **và** thẻ riêng cho từng app, cấp khi người dùng bấm kết nối |
| App được cắm làm quá quyền | Quyền theo luồng: ví dụ công cụ ngoài chỉ được gửi `state` |
| Giả quyết định duyệt | Cầu không tạo phép duyệt. Nó chuyển quyết định cho Authority của runtime, nơi kiểm và ghi biên nhận |
| Lộ API key | Key không đi qua cầu và không nằm trong kho chung |
| Nội dung chứa lệnh | Dữ liệu nhận từ app khác chỉ để hiển thị, không bao giờ được chạy |

Giới hạn đã biết: kiểm người dùng của socket chỉ chứng minh cùng tài khoản, không chứng minh đúng app.
Các app hiện ký tạm (ad-hoc) nên chưa dựa được vào chữ ký. Thẻ riêng nằm trong Keychain làm khó hơn nhưng
không chặn được mã độc đã chạy dưới tài khoản của người dùng. Phần này cần hai người soát theo luật của repo.

## 6. Kiểm kê (đọc mã ngày 2026-11-26)

| Dữ liệu | App chat và notch | Yana Studio | Đồng bộ | Chủ | Đợt |
|---|---|---|---|---|---|
| Lịch sử chat | `Application Support/YanaApp/chats` | `workspaceState` `yana.conversations.v1` | Có | kho chung | 1 |
| Model, provider, base URL | UserDefaults `yana.model`, `yana.provider`, `yana.baseUrl` | cài đặt cùng tên | Có | kho chung | 2 |
| Provider tự thêm | `yana.customProviders` | `yana.studio.settings.v1` (chưa đọc kỹ) | Có | kho chung | 2 |
| Nhiệt độ, số token tối đa | `yana.chat.*` | `yana.chat.*` | Có | kho chung | 2 |
| Ngôn ngữ | `yana.language` | `yana.language` | Có | kho chung | 2 |
| Dự án | `yana.projects` | theo thư mục mở | Cần bàn | chưa rõ | 2 |
| Bộ nhớ, cá nhân hóa | `YanaApp/memory`, `yana.personal.*` | Project Memory | Cần bàn, hai khái niệm khác nhau | chưa rõ | 2 |
| Số liệu sử dụng | `ledger.jsonl` | `yana.usage.v1` | Có | runtime | 2 |
| Chờ duyệt, biên nhận | `pending-approvals.json`, `authority-receipts.jsonl` | cùng runtime | Có | runtime | 3 |
| Agent đang chạy (notch) | hook ghi vào `YanaApp/Agents`, đọc log Claude Code | trạng thái trong extension | Có | cầu | 3 |
| Nhờ IDE chạy việc | ghi `ide-request.json` | `app-bridge.js` đọc | Thay bằng luồng `control` | cầu | 3 |
| Phiên bản `yana-rt` | kiểm lúc mở và sau khi tự cài | kiểm sau 10 giây và khi bấm | Có | cầu | 1 |
| API key | Keychain `ai.yana.app.provider-key` | SecretStorage | **Không qua cầu** | bàn riêng | 4 |
| Plugin, MCP | `.plugin.json`, `.mcp-servers.json` | chưa kiểm | Cần kiểm | chưa rõ | sau |
| Nhạc, hẹn giờ, việc cần làm, ghi chú, clipboard (notch) | `yana.notch.*` | không có | Không, clipboard không bao giờ qua cầu | app chat | không |
| Thiết kế canvas | không có | `yana.studio.designs.v1` | Không | Studio | không |
| Giao diện, hình nền, phím tắt | riêng | riêng | Không (hỏi lại anh nếu muốn chung theme) | từng app | không |

## 7. Phần có sẵn để dùng lại

| Có sẵn | Dùng cho | Đã kiểm tới đâu |
|---|---|---|
| `src/session_db/` (SQLite, tìm kiếm, khóa phiên, nhập lịch sử) | kho chat | đọc hợp đồng `docs/contracts/ws4-state.md`; bản sao lịch sử đang tắt mặc định |
| `tokio` (phụ thuộc tùy chọn trong `Cargo.toml`) | chạy các luồng song song | mới thấy khai báo |
| `src/runtime/pending_approval.rs` | duyệt lệnh qua nhiều tiến trình | đọc phần mô tả đầu file |
| `src/connector.rs` (danh sách quyền) | cấp quyền cho app cắm vào | đọc phần mô tả đầu file |
| `src/mcp.rs` | công cụ ngoài; MCP hiện không cho duyệt lệnh mức Critical | đọc phần mô tả đầu file |
| `src/bus.rs` | chưa rõ | chưa đọc |

Chưa thấy mã dùng Unix socket trong `src/`. Phần socket là viết mới. Windows cần cách khác, để sau.

## 8. Các đợt và điều kiện xong

| Đợt | Làm | Xong khi |
|---|---|---|
| 0 | Sửa việc Studio chép `target/release/yana-rt` của thư mục đang mở đè lên bản chung | Có test: mở thư mục có binary đó thì bản chung không đổi |
| 1 | Lõi cầu, bắt tay, phiên bản, luồng `system` và `chat`, kho chat chung, nhập chat cũ | Chat ở app này thấy ở app kia; chat cũ còn nguyên file gốc; tắt cầu app vẫn chat |
| 2 | Luồng `settings`, số liệu sử dụng | Đổi model một nơi, nơi kia đổi; sửa đồng thời không mất dữ liệu |
| 3 | `control`, `state`, duyệt lệnh, notch | Thấy và duyệt việc của Studio từ notch; app không đủ quyền bị từ chối (có test) |
| 4 | API key dùng chung | Thiết kế riêng, duyệt riêng |

Mục tiêu đo, **là đề xuất, chưa đo gì:** gói `control` đến bên nhận dưới 50 ms ở phân vị 95 trong khi
luồng `chat` đang đẩy 1 MiB mỗi giây. Test tải này chạy từ đợt 1.

## 9. Câu hỏi cần anh quyết

1. Kho chung đặt theo người dùng (đề xuất) hay theo từng thư mục dự án?
2. Cầu tự chạy khi app đầu tiên mở và tự tắt khi không còn app (đề xuất), hay chạy cùng máy?
3. "Dự án" và "bộ nhớ" của hai app: gộp làm một khái niệm, hay chỉ đồng bộ phần giống nhau?

## 10. Chưa kiểm

- Chưa chạy hai app cùng lúc để quan sát hành vi thật.
- Bảng kiểm kê dựa trên tên file, khóa lưu và vài file đọc trực tiếp; có thể thiếu tính năng.
- Bản Yana 1.0 đang phát hành khác mã `main`; kiểm kê này đọc mã `main`.
- Chưa rà phần plugin, MCP, canvas, và `src/bus.rs`.
