'use strict';
// Draws the org chart from /org.json and colours each role from /status.json (measured probes).
// Every role shows one seat per agent it may hold: filled seats are the pilot headcount, the rest are vacant.
const SVG_NS = 'http://www.w3.org/2000/svg';
const LEVEL = {co: 'Có nền', mot_phan: 'Một phần', chua: 'Chưa có'};
const GLYPH = {co: '✔', mot_phan: '◐', chua: '✕'};
const STATE = {quyet: 'Anh đã quyết', de_xuat: 'Em đề xuất'};
const LINE = {1: 'làm việc và tự kiểm', 2: 'pháp chế, an ninh, rủi ro', 3: 'kiểm toán độc lập'};
const BOX_W = 250;        // width of a tree node
const BOX_H = 86;         // height of a tree node (title, sub line, one row of seats)
const STACK_W = 230;      // width of a role listed inside a department
const STACK_BASE = 52;    // height of a listed role before its seat rows
const STACK_GAP = 10;     // vertical gap between listed roles
const STACK_TOP = 18;     // gap between a department and its first listed role
const SEAT = 22;          // seat square size
const SEAT_PITCH = 26;    // seat square plus spacing
const GAP_X = 28;         // horizontal gap between sibling subtrees
const LEVEL_Y = 132;      // vertical distance between tree levels
const PAD = 16;           // outer margin of the drawing
const DESIGN_ONLY = {label: 'Chức vụ này có trong mã Yana', ok: false, detail: 'chưa có, mới ở mức thiết kế'};
let seatCounters = {};

function s(tag, attrs, text) {
  const node = document.createElementNS(SVG_NS, tag);
  Object.entries(attrs).forEach(([key, value]) => node.setAttribute(key, value));
  if (text !== undefined) node.textContent = text;
  return node;
}

function el(tag, cls, text) {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
}

function kids(node) { return node.children || []; }
function perRow(width) { return Math.floor((width - 24) / SEAT_PITCH); }
function seatRows(node, width) { return node.hc ? Math.ceil(node.hc[1] / perRow(width)) : 0; }
function stackHeight(child) { return STACK_BASE + seatRows(child, STACK_W) * SEAT_PITCH; }

function measure(node) {
  if (node.stack || !kids(node).length) {
    node.w = BOX_W;
  } else {
    const total = kids(node).reduce((sum, child) => sum + measure(child), 0) + GAP_X * (kids(node).length - 1);
    node.w = Math.max(BOX_W, total);
  }
  return node.w;
}

function place(node, left, depth, out) {
  Object.assign(node, {x: left + (node.w - BOX_W) / 2, y: PAD + depth * LEVEL_Y, bw: BOX_W, bh: BOX_H});
  out.push(node);
  if (node.stack) {
    let y = node.y + BOX_H + STACK_TOP;
    kids(node).forEach((child) => {
      Object.assign(child, {x: node.x + 20, y, bw: STACK_W, bh: stackHeight(child), stacked: true});
      out.push(child);
      y += child.bh + STACK_GAP;
    });
    return;
  }
  const span = kids(node).reduce((sum, child) => sum + child.w, 0) + GAP_X * Math.max(0, kids(node).length - 1);
  let cursor = left + (node.w - span) / 2;
  kids(node).forEach((child) => { place(child, cursor, depth + 1, out); cursor += child.w + GAP_X; });
}

function edges(node, paths) {
  if (node.stack) {
    const trunk = node.x + 10;
    const last = kids(node)[kids(node).length - 1];
    paths.push(`M${trunk} ${node.y + BOX_H}V${last.y + 22}`);
    kids(node).forEach((child) => paths.push(`M${trunk} ${child.y + 22}H${child.x}`));
    return;
  }
  if (!kids(node).length) return;
  const midY = node.y + BOX_H + (LEVEL_Y - BOX_H) / 2;
  const centers = kids(node).map((child) => child.x + BOX_W / 2);
  paths.push(`M${node.x + BOX_W / 2} ${node.y + BOX_H}V${midY}`);
  paths.push(`M${Math.min(...centers)} ${midY}H${Math.max(...centers)}`);
  centers.forEach((cx) => paths.push(`M${cx} ${midY}V${midY + (LEVEL_Y - BOX_H) / 2}`));
  kids(node).forEach((child) => edges(child, paths));
}

