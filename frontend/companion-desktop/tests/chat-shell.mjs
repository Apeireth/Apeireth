// T0 聊天壳骨骼 — 会话列表归并 / 审批事件归一化 / 治理通报分流 纯逻辑单测
// 直接 import 真实实现 ../src/lib/chat-shell/*.ts（Node ≥23.6 类型擦除；
// 两个模块零运行时依赖，DOM-free，与 presence.ts 同一约定）。
// 契约依据: docs/design/00-PHILOSOPHY.md §3、docs/gateway-api-contract.md §4/§6/§8。
// 不依赖后端进程。
import assert from 'node:assert/strict';

import {
  mergeSessionLedger,
  sessionPreview,
  formatSessionTime,
} from '../src/lib/chat-shell/session-list.ts';
import {
  parseApprovalEventPayload,
  applyApprovalEventToPending,
  classifyGovernanceNotice,
} from '../src/lib/chat-shell/gateway-events.ts';

console.log('--- Starting T0 Chat Shell Logic Check ---');

// ---------------------------------------------------------------------------
// 1. mergeSessionLedger — 本地会话 ⋈ 后端账本（/v1/panel/sessions）
// ---------------------------------------------------------------------------

const localConv = (id, over = {}) => ({
  id,
  title: over.title ?? `会话 ${id}`,
  createdAt: over.createdAt ?? 1000,
  updatedAt: over.updatedAt ?? 2000,
  messages: over.messages ?? [],
  archived: over.archived,
  scope: 'global',
});

// 本地有而后端没有 = 纯本地草稿，照常列出
{
  const items = mergeSessionLedger({
    local: [localConv('a', {updatedAt: 500})],
    backend: [],
  });
  assert.equal(items.length, 1);
  assert.equal(items[0].origin, 'local');
  assert.equal(items[0].id, 'a');
}

// 后端有而本地没有 = 他那边记得的会话；预览为 null（不伪造消息正文）
{
  const items = mergeSessionLedger({
    local: [],
    backend: [{id: 'b', title: '后端会话', started_at: 100, last_active_at: 300, episode_count: 7}],
  });
  assert.equal(items.length, 1);
  assert.equal(items[0].origin, 'backend');
  assert.equal(items[0].preview, null);
  assert.equal(items[0].messageCount, 7);
}

// 两边都有 → origin='both'，活跃时刻取较新，本地空标题用后端标题补
{
  const items = mergeSessionLedger({
    local: [localConv('c', {title: '新对话', updatedAt: 100})],
    backend: [{id: 'c', title: '后端命名', started_at: 50, last_active_at: 900, episode_count: 3}],
  });
  assert.equal(items.length, 1);
  assert.equal(items[0].origin, 'both');
  assert.equal(items[0].lastActiveAt, 900);
  assert.equal(items[0].title, '后端命名');
}

// 排序 = 最后活跃倒序；归档不进主页
{
  const items = mergeSessionLedger({
    local: [
      localConv('old', {updatedAt: 100}),
      localConv('archived', {updatedAt: 9999, archived: true}),
      localConv('new', {updatedAt: 800}),
    ],
    backend: [{id: 'mid', started_at: 0, last_active_at: 500, episode_count: 1}],
  });
  assert.deepEqual(items.map((i) => i.id), ['new', 'mid', 'old']);
}

// 待签标记只落在 SSE 点名的会话上
{
  const items = mergeSessionLedger({
    local: [localConv('x'), localConv('y')],
    backend: null, // 账本拉取失败/不支持 → 本地列表照常（0 装降级）
    pendingApprovalSessions: new Set(['y']),
  });
  assert.equal(items.find((i) => i.id === 'x').pendingApproval, false);
  assert.equal(items.find((i) => i.id === 'y').pendingApproval, true);
}

// sessionPreview：空白折叠 + 80 字截断 + 空消息为 null
{
  const conv = localConv('p', {messages: [{id: 'm1', role: 'assistant', text: '  你好\n\n世界  ', time: ''}]});
  assert.equal(sessionPreview(conv), '你好 世界');
  const long = '字'.repeat(120);
  const convLong = localConv('p2', {messages: [{id: 'm2', role: 'user', text: long, time: ''}]});
  assert.equal(sessionPreview(convLong).length, 80);
  assert.equal(sessionPreview(localConv('p3')), null);
}

// formatSessionTime：今天给时刻 / 昨天 / 跨年带年份
{
  const now = new Date('2026-10-13T15:00:00').getTime();
  const today = new Date('2026-10-13T09:05:00').getTime();
  assert.match(formatSessionTime(today, now), /09:05/);
  const yesterday = new Date('2026-10-12T23:40:00').getTime();
  assert.equal(formatSessionTime(yesterday, now), '昨天');
  const lastYear = new Date('2025-12-31T10:00:00').getTime();
  assert.match(formatSessionTime(lastYear, now), /2025/);
  assert.equal(formatSessionTime(0, now), '');
  assert.equal(formatSessionTime(Number.NaN, now), '');
}

