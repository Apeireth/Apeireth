// 用户卡（D10 用户/域管理上屏：真实模块 src/lib/user-admin.ts + runtime.ts 读面），
// 并对 UserAdminPanel.svelte 做源码级镜像校验。
// 契约（frontend-data-contract §10.6 / §10.7 / §10.3）：
//   1. 列表：id / 角色 / 客户端数 真值上屏投影（形状不符的行丢弃，不编默认值）。
//   2. 只读态：读面被拒（501 no_role_face / 403 permission_denied / 网络失败）
//      → read-only + 真因；读面可用 = manage。
//   3. CLI 指引：与写面同源的具体可复制命令（apeireth user role/remove/list）。
//   4. owner 写面：PATCH /v1/admin/users/:id {role} / DELETE 同路径，请求形状钉死。
//   5. 当前域节：self_status 域节合同形状（counts 缺探测口显式 null + reason）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  DOMAIN_COUNTS_NO_PROBE,
  USER_ROLES,
  domainSectionView,
  normalizeDomainRows,
  normalizeUserRows,
  patchUserRole,
  removeUser,
  userAdminView,
  userCliBootstrap,
  userCliGuidance,
  userPanelMode,
} from '../src/lib/user-admin.ts';
import {fetchUsers, fetchDomains} from '../src/lib/runtime.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const panelSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'components', 'UserAdminPanel.svelte'),
  'utf8',
);
const runtimeSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'runtime.ts'), 'utf8');

console.log('--- user admin card (D10) ---');

function check(name, fn) {
  // 顺序执行（不并发）：读面桩互换 globalThis.fetch，抢跑会互踩。
  queue.push({name, fn});
}
const queue = [];
const awaitAll = [];

// ---------------------------------------------------------------------------
// 1. 列表：id / 角色 / 客户端数 投影真值
// ---------------------------------------------------------------------------
awaitAll.push(
  check('列表: id / 角色 / 客户端数 三列真值投影', () => {
    const rows = normalizeUserRows([
      {user_id: 'alice', role: 'owner', client_names: ['desk', 'phone'], client_count: 2, domain: 'd1'},
      {user_id: 'bob', role: 'member', client_names: [], client_count: 0},
    ]);
    const view = userAdminView(rows);
    assert.equal(view.count, 2);
    assert.equal(view.rows[0].id, 'alice');
    assert.equal(view.rows[0].role, 'owner');
    assert.equal(view.rows[0].clientCount, 2, '客户端数 = 合同 client_count 真值');
    assert.equal(view.rows[0].domain, 'd1');
    assert.equal(view.rows[1].clientCount, 0, '真实 0 照显 0');
    assert.equal(view.rows[1].domain, '', '无域 → 空串（不编域名）');
  }),
);

awaitAll.push(
  check('列表: 形状不符的行丢弃 + client_count 缺席用实得名单长度（不编 0）', () => {
    const rows = normalizeUserRows([
      {user_id: 'ok', role: 'guest', client_names: ['a', 'b']},
      {role: 'owner', client_names: [], client_count: 9},
      {user_id: '', role: 'owner', client_names: [], client_count: 1},
      'not-an-object',
    ]);
    assert.equal(rows.length, 1, '缺 id / 空 id / 非对象一律丢弃');
    assert.equal(rows[0].client_count, 2, 'client_count 缺席 → 实得名单长度（实数）');
    assert.deepEqual(normalizeUserRows(null), [], '非数组 → 空（不编行）');
  }),
);

// ---------------------------------------------------------------------------
// 2. 只读态：三类拒绝各有真因；读面可用 = manage
// ---------------------------------------------------------------------------
awaitAll.push(
  check('只读态: 501 no_role_face → read-only + 引导真因', () => {
    const mode = userPanelMode({ok: false, status: 501, code: 'no_role_face', error: 'x'});
    assert.equal(mode.mode, 'read-only');
    assert.ok(mode.reason.includes('no_role_face'), '真因口径带 code');
    assert.ok(mode.reason.includes('CLI'), '只读态指向 CLI 引导');
  }),
);

awaitAll.push(
  check('只读态: 403 permission_denied / 网络失败同样 read-only；读面可用 = manage', () => {
    assert.equal(userPanelMode({ok: false, status: 403, code: 'permission_denied'}).mode, 'read-only');
    const net = userPanelMode({ok: false, status: 0, code: 'network_error', error: 'ECONNREFUSED'});
    assert.equal(net.mode, 'read-only');
    assert.ok(net.reason.includes('ECONNREFUSED'), '网络失败报真因（错误即帧）');
    assert.deepEqual(userPanelMode({ok: true}), {mode: 'manage', reason: ''}, '读面可用 = 管理视角');
  }),
);