function resolve(node, probes) {
  if (node.kind === 'group' || node.kind === 'dept') return {level: null, checks: [], tags: []};
  const probe = node.probe && probes[node.probe];
  return probe ? probe : {level: 'chua', checks: [DESIGN_ONLY], tags: []};
}

function assignSeats(node) {
  if (!node.hc) return;
  const start = seatCounters[node.code] || 0;
  seatCounters[node.code] = start + node.hc[1];
  node.seats = Array.from({length: node.hc[1]}, (_, i) => ({
    id: `${node.code}-${String(start + i + 1).padStart(4, '0')}`, filled: i < node.hc[0]}));
}

function drawSeats(g, node) {
  const per = perRow(node.bw);
  node.seats.forEach((seat, i) => {
    const x = node.x + 12 + (i % per) * SEAT_PITCH;
    const y = node.y + 50 + Math.floor(i / per) * SEAT_PITCH;
    const box = s('g', {class: seat.filled ? 'seat filled' : 'seat vacant'});
    box.appendChild(s('title', {}, `${seat.id} · ${seat.filled ? 'có agent (chạy thử)' : 'ghế trống'}`));
    box.appendChild(s('rect', {x, y, width: SEAT, height: SEAT, rx: 4}));
    box.appendChild(s('text', {x: x + SEAT / 2, y: y + 16, 'text-anchor': 'middle'}, String(i + 1)));
    g.appendChild(box);
  });
}

function drawNode(node, probes) {
  const info = resolve(node, probes);
  const kind = node.kind ? ' ' + node.kind : '';
  const g = s('g', {class: `node${kind}${info.level ? ' lv-' + info.level : ''}`, tabindex: 0, role: 'button',
    'aria-label': node.title + (info.level ? ', ' + LEVEL[info.level] : '')});
  g.appendChild(s('rect', {x: node.x, y: node.y, width: node.bw, height: node.bh, rx: 8}));
  const prefix = info.level ? GLYPH[info.level] + ' ' : '';
  g.appendChild(s('text', {x: node.x + 12, y: node.y + 22}, prefix + node.title + (info.tags.length ? ' ⚠' : '')));
  if (node.hc) g.appendChild(s('text', {class: 'sub', x: node.x + node.bw - 10, y: node.y + 22, 'text-anchor': 'end'},
    `${node.hc[0]}/${node.hc[1]}`));
  g.appendChild(s('text', {class: 'sub', x: node.x + 12, y: node.y + 42}, [node.sub, node.code].filter(Boolean).join(' · ')));
  if (node.seats) drawSeats(g, node);
  const open = () => showDetail(node, info);
  g.addEventListener('click', open);
  g.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); open(); }
  });
  return g;
}

function showDetail(node, info) {
  const box = document.getElementById('detail');
  const hc = node.hc && `định biên: chạy thử ${node.hc[0]}, tối đa ${node.hc[1]} agent (số tạm)`;
  const line = node.line && `tuyến ${node.line}: ${LINE[node.line]}`;
  const meta = [node.code, info.level && LEVEL[info.level], node.state && STATE[node.state], hc, line]
    .filter(Boolean).join(' · ');
  box.replaceChildren(el('h3', '', node.title + (node.sub ? ` (${node.sub})` : '')), el('p', 'muted small', meta));
  if (node.note) box.appendChild(el('p', 'small', node.note));
  if (node.seats) {
    const filled = node.seats.filter((seat) => seat.filled).map((seat) => seat.id);
    box.appendChild(el('p', 'small', filled.length ? 'Ghế có agent: ' + filled.join(', ') : 'Chưa có ghế nào có agent.'));
  }
  info.tags.forEach((tag) => box.appendChild(el('p', 'small', '⚠ ' + tag)));
  const list = el('ul', 'checks');
  info.checks.forEach((check) => {
    const item = el('li', check.ok ? 'ok' : 'no');
    item.appendChild(el('span', 'mark', check.ok ? '✔' : '✘'));
    item.appendChild(el('span', '', check.label));
    if (check.detail) item.appendChild(el('div', 'detail-line', check.detail));
    list.appendChild(item);
  });
  if (info.checks.length) box.appendChild(list);
  if (typeof box.showPopover !== 'function') { box.style.display = 'block'; return; }
  if (!box.matches(':popover-open')) box.showPopover();
}