console.log('✓ mergeSessionLedger / sessionPreview / formatSessionTime 全分支');

// ---------------------------------------------------------------------------
// 1b. groupHomeSessions / archivedHomeItems — 项目区（工作目录）+ 联系人区（人设）
//     分组、置顶优先、组间活跃倒序；归档组单独供给（2026-10-11 主人拍板批）
// ---------------------------------------------------------------------------

import {
  archivedHomeItems,
  groupHomeSessions,
  personaLabel,
  workspaceLabel,
} from '../src/lib/chat-shell/session-list.ts';

const mkItem = (id, over = {}) => ({
  id,
  title: over.title ?? `会话 ${id}`,
  lastActiveAt: over.lastActiveAt ?? 0,
  messageCount: 1,
  preview: null,
  origin: over.origin ?? 'local',
  pendingApproval: false,
  archived: false,
  pinned: over.pinned ?? false,
  workspace: over.workspace ?? null,
  personaId: over.personaId ?? null,
  personaName: over.personaName ?? null,
});

// 项目分组：按 workspace 原串归组，标签取路径末段；未设归「未关联项目」
{
  const {projects, contacts} = groupHomeSessions([
    mkItem('a', {workspace: 'C:\\Users\\me\\Apeireth-rust', lastActiveAt: 100}),
    mkItem('b', {workspace: 'C:/Users/me/Apeireth-rust', lastActiveAt: 900}),
    mkItem('c', {workspace: null, lastActiveAt: 500}),
  ]);
  assert.equal(projects.length, 3); // 两种写法原串不同 = 诚实各一组；未设再一组
  assert.equal(workspaceLabel('C:\\Users\\me\\Apeireth-rust'), 'Apeireth-rust');
  assert.equal(workspaceLabel(null), '未关联项目');
  assert.equal(workspaceLabel('C:\\'), 'C:');
  // 组间按最新活跃倒序：b(900) 组在前
  assert.equal(projects[0].items[0].id, 'b');
  assert.equal(contacts.length, 1); // 无 persona 全落同一组
  assert.equal(personaLabel(null), '旧会话');
  assert.equal(personaLabel('阿佩瑞斯'), '阿佩瑞斯');
}

// 联系人分组：按 personaId 归组，标签取 personaName；组内置顶优先再活跃倒序
{
  const {contacts} = groupHomeSessions([
    mkItem('x', {personaId: 'p1', personaName: '阿佩瑞斯', lastActiveAt: 100, pinned: true}),
    mkItem('y', {personaId: 'p1', personaName: '阿佩瑞斯', lastActiveAt: 900}),
    mkItem('z', {personaId: 'p2', personaName: '凯尔', lastActiveAt: 300}),
  ]);
  assert.equal(contacts.length, 2);
  const g1 = contacts.find((g) => g.key === 'p1');
  assert.equal(g1.label, '阿佩瑞斯');
  assert.deepEqual(g1.items.map((i) => i.id), ['x', 'y']); // 置顶 x 在前,尽管 y 更新
  // 组间：p1 最新 900 > p2 300 → p1 在前
  assert.equal(contacts[0].key, 'p1');
}

// 归档组：只收 archived 本地会话，活跃倒序，带待签标记
{
  const convs = [
    {...localConv('n1', {updatedAt: 100})},
    {...localConv('a1', {updatedAt: 500, archived: true})},
    {...localConv('a2', {updatedAt: 900, archived: true})},
  ];
  const list = archivedHomeItems(convs, new Set(['a2']));
  assert.deepEqual(list.map((i) => i.id), ['a2', 'a1']);
  assert.equal(list[0].pendingApproval, true);
  assert.equal(list[1].pendingApproval, false);
  assert.equal(archivedHomeItems(convs).length, 2);
}

