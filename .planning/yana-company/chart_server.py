"""Local org-chart status server: serves index.html and /status.json, and nothing else on disk.
The JSON probes the repo (file existence, counts, references, one read-only `yana-rt harness audit`).
A level means "the code is present", never "it has been run end to end"."""
from __future__ import annotations

import argparse
import json
import logging
import shutil
import subprocess
import threading
import time
from dataclasses import asdict, dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

LOG = logging.getLogger("yana-chart")

REPO_ROOT = Path(__file__).resolve().parents[2]
STATIC = {"/": ("index.html", "text/html; charset=utf-8"), "/chart.js": ("chart.js", "text/javascript; charset=utf-8"),
          "/org.json": ("org.json", "application/json; charset=utf-8")}  # fixed names only, no path from the URL
# Loopback only: this page shows repository internals and must not be reachable from the network.
BIND_ADDRESS = "127.0.0.1"
DEFAULT_PORT = 8765  # arbitrary unprivileged port, override with --port
CACHE_TTL_S = 60  # probing reads many files; a refresh within a minute reuses the last result
AUDIT_TIMEOUT_S = 90  # the audit scans about 4,000 skill files; leave headroom on a slow disk
MAX_SCAN_BYTES = 2_000_000  # skip files larger than this when reading or searching
SCAN_SUFFIXES = {".rs", ".sh", ".js", ".py", ".json", ".toml"}
REFERENCE_ROOTS = ("src", "core/hooks", "core/scripts", ".claude/settings.json")
LEVELS = ("co", "mot_phan", "chua")


@dataclass(frozen=True)
class Check:
    label: str
    ok: bool
    detail: str = ""


