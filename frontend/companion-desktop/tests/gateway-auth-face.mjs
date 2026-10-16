// 网关认证面收口 — 令牌三源取值 / 全 fetch 带 Bearer / 无效鉴权头清零
// (真实模块 src/lib/tauri-bridge.ts + src/lib/runtime.ts, Node 类型擦除直 import)。
//
// 覆盖 (W2-② 残余 (d) 配对批):
//   ① 三源令牌取值: IPC (桌面) > env (web 构建注入) > 无; 哨兵=明示不设防;
//      每次现取不缓存 (侧车重启换令牌不滞留);
//   ② 全 fetch 带 Bearer 镜像断言: 网关调用面逐个过 gatewayAuthHeaders();
//   ③ 无效鉴权头清零: 无令牌不发鉴权头 (不发空 Bearer); 凭据
//      (config.apiKey) 从来不是入站鉴权, 任何请求头不得出现;
//   ④ 源码镜像: 网关 fetch 调用点全部带鉴权头缝 + 陈旧凭据头零残留。
//
// 已注册: tests/run-all.mjs SUITES (strip:true, 随 pnpm test 全量跑)。
// 独立运行: node --experimental-strip-types tests/gateway-auth-face.mjs
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const runtime = await import('../src/lib/runtime.ts');
const {
  pickGatewayTokenFace,
  resolveGatewayToken,
  gatewayAuthHeaders,
  UNPROTECTED_TOKEN_SENTINEL,
} = await import('../src/lib/tauri-bridge.ts');

console.log('--- gateway auth face (real modules) ---');

let checks = 0;
const check = (name, fn) => {
  fn();
  checks += 1;
  console.log(`  ok  ${name}`);
};

// ---------------------------------------------------------------------------
// ① 三源令牌取值 — IPC > env > 无 (纯取值 + 注入读取面 + IPC 端到端)
// ---------------------------------------------------------------------------

check('三源取值: IPC 赢 > env 兜底 > 无', () => {
  assert.equal(pickGatewayTokenFace('ipc-tok', 'env-tok'), 'ipc-tok', 'IPC (桌面) 优先');
  assert.equal(pickGatewayTokenFace('  ipc-tok  ', 'env-tok'), 'ipc-tok', '取值先裁空白');
  assert.equal(pickGatewayTokenFace('', 'env-tok'), 'env-tok', 'IPC 空白逐层跳过 → env');
  assert.equal(pickGatewayTokenFace(null, 'env-tok'), 'env-tok', 'IPC 缺席 → env');
  assert.equal(pickGatewayTokenFace(null, null), null, '两面皆空 = 无 (三态门未设口径照旧)');
  assert.equal(pickGatewayTokenFace('', '   '), null, '全空白 = 无, 不发空 Bearer');
});

check('逃生阀哨兵 = 明示不设防 → 无令牌可呈 (不回落下层)', () => {
  assert.equal(UNPROTECTED_TOKEN_SENTINEL, 'none');
  assert.equal(pickGatewayTokenFace('none', 'env-tok'), null, '哨兵在高层 = 不设防, 不拿下层令牌');
  assert.equal(pickGatewayTokenFace('', 'none'), null, 'env 面哨兵同样是不设防');
});

check('resolveGatewayToken 注入三源按序取值', async () => {
  const via = async (ipc, env) =>
    resolveGatewayToken({ipc: async () => ipc, env: () => env});
  assert.equal(await via('ipc-tok', 'env-tok'), 'ipc-tok');
  assert.equal(await via('', 'env-tok'), 'env-tok');
  assert.equal(await via('', ''), null);
  assert.equal(await via('none', 'env-tok'), null);
});