// 工作区默认回退（2026-10-11 批）：未戳工作区的旧会话归入当前项目组；
// 显式戳了的会话不被覆盖；不传回退仍归「未关联项目」（行为不静默变）
{
  const convs = [
    {...localConv('old1', {updatedAt: 100})}, // 旧数据，无 workspace 戳
    {...localConv('stamped', {updatedAt: 200}), workspace: 'D:\\other\\proj'},
  ];
  const backend = [{id: 'be1', started_at: 300, last_active_at: 300, episode_count: 1}];
  const merged = mergeSessionLedger({
    local: convs,
    backend,
    defaultWorkspace: 'C:\\Users\\me\\Apeireth-rust',
  });
  const byId = new Map(merged.map((i) => [i.id, i]));
  assert.equal(byId.get('old1').workspace, 'C:\\Users\\me\\Apeireth-rust');
  assert.equal(byId.get('stamped').workspace, 'D:\\other\\proj'); // 显式戳记优先
  assert.equal(byId.get('be1').workspace, 'C:\\Users\\me\\Apeireth-rust');
  // 不传 defaultWorkspace：旧行为保持（null → 未关联项目组）
  const legacy = mergeSessionLedger({local: convs, backend});
  const legacyById = new Map(legacy.map((i) => [i.id, i]));
  assert.equal(legacyById.get('old1').workspace, null);
  assert.equal(legacyById.get('be1').workspace, null);
  // 归档组同样吃回退
  const archived = archivedHomeItems(
    [{...localConv('a9', {updatedAt: 50, archived: true})}],
    undefined,
    'C:\\Users\\me\\Apeireth-rust',
  );
  assert.equal(archived[0].workspace, 'C:\\Users\\me\\Apeireth-rust');
  // 回退归组后标签 = 路径末段
  const {projects} = groupHomeSessions(merged);
  const labels = projects.map((g) => g.label);
  assert.ok(labels.includes('Apeireth-rust'));
  assert.ok(labels.includes('proj'));
}

console.log('✓ groupHomeSessions / archivedHomeItems / 分组标签 全分支');

// ---------------------------------------------------------------------------
// 2. parseApprovalEventPayload — SSE approval_required/resolved 帧归一化
//    （payload 字段逐字核实 crates/adapters/gateway/src/events.rs:134-143,292-299）
// ---------------------------------------------------------------------------

{
  const required = parseApprovalEventPayload({
    session: 's-1',
    request: 'r-1',
    trace_id: 't-1',
    approval_id: 'ap-1',
    capability_id: 'tool.shell',
    tool_name: 'tool.shell',
    tool_call_id: 'tc-1',
  });
  assert.deepEqual(required, {
    session: 's-1',
    approvalId: 'ap-1',
    toolName: 'tool.shell',
    decision: null,
  });

  const resolved = parseApprovalEventPayload({
    session: 's-1',
    trace_id: 't-1',
    approval_id: 'ap-1',
    decision: 'approve',
    round: 2,
  });
  assert.equal(resolved.decision, 'approve');

  // 缺主键（session 与 approval_id 皆无）→ null；非对象 → null
  assert.equal(parseApprovalEventPayload({tool_name: 'x'}), null);
  assert.equal(parseApprovalEventPayload(null), null);
  assert.equal(parseApprovalEventPayload('approval_required'), null);
  assert.equal(parseApprovalEventPayload([1, 2]), null);
  // 未知 decision 前向兼容 → null（不猜）
  assert.equal(parseApprovalEventPayload({session: 's', decision: 'maybe'}).decision, null);
}

// applyApprovalEventToPending：required 加标记 / resolved 按 session 清除；不可变入参
{
  const start = new Set(['s-0']);
  const afterRequired = applyApprovalEventToPending(
    start,
    {session: 's-1', approvalId: 'ap-1', toolName: 'tool.shell', decision: null},
    'approval_required',
  );
  assert.deepEqual([...afterRequired].sort(), ['s-0', 's-1']);
  assert.deepEqual([...start], ['s-0']); // 入参不被改动

  const afterResolved = applyApprovalEventToPending(
    afterRequired,
    {session: 's-1', approvalId: 'ap-1', toolName: null, decision: 'reject'},
    'approval_resolved',
  );
  assert.deepEqual([...afterResolved], ['s-0']);
}

console.log('✓ parseApprovalEventPayload / applyApprovalEventToPending 全分支');

// ---------------------------------------------------------------------------
// 3. classifyGovernanceNotice — 守卫通报只接核实存在的真实 code
// ---------------------------------------------------------------------------

{
  // review_rejected（error_codes.rs:21 真实 code）→ 琥珀通报
  const review = classifyGovernanceNotice({backendCode: 'review_rejected', message: '…'});
  assert.equal(review.kind, 'review_rejected');

  // 普通错误一律 null（走 ErrorSolutionBanner，不冒充守卫通报）
  assert.equal(classifyGovernanceNotice({backendCode: 'provider_error'}), null);
  assert.equal(classifyGovernanceNotice({code: 'denied'}), null);
  assert.equal(classifyGovernanceNotice(new Error('network boom')), null);
  assert.equal(classifyGovernanceNotice('plain string'), null);
  assert.equal(classifyGovernanceNotice(null), null);
  assert.equal(classifyGovernanceNotice(undefined), null);
}

console.log('✓ classifyGovernanceNotice：review_rejected 唯一真实信号，其余不冒充');

console.log('--- T0 Chat Shell Logic Check: ALL PASS ---');