function renderChart(org, probes) {
  seatCounters = {};
  measure(org);
  const nodes = [];
  place(org, PAD, 0, nodes);
  nodes.forEach(assignSeats);
  const paths = [];
  edges(org, paths);
  const width = org.w + PAD * 2;
  const height = Math.max(...nodes.map((n) => n.y + n.bh)) + PAD;
  const svg = s('svg', {viewBox: `0 0 ${width} ${height}`, width, height, role: 'group',
    'aria-label': 'Sơ đồ tổ chức công ty Yana'});
  paths.forEach((d) => svg.appendChild(s('path', {class: 'edge', d})));
  nodes.forEach((node) => svg.appendChild(drawNode(node, probes)));
  document.getElementById('chart').replaceChildren(svg);
  return nodes;
}

function renderShared(statusNodes, probes) {
  const wrap = document.getElementById('shared');
  wrap.replaceChildren();
  statusNodes.filter((n) => n.group === 'shared').forEach((n) => {
    const button = el('button', 'chipcard lv-' + n.level, GLYPH[n.level] + ' ' + n.title);
    button.type = 'button';
    button.addEventListener('click', () => showDetail({title: n.title, state: n.state}, probes[n.id]));
    wrap.appendChild(button);
  });
}

function renderBar(roles, levelOf, audit, at) {
  const bar = document.getElementById('bar');
  bar.replaceChildren();
  ['co', 'mot_phan', 'chua'].forEach((lv) => {
    const count = roles.filter((n) => levelOf(n) === lv).length;
    bar.appendChild(el('span', 'pill', `${GLYPH[lv]} ${count} / ${roles.length} chức vụ ${LEVEL[lv]}`));
  });
  const seats = roles.flatMap((n) => n.seats || []);
  const filled = seats.filter((seat) => seat.filled).length;
  bar.appendChild(el('span', 'pill', `${seats.length} ghế: ${filled} có agent, ${seats.length - filled} trống (số tạm)`));
  bar.appendChild(el('span', 'pill', audit
    ? `Kiểm tra agent và skill (trực tiếp): ${audit.errors} lỗi, ${audit.warnings} cảnh báo`
    : 'Kiểm tra agent và skill: không chạy được'));
  bar.appendChild(el('span', 'muted small', 'Đo lúc ' + at));
  const refresh = el('button', 'refresh', 'Làm mới');
  refresh.type = 'button';
  refresh.addEventListener('click', load);
  bar.appendChild(refresh);
}

async function load() {
  try {
    const [orgRes, statusRes] = await Promise.all([fetch('/org.json', {cache: 'no-store'}),
      fetch('/status.json', {cache: 'no-store'})]);
    if (!orgRes.ok || !statusRes.ok) throw new Error(`HTTP ${orgRes.status}/${statusRes.status}`);
    const [org, status] = [await orgRes.json(), await statusRes.json()];
    const probes = Object.fromEntries(status.nodes.map((n) => [n.id, n]));
    const nodes = renderChart(org, probes);
    const roles = nodes.filter((n) => n.kind !== 'group' && n.kind !== 'dept');
    renderBar(roles, (n) => resolve(n, probes).level, status.audit, status.generated_at);
    renderShared(status.nodes, probes);
  } catch (error) {
    document.getElementById('chart').textContent = 'Không tải được sơ đồ: ' + error.message;
  }
}
load();