{
  // IPC 端到端: 假 Tauri internals 接住 get_gateway_token (同 provider-transport 手法)。
  const realWindow = globalThis.window;
  let token = 'tok-ipc-face-1';
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke: async (cmd) => (cmd === 'get_gateway_token' ? token : null),
    },
  };
  try {
    assert.equal(await resolveGatewayToken(), 'tok-ipc-face-1', '桌面 IPC 面取到令牌');
    const headers = await gatewayAuthHeaders();
    assert.deepEqual(headers, {Authorization: 'Bearer tok-ipc-face-1'});
    token = 'tok-ipc-face-2';
    assert.equal(await resolveGatewayToken(), 'tok-ipc-face-2', '每次现取不缓存 (重启换令牌不滞留)');
    token = '';
    assert.equal(await resolveGatewayToken(), null, 'IPC 空白 = 无令牌 (不发空 Bearer)');
  } finally {
    if (realWindow === undefined) delete globalThis.window;
    else globalThis.window = realWindow;
  }
  check('IPC 令牌面: 端到端取值 / 不缓存 / 空白回落无', () => {});
}

check('无任何令牌源 = 空鉴权头对象 (web 构建无令牌行为与现状一致)', async () => {
  assert.deepEqual(await gatewayAuthHeaders(), {});
});

// ---------------------------------------------------------------------------
// ② 全 fetch 带 Bearer + ③ 无效鉴权头清零 — 同一张调用面清单, 双向钉死
// ---------------------------------------------------------------------------

const GATEWAY = 'http://gateway.test';
const STALE_CREDENTIAL = 'sk-stale-inbound-credential';
/** apiKey 带陈旧凭据值: 收口后它绝不能再出现在任何入站鉴权头上。 */
const cfg = {baseUrl: GATEWAY, apiKey: STALE_CREDENTIAL, model: 'm-x'};

function headerRecord(headers) {
  const out = {};
  if (!headers) return out;
  for (const [key, value] of new Headers(headers).entries()) out[key.toLowerCase()] = value;
  return out;
}

const realFetch = globalThis.fetch;

/**
 * 统一请求记录面: 直连 fetch 与 providerFetch 的 IPC 代发都记进同一张表
 * (两条传输都必须带同一份鉴权头 —— 鉴权面不随传输分叉)。
 * 传 token = 装假 Tauri 面 (IPC 取令牌 + provider_request 代发); 不传 = 纯 fetch。
 */
function installRecorder(token) {
  const calls = [];
  const realWindow = globalThis.window;
  const record = (url, method, headers) =>
    calls.push({url: String(url), method: method ?? 'GET', headers: headerRecord(headers)});
  if (token !== undefined) {
    globalThis.window = {
      __TAURI_INTERNALS__: {
        invoke: async (cmd, args) => {
          if (cmd === 'get_gateway_token') return token;
          if (cmd === 'provider_request') {
            record(args.url, args.method, args.headers);
            return {status: 200, body_text: '{"data":[]}', content_type: 'application/json'};
          }
          return null;
        },
      },
    };
  }
  globalThis.fetch = async (input, init) => {
    const url = String(input);
    record(url, init?.method, init?.headers);
    if (url.endsWith('/v1/chat/completions') && String(init?.body ?? '').includes('"stream":true')) {
      return new Response('data: {"choices":[{"delta":{"content":"ok"}}]}\n\n', {status: 200});
    }
    return new Response(
      JSON.stringify({
        ok: true,
        choices: [{message: {content: 'ok'}}],
        sessions: [],
        episodes: [],
        nodes: [],
        edges: [],
        events: [],
        tools: [],
        approvals: [],
        grants: [],
        traces: [],
        spans: [],
        data: [],
        turn: null,
        warnings: [],
      }),
      {status: 200, headers: {'content-type': 'application/json'}},
    );
  };
  return {
    calls,
    restore() {
      globalThis.fetch = realFetch;
      if (token !== undefined) {
        if (realWindow === undefined) delete globalThis.window;
        else globalThis.window = realWindow;
      }
    },
  };
}

