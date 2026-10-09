#!/usr/bin/env node
// 用户中心真机验收——对**运行中的已安装应用**（WebView2）发 CDP 指令，验证
// 「左上角单字 → 用户头像 + 设置栏目第一页用户中心（暂只随应用存本地）」
// 已在运行中的程序里生效。
//
// 用法（先带调试端口启动应用）：
//   $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223'
//   Start-Process 'C:\Program Files\Apeireth Companion\companion-desktop.exe'
//   node scripts/verify-user-center-live.mjs [--port 9223] [--out artifacts]
//
// 断言：①左上角是头像按钮（.rail-avatar） ②单字「燧」已退场 ③点击头像无条件
//      落设置 › 用户中心 ④用户信息表单在位 ⑤昵称失焦提交真落本地 localStorage，
//      验收后把昵称还原，不给用户资料留测试数据。
// 验收后连拍两张（主界面头像 / 用户中心页）作实证。

import {writeFileSync} from 'node:fs';

const args = process.argv.slice(2);
function argValue(name, fallback) {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
}
const PORT = Number(argValue('port', '9223'));
const OUT_DIR = argValue('out', 'artifacts');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function connect(wsUrl) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    let id = 0;
    const pending = new Map();
    socket.onopen = () =>
      resolve({
        send(method, params = {}) {
          return new Promise((res, rej) => {
            const i = ++id;
            pending.set(i, {res, rej});
            socket.send(JSON.stringify({id: i, method, params}));
          });
        },
        close: () => socket.close(),
      });
    socket.onmessage = (e) => {
      const msg = JSON.parse(String(e.data));
      if (msg.id && pending.has(msg.id)) {
        const {res, rej} = pending.get(msg.id);
        pending.delete(msg.id);
        msg.error ? rej(new Error(JSON.stringify(msg.error))) : res(msg.result);
      }
    };
    socket.onerror = reject;
  });
}

let ws;
for (let i = 0; i < 30; i++) {
  await sleep(500);
  try {
    const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
    // 主窗才是宿主：跳过 ?window=quick 快捷助手小窗。
    const page = list.find((t) => t.type === 'page' && !/window=quick/.test(t.url || ''));
    if (page) {
      ws = await connect(page.webSocketDebuggerUrl);
      break;
    }
  } catch {}
}
if (!ws) {
  console.error('FATAL: CDP 未就绪（应用没带 --remote-debugging-port 启动？）');
  process.exit(1);
}

async function evalJs(expression) {
  const r = await ws.send('Runtime.evaluate', {expression, returnByValue: true});
  return r.result?.value;
}
async function shot(name) {
  const png = await ws.send('Page.captureScreenshot', {format: 'png'});
  writeFileSync(`${OUT_DIR}/${name}`, Buffer.from(png.data, 'base64'));
  console.log('saved', name);
}

const failures = [];
function expect(ok, msg) {
  if (!ok) failures.push(msg);
}

// ---- ①② 左上角 = 头像按钮；单字「燧」退场 ----
const baseline = await evalJs(`(() => ({
  avatarBtn: !!document.querySelector('button.rail-avatar'),
  avatarImg: !!document.querySelector('button.rail-avatar img.rail-avatar-img'),
  brandText: (document.querySelector('.rail-brand')?.textContent || '').trim(),
  railText: (document.querySelector('.rail')?.textContent || '').includes('燧'),
  busy: document.body.innerText.includes('正在输出'),
}))()`);
console.log('基线快照:', JSON.stringify(baseline));
expect(baseline?.avatarBtn, '左上角应是头像按钮（button.rail-avatar）');
expect(!baseline?.railText, '单字「燧」应已退场');
expect(!baseline?.brandText, '头像按钮内不应再有单字文本');
if (baseline?.busy) {
  console.error('FATAL: 应用正在输出（有进行中的回合），不打扰——改期再验收');
  ws.close();
  process.exit(2);
}
await shot('user-center-live-rail.png');