// ---------------------------------------------------------------------------
// 3. CLI 指引：同源具体可复制命令
// ---------------------------------------------------------------------------
awaitAll.push(
  check('CLI 指引: 角色调整 / 移除 / 列表三命令具体可复制（与写面同源）', () => {
    const cmds = userCliGuidance('alice', 'owner');
    assert.equal(cmds.length, 3);
    assert.equal(cmds[0], 'apeireth user role --user-id alice --role owner', '角色命令含具体 id/role');
    assert.equal(cmds[1], 'apeireth user remove --user-id alice', '移除命令含具体 id');
    assert.equal(cmds[2], 'apeireth user list');
    assert.ok(USER_ROLES.includes('owner') && USER_ROLES.includes('member') && USER_ROLES.includes('guest'),
      '角色词表冻结 owner|member|guest');
    assert.equal(userCliBootstrap('carol'), 'apeireth user add --user-id carol --role owner', '空表引导命令同源 CLI 用法');
  }),
);

// ---------------------------------------------------------------------------
// 4. owner 写面：PATCH / DELETE 请求形状 + 鉴权头缝
// ---------------------------------------------------------------------------
awaitAll.push(
  check('写面: PATCH /v1/admin/users/:id {role} 请求形状 + 鉴权头过缝', async () => {
    const calls = [];
    const deps = {
      authHeaders: async () => ({Authorization: 'Bearer test-token'}),
      fetchJson: async (url, init) => {
        calls.push({url, init});
        return {ok: true, status: 200, json: async () => ({user: {user_id: 'alice', role: 'member', client_names: [], client_count: 1}})};
      },
    };
    const r = await patchUserRole('http://127.0.0.1:8080', 'alice', 'member', deps);
    assert.ok(r.ok && r.user.role === 'member', '成功帧带归一 user 行');
    assert.equal(calls[0].url, 'http://127.0.0.1:8080/v1/admin/users/alice');
    assert.equal(calls[0].init.method, 'PATCH');
    assert.deepEqual(JSON.parse(calls[0].init.body), {role: 'member'}, '请求体只带 {role}');
    assert.equal(calls[0].init.headers.Authorization, 'Bearer test-token', '鉴权头走注入缝单源');
    const bad = await patchUserRole('http://x', 'alice', 'root', deps);
    assert.ok(!bad.ok && bad.code === 'invalid_role', '词表外角色就地拒绝（不发请求）');
  }),
);

awaitAll.push(
  check('写面: DELETE /v1/admin/users/:id + 错误帧 {code,message} 归一', async () => {
    const calls = [];
    const deps = {
      authHeaders: () => ({Authorization: 'Bearer t'}),
      fetchJson: async (url, init) => {
        calls.push({url, init});
        return {
          ok: false,
          status: 403,
          json: async () => ({error: {code: 'permission_denied', message: '仅 owner 可移除住户'}}),
        };
      },
    };
    const r = await removeUser('http://127.0.0.1:8080/', 'bob', deps);
    assert.equal(calls[0].url, 'http://127.0.0.1:8080/v1/admin/users/bob', '尾斜杠裁掉 + id 编码');
    assert.equal(calls[0].init.method, 'DELETE');
    assert.ok(!r.ok);
    assert.equal(r.code, 'permission_denied', '错误帧 code 透出');
    assert.equal(r.error, '仅 owner 可移除住户', '错误帧 message 透出（错误即帧）');
    assert.equal(r.status, 403);
  }),
);

// ---------------------------------------------------------------------------
// 5. 当前域节：self_status 域节合同形状（§10.3）
// ---------------------------------------------------------------------------
awaitAll.push(
  check('当前域节: §10.3 同形 + counts 三维显式 null + reason', () => {
    const rows = normalizeDomainRows([
      {user_id: 'd1', display_name: '主域', created_at: 1728000000000, role: 'owner', user_count: 3},
      {user_id: '', display_name: 'x', created_at: 0, role: 'owner'},
    ]);
    assert.equal(rows.length, 1, '缺 id 的域行丢弃');
    const sec = domainSectionView(rows[0]);
    assert.equal(sec.user_id, 'd1');
    assert.equal(sec.display_name, '主域');
    assert.equal(sec.role, 'owner');
    assert.equal(sec.created_at, 1728000000000);
    assert.equal(sec.counts.sessions, null, 'sessions 缺探测口 → 显式 null');
    assert.equal(sec.counts.memories, null);
    assert.equal(sec.counts.protected, null);
    assert.equal(sec.counts.reason, DOMAIN_COUNTS_NO_PROBE, 'counts.reason 如实写缺源原因');
  }),
);