/** 网关调用面清单 (行为表): 每行 = 一个网关请求面; 改造点全数在案。 */
const GATEWAY_CALL_SITES = [
  {path: '/health', call: () => runtime.checkHealth(GATEWAY)},
  {
    path: '/health',
    call: () => runtime.checkHealthDetailed(GATEWAY, STALE_CREDENTIAL, 'm-x'),
  },
  {path: '/v1/models', call: () => runtime.checkHealthDetailed(GATEWAY, STALE_CREDENTIAL, 'm-x')},
  {path: '/v1/models', call: () => runtime.listModels(cfg)},
  {path: '/v1/admin/config', call: () => runtime.getAdminConfig(cfg)},
  {path: '/v1/admin/config', call: () => runtime.applyAdminConfig(cfg, {})},
  {path: '/v1/sessions/s-1/settings', call: () => runtime.getSessionSettings(cfg, 's-1')},
  {path: '/v1/sessions/s-1/settings', call: () => runtime.patchSessionSettings(cfg, 's-1', {})},
  {path: '/v1/chat/completions', call: () => runtime.streamChat(cfg, [{role: 'user', content: 'hi'}], {})},
  {path: '/v1/chat/completions', call: () => runtime.chatOnce(cfg, 'hi')},
  {path: '/v1/apeireth/capabilities', call: () => runtime.fetchCapabilities(cfg)},
  {path: '/v1/panel/sessions', call: () => runtime.fetchBackendSessions(cfg)},
  {path: '/v1/sessions/s-1', call: () => runtime.deleteBackendSession(cfg, 's-1')},
  {path: '/v1/panel/memory/episodes', call: () => runtime.fetchMemoryEpisodes(cfg)},
  {path: '/v1/panel/graph', call: () => runtime.fetchGraphData(cfg)},
  {path: '/v1/runtime/snapshot', call: () => runtime.fetchRuntimeSnapshot(cfg)},
  {path: '/v1/panel/audit', call: () => runtime.fetchAuditLogs(cfg)},
  {path: '/v1/tools/list', call: () => runtime.fetchTools(cfg)},
  {path: '/v1/approvals', call: () => runtime.fetchCanonicalApprovals(cfg, 's-1')},
  {
    path: '/v1/approvals/resolve',
    call: () => runtime.resolveCanonicalApproval(cfg, {session: 's-1', approval_id: 'a-1', tool_name: 'shell'}, 'approve'),
  },
  {path: '/v1/memory/append', call: () => runtime.appendMemoryEpisode(cfg, 'note')},
  {path: '/forget', call: () => runtime.forgetMemoryEpisode(cfg, 'e-1', 1)},
  {path: '/protect', call: () => runtime.protectMemoryEpisode(cfg, 'e-1', 1)},
  {path: '/unprotect', call: () => runtime.unprotectMemoryEpisode(cfg, 'e-1', 1)},
  {path: '/v1/panel/grants', call: () => runtime.fetchGrants(cfg)},
  {path: '/v1/panel/grants/revoke', call: () => runtime.revokeGrant(cfg, 'g-1')},
  {path: '/v1/panel/traces', call: () => runtime.fetchTraceList(cfg)},
  {path: '/v1/panel/traces/t-1', call: () => runtime.fetchTraceDetail(cfg, 't-1')},
  {path: '/v1/safety/guard/status', call: () => runtime.fetchGuardStatus(cfg)},
  {path: '/v1/safety/guard/events', call: () => runtime.fetchGuardEvents(cfg)},
  {path: '/v1/safety/guard/evaluate', call: () => runtime.evaluateGuard(cfg, {})},
  {path: '/v1/workbench/turn', call: () => runtime.fetchWorkbenchTurn(cfg)},
  {path: '/v1/panel/skills', call: () => runtime.fetchSkills(cfg)},
];

{
  // ② 带令牌: 每个网关调用面 (直连 fetch 与 IPC 代发两条传输) 都带 Bearer <回环令牌>。
  const recorder = installRecorder('tok-face-abc');
  try {
    for (const site of GATEWAY_CALL_SITES) {
      recorder.calls.length = 0;
      await site.call();
      assert.ok(recorder.calls.length >= 1, `${site.path} 确实发出了请求`);
      for (const call of recorder.calls) {
        assert.ok(
          call.url.startsWith(GATEWAY),
          `${site.path}: 只打网关面 (实际=${call.url})`,
        );
        assert.equal(
          call.headers.authorization,
          'Bearer tok-face-abc',
          `${site.path} (${call.method} ${call.url}) 必须带回环令牌 Bearer`,
        );
      }
    }
  } finally {
    recorder.restore();
  }
  check(`全 fetch 带 Bearer: 网关调用面 ${GATEWAY_CALL_SITES.length} 项逐个过 gatewayAuthHeaders()`, () => {});
}