class Repo:
    """Read-only probes over the repository. Every probe fails closed (False, 0, empty)."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self._settings = self._read(root / ".claude/settings.json")

    @staticmethod
    def _read(path: Path) -> str:
        try:
            if path.stat().st_size > MAX_SCAN_BYTES:
                return ""
            return path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            return ""

    def has(self, *rels: str) -> bool:
        return any((self.root / rel).exists() for rel in rels)

    def line_count(self, rel: str) -> int:
        return self._read(self.root / rel).count("\n")

    def contains(self, rel: str, needle: str) -> bool:
        return needle in self._read(self.root / rel)

    def wired(self, hook: str) -> bool:
        return hook in self._settings

    def count_files(self, rel: str, pattern: str) -> int:
        base = self.root / rel
        return sum(1 for _ in base.rglob(pattern)) if base.is_dir() else 0

    def referenced_by(self, needle: str, roots: tuple[str, ...]) -> list[str]:
        hits: list[str] = []
        for rel in roots:
            base = self.root / rel
            if base.is_file():
                files = [base]
            elif base.is_dir():
                files = [p for p in base.rglob("*") if p.is_file()]
            else:
                files = []
            for path in files:
                if path.suffix in SCAN_SUFFIXES and needle in self._read(path):
                    hits.append(str(path.relative_to(self.root)))
        return hits


def make_node(node_id: str, title: str, place: tuple[str, str], checks: list[Check],
              tags: tuple[str, ...] = ()) -> dict:
    """place is (group, state); level is derived from how many requirements are met."""
    passed = sum(1 for c in checks if c.ok)
    level = "co" if checks and passed == len(checks) else "mot_phan" if passed else "chua"
    return {"id": node_id, "title": title, "group": place[0], "state": place[1], "level": level,
            "tags": list(tags), "checks": [asdict(c) for c in checks]}


def run_harness_audit(root: Path) -> dict[str, int] | None:
    """Exit code 1 only means 'errors were found', which is a normal result here."""
    binary = shutil.which("yana-rt")
    if binary is None:
        LOG.warning("yana-rt is not on PATH; live harness audit skipped")
        return None
    try:
        done = subprocess.run([binary, "harness", "audit", "--root", str(root), "--json"],
                              capture_output=True, text=True, timeout=AUDIT_TIMEOUT_S, check=False)
        data = json.loads(done.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError, OSError) as exc:
        LOG.warning("harness audit failed: %s", exc)
        return None
    return {key: int(data.get(key, 0)) for key in ("agents", "skills", "errors", "warnings")}


def top_nodes(r: Repo) -> list[dict]:
    return [
        make_node("chu_tich", "Chủ tịch", ("top", "quyet"), [
            Check("Duyệt của người thật trước việc có hậu quả", r.has("src/runtime/pending_approval.rs"),
                  "src/runtime/pending_approval.rs"),
            Check("Nút dừng cả công ty", False, "chưa có"),
            Check("Chức vụ và mã số Chủ tịch", False, "chưa có thẻ nhân sự")]),
        make_node("hoi_dong", "Hội đồng quản trị", ("top", "quyet"), [
            Check("Ghi quyết định không sửa được", r.has(".claude/state/audit-chain.log"),
                  "sổ băm .claude/state/audit-chain.log"),
            Check("Đặt mục tiêu và ngân sách cho công ty", False, "chưa có"),
            Check("Quyết định tập thể nhiều bên", False, "chưa có")]),
    ]


def exec_nodes(r: Repo) -> list[dict]:
    mission = "src/mission/mod.rs"
    agents = r.count_files("core/agents", "*.md")
    return [
        make_node("ceo", "Tổng giám đốc (CEO)", ("exec", "quyet"), [
            Check("Chia mục tiêu thành việc cụ thể", r.has(mission), mission),
            Check("Chức vụ CEO có mã số", False, "chưa có")]),
        make_node("giam_doc", "Giám đốc từng mảng", ("exec", "quyet"), [
            Check("Giao việc kèm agent gợi ý", r.contains(mission, "agent"), "tham số agent của mission task"),
            Check("Phòng ban và mã số", False, "chưa có khái niệm phòng")]),
        make_node("dieu_hanh", "Người điều hành", ("exec", "quyet"), [
            Check("Việc phụ thuộc nhau", r.contains("src/task.rs", "depend"), "src/task.rs"),
            Check("Hợp đồng việc khai file tạo ra và cần", r.contains(mission, "produces"), mission),
            Check("Hạn quyền gắn vào hợp đồng việc", False, "chưa có")]),
        make_node("nhan_vien", "Nhân viên (thiết kế, lập trình, nghiên cứu...)", ("exec", "quyet"), [
            Check("File mô tả vai trò agent", agents > 0, f"{agents} file trong core/agents"),
            Check("Mã số, phòng, bậc cho từng agent", False, "chưa có"),
            Check("Thống kê kết quả từng agent", False, "chưa có")]),
    ]


def control_nodes(r: Repo, audit: dict[str, int] | None) -> list[dict]:
    clean = audit is not None and audit["errors"] == 0
    live = (f"{audit['agents']} agent, {audit['skills']} skill, {audit['errors']} lỗi, "
            f"{audit['warnings']} cảnh báo" if audit else "không chạy được yana-rt harness audit")
    ledger = r.has("core/memory/trust-ledger.js")
    refs = r.referenced_by("trust-ledger", REFERENCE_ROOTS)
    token_hook = "core/hooks/token-budget-guard.sh"
    return [
        make_node("giam_thi", "Giám thị / kiểm thử (QA)", ("control", "quyet"), [
            Check("Lệnh kiểm tra agent và skill", r.has("src/harness/audit.rs"), "yana-rt harness audit"),
            Check("Kiểm tra sạch lỗi", clean, live),
            Check("Giám thị đến từ phòng khác", False, "chưa có khái niệm phòng")]),
        make_node("bao_ve", "Bảo vệ cổng", ("control", "quyet"), [
            Check("Lõi chặn lệnh nguy hiểm", r.has("src/guard"), "src/guard"),
            Check("Nối vào hook của Claude Code", r.wired("guard-destructive.sh"), ".claude/settings.json"),
            Check("Luật coi nội dung ngoài là dữ liệu", r.has(".claude/rules/prompt-jailbreak-guard.md"),
                  "prompt-jailbreak-guard.md"),
            Check("Chặn thật được trong đội nhiều agent", False, "chưa kiểm")]),
        make_node("cfo", "Giám đốc tài chính (CFO)", ("control", "de_xuat"), [
            Check("Ngân sách theo phòng", False, "chưa có"),
            Check("Báo cáo thẳng lên hội đồng", False, "chưa có")]),
        make_node("ke_toan", "Kế toán", ("control", "quyet"), [
            Check("Mã tính chi phí", r.has("src/cost.rs"), f"src/cost.rs, {r.line_count('src/cost.rs')} dòng"),
            Check("Sổ chi theo từng agent", False, "chưa có")]),
        make_node("token", "Kiểm soát token", ("control", "quyet"), [
            Check("Hook giữ ngân sách token", r.has(token_hook), token_hook),
            Check("Hook đã nối vào settings", r.wired("token-budget-guard.sh"), ".claude/settings.json"),
            Check("Trần theo từng agent", False, "chưa có"),
            Check("Đóng băng ngân sách của phòng", False, "chưa có")]),
        make_node("kiem_toan", "Kiểm toán nội bộ", ("control", "de_xuat"), [
            Check("Sổ băm ghi mọi lần gọi công cụ", r.has(".claude/state/audit-chain.log"),
                  f"{r.line_count('.claude/state/audit-chain.log')} dòng"),
            Check("Script xác minh chuỗi", r.has("core/scripts/verify-audit-chain.sh"), "verify-audit-chain.sh"),
            Check("Độc lập với tài chính", False, "chưa có")]),
        make_node("phap_che", "Pháp chế và tuân thủ", ("control", "de_xuat"), [
            Check("Bất kỳ phần nào", False, "chưa có")]),
        make_node("nhan_su", "Nhân sự agent (thẻ, khen phạt)", ("control", "de_xuat"), [
            Check("Sổ điểm tin cậy", ledger, "core/memory/trust-ledger.js"),
            Check("Có nơi nào ghi vào sổ", bool(refs), ", ".join(refs) or "không nơi nào gọi"),
            Check("Khen thưởng, thăng giáng bậc", False, "chưa có")],
            ("Có file nhưng không nơi nào gọi",) if ledger and not refs else ()),
    ]


def shared_nodes(r: Repo) -> list[dict]:
    receipts = ".yana-ai/authority-receipts.jsonl"
    lease = r.has("src/capability/lease.rs", "src/capability/lease/mod.rs")
    breaker = "core/hooks/per-tool-circuit-breaker.sh"
    return [
        make_node("thu_ky", "Thư ký (biên bản họp)", ("office", "de_xuat"), [
            Check("Gửi nhận tin giữa agent", r.has("src/bus.rs"), "src/bus.rs"),
            Check("Biên bản họp và quyết định", False, "chưa có")]),
        make_node("the", "Thẻ nhân sự và mã số", ("shared", "quyet"), [
            Check("Mã số cho từng agent", False, "chưa có"),
            Check("Chức vụ, phòng, bậc", False, "chưa có")]),
        make_node("bien_nhan", "Sổ biên nhận quyền", ("shared", "de_xuat"), [
            Check("Mã ghi biên nhận", r.has("src/runtime/receipt.rs"), "src/runtime/receipt.rs"),
            Check("File biên nhận có dữ liệu", r.has(receipts), f"{r.line_count(receipts)} dòng"),
            Check("Kho dùng chung nhiều bên ghi", False, "hiện là file cục bộ")]),
        make_node("han_quyen", "Hạn quyền (lease)", ("shared", "de_xuat"), [
            Check("Cấp quyền có hạn", lease, "src/capability/lease"),
            Check("Chỉ được thu hẹp theo ý định", r.contains("src/runtime/authority.rs", "narrow_by_intent"),
                  "narrow_by_intent"),
            Check("Cấp theo mã agent", False, "chưa có")]),
        make_node("ngan_sach", "Ngân sách và biên chế", ("shared", "quyet"), [
            Check("Đo chi phí", r.has("src/cost.rs"), "src/cost.rs"),
            Check("Ngân sách từng phòng, định biên", False, "chưa có"),
            Check("Chấm hiệu suất phòng", False, "chưa có")]),
        make_node("thong_ke", "Bảng thống kê phòng", ("shared", "quyet"), [
            Check("Nghiệm thu việc có bằng chứng", r.contains("src/task.rs", "evidence"), "task done --evidence"),
            Check("Thống kê đúng sai từng agent", False, "chưa có"),
            Check("Bảng xem theo phòng", False, "chưa có")]),
        make_node("discord", "Kênh Discord", ("shared", "quyet"), [
            Check("Mã kênh Discord", r.has("src/remote/discord.rs"), "src/remote/discord.rs"),
            Check("Tính năng discord trong Cargo", r.contains("Cargo.toml", "discord"), "Cargo.toml"),
            Check("Đã chạy thử với đội agent", False, "chưa kiểm")]),
        make_node("nut_dung", "Cầu dao và nút dừng", ("shared", "de_xuat"), [
            Check("Cầu dao theo công cụ", r.has(breaker), breaker),
            Check("Đã nối vào settings", r.wired("per-tool-circuit-breaker.sh"), ".claude/settings.json"),
            Check("Dừng theo phòng hoặc cả công ty", False, "chưa có")]),
    ]


_cache: dict = {"at": 0.0, "data": None}
_lock = threading.Lock()


def snapshot() -> dict:
    with _lock:
        if _cache["data"] is not None and time.monotonic() - _cache["at"] < CACHE_TTL_S:
            return _cache["data"]
        repo, audit = Repo(REPO_ROOT), run_harness_audit(REPO_ROOT)
        nodes = top_nodes(repo) + exec_nodes(repo) + control_nodes(repo, audit) + shared_nodes(repo)
        data = {"generated_at": time.strftime("%Y-%m-%d %H:%M:%S"), "repo": REPO_ROOT.name,
                "audit": audit, "nodes": nodes,
                "counts": {lv: sum(1 for n in nodes if n["level"] == lv) for lv in LEVELS}}
        _cache.update(at=time.monotonic(), data=data)
        return data


class Handler(BaseHTTPRequestHandler):
    server_version = "YanaChart"

    def do_GET(self) -> None:  # noqa: N802 - name fixed by BaseHTTPRequestHandler
        route = self.path.split("?", 1)[0]
        try:
            if route in STATIC:
                name, kind = STATIC[route]
                self._send(200, kind, Path(__file__).with_name(name).read_bytes())
            elif route == "/status.json":
                body = json.dumps(snapshot(), ensure_ascii=False).encode("utf-8")
                self._send(200, "application/json; charset=utf-8", body)
            else:
                self._send(404, "text/plain; charset=utf-8", b"not found")
        except OSError as exc:
            LOG.error("request %s failed: %s", route, exc)
            self._send(500, "text/plain; charset=utf-8", b"server error")

    def _send(self, code: int, kind: str, body: bytes) -> None:
        self.send_response(code)
        self.send_header("Content-Type", kind)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Content-Security-Policy",
                         "default-src 'none'; style-src 'unsafe-inline'; script-src 'self'; "
                         "connect-src 'self'")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt: str, *args: object) -> None:
        LOG.info("%s", fmt % args)


def main() -> None:
    parser = argparse.ArgumentParser(description="Yana company org-chart status page (localhost only)")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    port = parser.parse_args().port
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
    server = ThreadingHTTPServer((BIND_ADDRESS, port), Handler)
    LOG.info("serving the org chart on the loopback interface, port %d (Ctrl+C to stop)", port)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        LOG.info("stopped")
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