// ---------------------------------------------------------------------------
// 6. 读面请求形状：fetchUsers / fetchDomains 打真 URL + 鉴权头缝（记录桩传输）
// ---------------------------------------------------------------------------
awaitAll.push(
  check('读面: fetchUsers / fetchDomains 请求契约（URL + 鉴权头 + 信封归一）', async () => {
    const calls = [];
    const realFetch = globalThis.fetch;
    globalThis.fetch = async (url, init) => {
      calls.push({url, init});
      return {
        ok: true,
        status: 200,
        json: async () =>
          String(url).endsWith('/v1/panel/users')
            ? {users: [{user_id: 'alice', role: 'owner', client_names: ['desk'], client_count: 1}], count: 1}
            : {domains: [{user_id: 'd1', display_name: '主域', created_at: 1, role: 'owner'}], count: 1},
      };
    };
    try {
      const cfg = {baseUrl: 'http://127.0.0.1:8080'};
      const usersRes = await fetchUsers(cfg);
      const domainsRes = await fetchDomains(cfg);
      assert.equal(calls[0].url, 'http://127.0.0.1:8080/v1/panel/users', '住户读面 URL 契约');
      assert.equal(calls[1].url, 'http://127.0.0.1:8080/v1/panel/domains', '域读面 URL 契约');
      assert.ok(calls.every((c) => c.init.headers), '读面请求带头（鉴权头缝）');
      assert.ok('users' in usersRes && usersRes.users[0].user_id === 'alice', '用户行归一');
      assert.ok('domains' in domainsRes && domainsRes.domains[0].user_id === 'd1', '域行归一');
    } finally {
      globalThis.fetch = realFetch;
    }
  }),
);

awaitAll.push(
  check('读面: 拒绝帧透出 status + code（只读态据此判定，不吞错）', async () => {
    const realFetch = globalThis.fetch;
    globalThis.fetch = async () => ({
      ok: false,
      status: 403,
      json: async () => ({error: {code: 'permission_denied', message: '仅 owner 可读'}}),
    });
    try {
      const res = await fetchUsers({baseUrl: 'http://127.0.0.1:8080'});
      assert.ok('error' in res, '拒绝帧是显式错误结果');
      assert.equal(res.status, 403);
      assert.equal(res.code, 'permission_denied');
      assert.equal(res.error, '仅 owner 可读');
    } finally {
      globalThis.fetch = realFetch;
    }
  }),
);

// ---------------------------------------------------------------------------
// 7. 源码镜像：读面走 runtime.ts 鉴权头缝 + 面板接线
// ---------------------------------------------------------------------------
awaitAll.push(
  check('源码镜像: runtime 读面带 gatewayAuthHeaders + 面板接线齐', () => {
    assert.ok(runtimeSrc.includes('/v1/panel/users'), '住户读面 URL 在案');
    assert.ok(runtimeSrc.includes('/v1/panel/domains'), '域读面 URL 在案');
    const usersIdx = runtimeSrc.indexOf('export async function fetchUsers');
    const window = runtimeSrc.slice(usersIdx, usersIdx + 700);
    assert.ok(window.includes('gatewayAuthHeaders()'), '住户读面过鉴权头缝（单源）');
    assert.ok(panelSrc.includes('fetchUsers(config)'), '面板走 runtime 读面');
    assert.ok(panelSrc.includes('userAdminView(users)'), '展示映射走纯函数');
    assert.ok(panelSrc.includes('userPanelMode('), '只读态判定走纯函数');
    assert.ok(panelSrc.includes('patchUserRole(') && panelSrc.includes('removeUser('), 'owner 写面接线');
    assert.ok(panelSrc.includes('userCliGuidance('), 'CLI 指引上屏');
    assert.ok(panelSrc.includes('domainSectionView'), '当前域节消费 §10.3 形状');
    assert.ok(panelSrc.includes('gatewayAuthHeaders()'), '写面鉴权头过缝');
  }),
);

for (const {name, fn} of queue) {
  await fn();
  console.log(`  ok - ${name}`);
}
console.log('user-admin: all checks passed');
