// Apeireth Web 面板 v2 — 共享脚本 (B1; 原生 JS, 无构建链/无框架)
// 数据全部来自 /v1/panel/* 只读端点 (apeireth-api panel_readonly.rs)

// 面板顶栏 (当前页高亮由 body[data-page] 决定)
function panelHeader(active) {
  const items = [
    ["index.html", "总览", "index"],
    ["sessions.html", "会话", "sessions"],
    ["memory.html", "记忆", "memory"],
    ["graph.html", "图谱", "graph"],
    ["approvals.html", "授权", "approvals"],
    ["audit.html", "审计", "audit"],
  ];
  const links = items.map(([href, label, key]) =>
    `<a href="${href}" class="${key === active ? "active" : ""}">${label}</a>`).join("");
  return `<header class="topbar"><div class="dot"></div><h1>Apeireth 面板</h1>
    <nav>${links}<a href="/" title="回到聊天页">聊天</a></nav></header>`;
}

// fetch JSON; 非 2xx 抛 Error(后端 error 字段)
async function panelFetch(path, opts) {
  const r = await fetch(path, opts);
  let j = {};
  try { j = await r.json(); } catch (_) { /* 空体 */ }
  if (!r.ok) throw new Error(j.error || ("HTTP " + r.status));
  return j;
}

// HTML 转义 (所有数据渲染必经 — 面板只读展示, 防注入)
function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g,
    c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

// 时间戳 → 本地时间 (秒/毫秒自适应)
function fmtTs(sec) {
  if (sec == null) return "—";
  const ms = Math.abs(sec) > 1e12 ? sec : sec * 1000;
  return new Date(ms).toLocaleString("zh-CN", { hour12: false });
}

// 截断展示
function trunc(s, n = 160) {
  s = String(s ?? "");
  return s.length > n ? s.slice(0, n) + "…" : s;
}

// 统一错误渲染
function showErr(el, e) {
  el.innerHTML = `<div class="errbox">加载失败: ${esc(e.message)}</div>`;
}
