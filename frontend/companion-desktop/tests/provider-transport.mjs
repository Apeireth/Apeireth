// 传输选择单测（IPC 代理 · webview 零出网）— import 真实 src/lib/provider-transport.ts
//
// 锁死传输选择语义：
//   1. 非 Tauri（开发浏览器）→ 回退 fetch，行为与改造前一致；
//   2. Tauri（window.__TAURI_INTERNALS__ 在场）→ invoke('provider_request')
//      由 Rust 侧代发，webview 不再直连 fetch。
// invoke 走 @tauri-apps/api/core 真实实现（内部转发 window.__TAURI_INTERNALS__.invoke），
// 用注入的假 internals 接住，不打桩模块本身。
import assert from 'node:assert/strict';

let checks = 0;
const check = (name, fn) => {
  fn();
  checks += 1;
  console.log(`  ok  ${name}`);
};

console.log('--- Provider transport selection (real module) ---');

const originalFetch = globalThis.fetch;

// ---------------------------------------------------------------------------
// 1. 非 Tauri：无 window → fetch 回退
// ---------------------------------------------------------------------------
{
  delete globalThis.window;
  const calls = [];
  globalThis.fetch = async (input, init) => {
    calls.push({url: String(input), init});
    return new Response('{"data":[]}', {
      status: 200,
      headers: {'content-type': 'application/json'},
    });
  };
  const {providerFetch} = await import('../src/lib/provider-transport.ts');
  const res = await providerFetch('https://api.deepseek.com/v1/models', {
    headers: {Authorization: 'Bearer sk-test'},
  });
  check('非 Tauri 环境回退 fetch', () => {
    assert.equal(calls.length, 1, '应恰好调用一次 fetch');
    assert.equal(calls[0].url, 'https://api.deepseek.com/v1/models');
    assert.equal(res.status, 200);
  });
}

// ---------------------------------------------------------------------------
// 2. Tauri：__TAURI_INTERNALS__ 在场 → invoke('provider_request')，不 fetch
// ---------------------------------------------------------------------------
{
  const invokes = [];
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke: async (cmd, args) => {
        invokes.push({cmd, args});
        return {
          status: 200,
          body_text: '{"data":[{"id":"deepseek-v4-flash"}]}',
          content_type: 'application/json',
        };
      },
    },
  };
  globalThis.fetch = async () => {
    throw new Error('Tauri 环境下 webview 不得直连 fetch');
  };
  try {
    const {providerFetch} = await import('../src/lib/provider-transport.ts');
    const res = await providerFetch('https://api.deepseek.com/v1/models', {
      method: 'GET',
      headers: {Authorization: 'Bearer sk-test'},
    });
    check('Tauri 环境走 invoke(provider_request)', () => {
      assert.equal(invokes.length, 1, '应恰好调用一次 provider_request');
      assert.equal(invokes[0].cmd, 'provider_request');
      assert.equal(invokes[0].args.url, 'https://api.deepseek.com/v1/models');
      assert.equal(invokes[0].args.method, 'GET');
      assert.equal(invokes[0].args.headers.authorization, 'Bearer sk-test');
    });
    check('IPC 载荷还原为 Response，调用侧语义不变', () => {
      assert.equal(res.status, 200);
      assert.equal(res.headers.get('content-type'), 'application/json');
    });
    const data = await res.json();
    check('body_text 原样返回给 json()', () => {
      assert.equal(data.data[0].id, 'deepseek-v4-flash');
    });
  } finally {
    globalThis.fetch = originalFetch;
    delete globalThis.window;
  }
}

globalThis.fetch = originalFetch;
console.log(`--- ${checks} checks ---`);
