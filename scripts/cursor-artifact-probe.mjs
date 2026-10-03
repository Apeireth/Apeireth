#!/usr/bin/env node
// 光标/插入符取证探针 v2（cursor-artifact probe）——
// 主人反馈：origin 主题下光标在输入框上"异常白色、几乎不可见"。
// 本探针做三件事（配合外层 pwsh 的 Win32 光标句柄/图标抓取）：
//   1. 强制切到指定主题（默认 origin），读输入框的计算样式
//      （cursor / caret-color / color / background）——判定白的是插入符还是鼠标光标；
//   2. 输出输入框中心的**屏幕坐标**（含 DPI 换算所需的 CSS 坐标），
//      供外层 SetCursorPos 把真光标放上去；
//   3. 可选连拍截图（伪影取证）。
//
// 用法（应用需带 --remote-debugging-port=9223 启动）：
//   node scripts/cursor-artifact-probe.mjs --audit-input [--theme origin] [--shot artifacts/x.png]
import {writeFileSync, mkdirSync} from 'node:fs';
import {dirname} from 'node:path';

const args = process.argv.slice(2);
const has = (f) => args.includes(`--${f}`);
function argValue(name, fallback) {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
}
const PORT = Number(argValue('port', '9223'));
const THEME = argValue('theme', 'origin');
const OUT = argValue('out', 'cursor-shot.png');

const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
const page = list.find((t) => t.type === 'page' && /tauri\.localhost\/(\?|$)/.test(t.url) && !t.url.includes('window='));
if (!page) {
  console.error('未找到主窗口 target:', list.map((t) => `${t.type} ${t.url}`).join(' | '));
  process.exit(1);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((res, rej) => {
  ws.onopen = res;
  ws.onerror = () => rej(new Error('CDP ws error'));
});
let nextId = 1;
const pending = new Map();
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  const w = msg.id && pending.get(msg.id);
  if (w) {
    pending.delete(msg.id);
    msg.error ? w.rej(new Error(msg.error.message)) : w.res(msg.result);
  }
};
const send = (method, params = {}) =>
  new Promise((res, rej) => {
    const id = nextId++;
    ws.send(JSON.stringify({id, method, params}));
    pending.set(id, {res, rej});
    setTimeout(() => pending.delete(id) && rej(new Error(`timeout ${method}`)), 8000);
  });
const evaluate = async (expression) => {
  const out = await send('Runtime.evaluate', {expression, returnByValue: true});
  return out.result.value;
};

if (has('audit-input')) {
  // 1) 强制切主题（纯 CSS 域足够做计算样式取证；app-root 类名同步补上）。
  const applied = await evaluate(`(() => {
    document.documentElement.setAttribute('data-theme', '${THEME}');
    const root = document.querySelector('.app-root');
    if (root) root.classList.add('theme-${THEME}');
    return document.documentElement.getAttribute('data-theme');
  })()`);
  await new Promise((r) => setTimeout(r, 300));
  console.log('theme forced:', applied);

  // 2) 输入框（优先聊天 composer textarea，退化到第一个可见 input/textarea）计算样式 + 原始视口矩形。
  const audit = await evaluate(`(() => {
    const el = document.querySelector('.composer textarea')
      || [...document.querySelectorAll('input, textarea')].find((e) => e.getBoundingClientRect().width > 0);
    if (!el) return JSON.stringify({missing: true});
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return JSON.stringify({
      tag: el.tagName,
      cls: String(el.className),
      cursor: cs.cursor,
      caretColor: cs.caretColor,
      color: cs.color,
      background: cs.backgroundColor,
      // 原始视口矩形（CSS px；外层 DPI 虚拟化下与 Win32 客户区 1:1）
      vp: {left: r.left, top: r.top, width: r.width, height: r.height},
      innerW: window.innerWidth,
      innerH: window.innerHeight,
    });
  })()`);
  console.log('INPUT_AUDIT ' + audit);

  // 2b) 会话搜索框（窗口左上、必在屏幕内，用于真光标句柄对比）。
  const audit2 = await evaluate(`(() => {
    const el = document.querySelector('.home-search-input') || document.querySelector('.search input');
    if (!el) return JSON.stringify({missing: true});
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return JSON.stringify({cursor: cs.cursor, caretColor: cs.caretColor, vp: {left: r.left, top: r.top, width: r.width, height: r.height}});
  })()`);
  console.log('SEARCH_AUDIT ' + audit2);
}

if (has('shot')) {
  mkdirSync(dirname(OUT), {recursive: true});
  for (let i = 0; i < 2; i++) {
    const shot = await send('Page.captureScreenshot', {format: 'png'});
    writeFileSync(OUT.replace(/\.png$/, '') + `-${i}.png`, Buffer.from(shot.data, 'base64'));
    console.log('saved', OUT.replace(/\.png$/, '') + `-${i}.png`);
    await new Promise((r) => setTimeout(r, 250));
  }
}
ws.close();