{
  // ③ 无令牌: 三态门未设口径照旧 —— 一个鉴权头都不发 (更不发空 Bearer);
  //    陈旧凭据 config.apiKey 不得变脸成入站鉴权。
  const recorder = installRecorder(undefined);
  try {
    for (const site of GATEWAY_CALL_SITES) {
      recorder.calls.length = 0;
      await site.call();
      for (const call of recorder.calls) {
        assert.equal(
          call.headers.authorization,
          undefined,
          `${site.path}: 无令牌不得发鉴权头 (实际=${JSON.stringify(call.headers.authorization)})`,
        );
        assert.ok(
          !JSON.stringify(call.headers).includes(STALE_CREDENTIAL),
          `${site.path}: 凭据从来不是入站鉴权 (config.apiKey 不得进请求头)`,
        );
      }
    }
  } finally {
    recorder.restore();
  }
  check('无效鉴权头清零: 无令牌零鉴权头 + 陈旧凭据零进头', () => {});
}

// ---------------------------------------------------------------------------
// ④ 源码镜像 — 网关 fetch 调用点全带鉴权头缝; 陈旧凭据头零残留
// ---------------------------------------------------------------------------

{
  const srcDir = join(here, '..', 'src');
  const runtimeSrc = readFileSync(join(srcDir, 'lib', 'runtime.ts'), 'utf8');
  const presenceSrc = readFileSync(join(srcDir, 'lib', 'presence.ts'), 'utf8');
  const activitySrc = readFileSync(join(srcDir, 'lib', 'views', 'ActivityView.svelte'), 'utf8');
  const approvalsSrc = readFileSync(join(srcDir, 'lib', 'views', 'governance', 'ApprovalsTab.svelte'), 'utf8');
  const faces = [
    ['runtime.ts', runtimeSrc],
    ['presence.ts', presenceSrc],
    ['ActivityView.svelte', activitySrc],
    ['ApprovalsTab.svelte', approvalsSrc],
  ];
  for (const [name, src] of faces) {
    assert.ok(!/Bearer \$\{config\.apiKey\}/.test(src), `${name}: 陈旧凭据 Bearer 头零残留`);
    assert.ok(!/Authorization[^\n]*config\.apiKey/.test(src), `${name}: 凭据不得构造鉴权头`);
    assert.ok(!/Authorization:\s*''/.test(src), `${name}: 空串鉴权头零残留`);
    assert.ok(!/Authorization:\s*""/.test(src), `${name}: 空串鉴权头零残留`);
  }

  // 每个打到网关面 (baseUrl 面) 的 fetch 调用点必须过 gatewayAuthHeaders();
  // 直连 provider 面 (provider.apiKey 上游凭据) 不在此列。
  const sites = [...runtimeSrc.matchAll(/\bfetch\(/g)].map((m) => m.index);
  let gatewaySites = 0;
  for (const idx of sites) {
    const window = runtimeSrc.slice(Math.max(0, idx - 400), idx + 400);
    if (!/config\.baseUrl|\$\{base\}|\$\{baseUrl\}|normalizeBaseUrl\(baseUrl\)/.test(window)) continue;
    gatewaySites += 1;
    assert.ok(
      window.includes('gatewayAuthHeaders()'),
      `网关 fetch 调用点缺鉴权头缝: …${runtimeSrc.slice(idx, idx + 90).replace(/\n/g, ' ')}…`,
    );
  }
  assert.ok(gatewaySites >= 20, `网关 fetch 调用点全数在镜像覆盖内 (实际=${gatewaySites})`);
  check(`源码镜像: 网关 fetch 调用点 ${gatewaySites} 处全带鉴权头缝 + 凭据头零残留`, () => {});
}

globalThis.fetch = realFetch;
console.log(`--- ${checks} checks ---`);
