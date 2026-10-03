#!/usr/bin/env node
// 设置子导航几何量测——对运行中的真实 WebView2 应用，在 980×640 最小窗视口
// （Emulation.setDeviceMetricsOverride，同 wheel-scroll-probe 手法）量出子导航
// 的高度预算与行高，为「多开多收」的总高度可行性给精确数字。
//
// 用法（先带调试端口启动应用）：
//   $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223'
//   Start-Process 'C:\Program Files\Apeireth Companion\companion-desktop.exe'
//   node scripts/measure-settings-nav.mjs [--port 9223] [--width 980] [--height 640]

const args = process.argv.slice(2);
function argValue(name, fallback) {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
}
const PORT = Number(argValue('port', '9223'));
const WIDTH = Number(argValue('width', '980'));
const HEIGHT = Number(argValue('height', '640'));
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
    const page = list.find((t) => t.type === 'page' && !/window=quick/.test(t.url || ''));
    if (page) {
      ws = await connect(page.webSocketDebuggerUrl);
      break;
    }
  } catch {}
}
if (!ws) {
  console.error('FATAL: CDP 未就绪');
  process.exit(1);
}

await ws.send('Page.enable');
await ws.send('Emulation.setDeviceMetricsOverride', {
  width: WIDTH,
  height: HEIGHT,
  deviceScaleFactor: 1,
  mobile: false,
});
await sleep(500);

// 进设置页
await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const b = [...document.querySelectorAll('button')].find((x) => (x.title || '').startsWith('设置'));
    if (b) b.click();
    return !!b;
  })()`,
});
await sleep(1000);

const geo = await ws.send('Runtime.evaluate', {
  expression: `(() => {
    const aside = document.querySelector('.settings-subnav');
    const cs = getComputedStyle(aside);
    const head = document.querySelector('.subnav-group-btn') || document.querySelector('.subnav-group-title');
    const page = document.querySelector('.subnav-btn');
    const groups = document.querySelectorAll('.subnav-group').length;
    return {
      viewport: {w: innerWidth, h: innerHeight},
      asideClientHeight: aside.clientHeight,
      asideScrollHeight: aside.scrollHeight,
      asidePadding: cs.padding,
      asideOverflow: cs.overflow,
      headRowHeight: head ? head.getBoundingClientRect().height : null,
      pageRowHeight: page ? page.getBoundingClientRect().height : null,
      groups,
    };
  })()`,
  returnByValue: true,
});
console.log(JSON.stringify(geo.result.value, null, 2));
ws.close();