// ---- ③ 点头像无条件落设置 › 用户中心 ----
const clicked = await evalJs(`(() => {
  const b = document.querySelector('button.rail-avatar');
  if (!b) return 'NO_BTN';
  b.click();
  return 'clicked';
})()`);
console.log('点头像:', clicked);
await sleep(1200);
const landed = await evalJs(`(() => ({
  open: !!document.querySelector('.drawer.open'),
  activePage: (document.querySelector('.subnav-btn.active')?.textContent || '').trim(),
  pageTitle: (document.querySelector('.setting-block .block-title')?.textContent || '').trim(),
  hasNickname: !!document.querySelector('#user-nickname'),
  hasBio: !!document.querySelector('#user-bio'),
  hasUpload: [...document.querySelectorAll('.user-avatar-actions button')].map((b) => b.textContent.trim()),
}))()`);
console.log('落区快照:', JSON.stringify(landed, null, 2));
expect(landed?.open, '点击头像应打开设置抽屉');
expect((landed?.activePage || '').includes('用户中心'), `应落用户中心页（实得 ${landed?.activePage}）`);
expect((landed?.pageTitle || '').includes('用户中心'), '内容列应渲染用户中心分区');
expect(landed?.hasNickname && landed?.hasBio, '用户信息表单（昵称/签名）应在位');
expect((landed?.hasUpload || []).some((t) => /头像/.test(t)), '应有头像上传/更换入口');
await shot('user-center-live-page.png');

// ---- ⑤ 昵称失焦提交真落本地；验收后还原 ----
// 后台 WebView2 会吞掉程序化 blur() 的失焦事件（焦点已不在前台窗口），所以这里
// 发与真实失焦同语义的 FocusEvent('focusout')：bubbles + relatedTarget 空 = 落到
// 页面外，正是 groupFocusOut 的真实输入。
const KEY = 'apeireth-user-profile';
const before = await evalJs(`localStorage.getItem(${JSON.stringify(KEY)})`);
const committed = await evalJs(`(() => {
  const el = document.querySelector('#user-nickname');
  if (!el) return 'NO_INPUT';
  const set = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
  set.call(el, '真机验收昵称');
  el.dispatchEvent(new Event('input', {bubbles: true}));
  el.dispatchEvent(new FocusEvent('focusout', {bubbles: true, relatedTarget: null}));
  return 'submitted';
})()`);
console.log('昵称提交:', committed);
await sleep(600);
const stored = await evalJs(`(() => {
  const raw = localStorage.getItem(${JSON.stringify(KEY)});
  return raw ? JSON.parse(raw).nickname : null;
})()`);
console.log('本地读回:', JSON.stringify(stored));
expect(stored === '真机验收昵称', `昵称应真存本地并读回（实得 ${JSON.stringify(stored)}）`);

// 还原：把昵称改回原值（原值没有 = 清空），不留验收数据。
const restoreTo = (() => {
  try {
    return before ? JSON.parse(before).nickname || '' : '';
  } catch {
    return '';
  }
})();
await evalJs(`(() => {
  const el = document.querySelector('#user-nickname');
  const set = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
  set.call(el, ${JSON.stringify(restoreTo)});
  el.dispatchEvent(new Event('input', {bubbles: true}));
  el.dispatchEvent(new FocusEvent('focusout', {bubbles: true, relatedTarget: null}));
  return true;
})()`);
await sleep(600);
const restored = await evalJs(`(() => {
  const raw = localStorage.getItem(${JSON.stringify(KEY)});
  return raw ? JSON.parse(raw).nickname : null;
})()`);
console.log('还原后:', JSON.stringify(restored), '（原值', JSON.stringify(restoreTo), '）');

if (failures.length) {
  console.error('验收失败:');
  for (const f of failures) console.error('  -', f);
  process.exit(1);
}
console.log('--- 运行中程序里的用户中心（头像 + 用户信息落本地）验收全部通过 ---');
ws.close();
