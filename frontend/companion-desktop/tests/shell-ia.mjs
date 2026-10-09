// 壳层接线源码镜像（App.svelte / Workbench.svelte / SettingsView.svelte /
// SessionListHome.svelte / presence.ts）——.svelte 无法被 node 直接 import，
// 接线事实用源码断言钉死（同一手法见 settings-save-buttons-removed.mjs）。
//
// 覆盖（内测反馈批）：
//   ① 删除链接线（乐观移除 / 后端真删 / 失败帧 / 陈账排除）
//   ② 打开会话滚底接线（打开/切换滚底、追加不拽回）
//   ③ 会话头「⊕ 新会话」接线（新开 + 聚焦输入框）+ 人名/模型下拉并入面板
//   ④ 侧栏四件化接线（入口迁移 / 深链重定向 / 命令面板同步 / 记录日记工作台
//      入口 / 治理设置区 / 日志并入状态）
//   ⑤ SSE 重连接线（端点变了重建订阅 + 徽章点击真重连 + 构造抛出不死链）
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const src = (p) => readFileSync(join(here, '..', 'src', p), 'utf8');

const app = src('App.svelte');
const workbench = src('lib/components/Workbench.svelte');
const settings = src('lib/views/SettingsView.svelte');
const homeList = src('lib/chat-shell/SessionListHome.svelte');
const sessionList = src('lib/chat-shell/session-list.ts');
const presence = src('lib/presence.ts');

let checks = 0;
const check = (name, fn) => {
  fn();
  checks += 1;
  console.log(`  ok  ${name}`);
};

console.log('--- shell wiring (source mirrors) ---');

// ---- ① 删除链 ----
check('删除链：乐观移除 + 后端真删 + 失败亮帧接线', () => {
  assert.ok(app.includes('await deleteSession(id, {'), '删除走真删链编排器');
  assert.ok(
    app.includes('deleteRemote: (sid) => deleteBackendSession(config, sid)'),
    '后端真删打 DELETE /v1/sessions/{id}（重启不复活的根因面）',
  );
  assert.ok(app.includes('<div class="action-error-frame">'), '失败即亮错误帧');
  assert.ok(app.includes('ErrorSolutionBanner'), '错误帧复用既有错误帧组件');
  assert.ok(app.includes('deletedSessionIds = restored'), '失败回滚：乐观排除一并撤销');
  assert.ok(app.includes('if (wasActive) activeId = id;'), '失败回滚：被删会话的活动视图一并复原');
});

check('删除链：后端陈账行乐观排除 + 列表刷新', () => {
  assert.ok(app.includes('deletedSessions={deletedSessionIds}'), '删除 id 传给会话列表做乐观排除');
  assert.ok(homeList.includes('exclude: deletedSessions ?? undefined'), '列表归并吃排除集');
  assert.ok(sessionList.includes('if (exclude.has(s.id)) continue;'), '归并纯函数跳过已删除 id');
  assert.ok(app.includes('homeReloadKey += 1'), '删除成功后对齐一次后端账本（列表刷新）');
});

// ---- ② 滚底 ----
check('滚动：打开/切换会话滚底，追加不拽回', () => {
  assert.ok(app.includes('scrollOnOpen(kind)'), '打开/切换走 scrollOnOpen（恒落底）');
  assert.ok(app.includes('scrollOnAppend(isNearBottom)'), '流式追加按贴底判定跟随/保持');
  assert.ok(app.includes('landAtBottom(container)'), '打开/切换落底走同步落底（首帧即贴底）');
});

// ---- ③ 新会话按钮 + 人名下拉让位 ----
check('会话头「⊕ 新会话」：新开 + 聚焦输入框', () => {
  assert.ok(app.includes('class="new-session-btn"'), '会话头圆角深底按钮（⊕ 图标 + 文字）');
  assert.ok(app.includes('onclick={startNewSession}'), '按钮接 startNewSession');
  const body = app.match(/function startNewSession\(\): void \{([\s\S]{0,400}?)\n {2}\}/);
  assert.ok(body, 'startNewSession 实现存在');
  assert.ok(body[1].includes('newConversation({askWorkspace: false})'), '点击 = 新开会话');
  assert.ok(body[1].includes('composerTextarea?.focus()'), '点击 = 聚焦输入框');
});

check('人名/模型下拉让位并入「模型与上下文」面板（不丢功能）', () => {
  assert.ok(!app.includes('persona-trigger'), '会话头不再挂人名下拉');
  assert.ok(app.includes('伙伴身份'), '面板里有伙伴身份入口');
  assert.ok(app.includes('setActivePersona(p.id)'), '切换人设（连带默认模型）功能保留');
});

// ---- ④ 侧栏四件化 ----
check('侧栏常驻四件：对话/工具/状态/设置', () => {
  const labels = [...app.matchAll(/class="rail-label">([^<]+)</g)].map((m) => m[1]);
  assert.deepEqual(labels, ['对话', '工具', '状态', '设置'], `侧栏标签 = ${labels.join('/')}`);
});

check('历史入口撤除（会话列表即历史）', () => {
  assert.ok(!app.includes('nav.history'), '命令面板 nav.history 已撤');
  assert.ok(!app.includes('ConversationsView'), '历史页不再进壳层（列表栏即历史）');
  assert.ok(app.includes('<SessionListHome'), '会话列表常驻 = 历史的家');
  assert.ok(homeList.includes('class="home-search"'), '列表带搜索框（会话列表 + 搜索即历史）');
  assert.ok(homeList.includes('sessionMatchesQuery(item, searchQuery)'), '搜索过滤已接线');
});

check('深链重定向：被移页面跳新位置', () => {
  assert.ok(app.includes('resolveLegacyDrawer(drawerQuery)'), '旧深链走重定向表');
  assert.ok(
    app.includes("applyShellTarget(NAV_COMMAND_TARGETS['nav.governance'])"),
    '命令面板与深链同一张目标表',
  );
  assert.ok(app.includes("function navRun(id: string)"), 'nav.* 执行闭包统一改道');
});

check('记忆/日记移入工作台（卡片入口 + 面板组件复用）', () => {
  assert.ok(workbench.includes('WORKBENCH_CARDS'), '工作台两张卡片入口');
  assert.ok(workbench.includes('class="wb-card"'), '卡片样式（圆角深底）');
  assert.ok(workbench.includes("import MemoryView from '../MemoryView.svelte'"), '记忆面板组件复用不重写');
  assert.ok(workbench.includes("import DiaryView from '../views/DiaryView.svelte'"), '日记面板组件复用不重写');
  assert.ok(workbench.includes('<MemoryView {config} {capabilities} />'), '记忆面板真渲染');
  assert.ok(workbench.includes('<DiaryView />'), '日记面板真渲染');
  assert.ok(app.includes('section={wbSection}'), '壳层驱动工作台分区');
  assert.ok(app.includes('onSelectSection='), '卡片切换分区已接线');
});

check('治理移入设置「安全与治理」区（面板组件复用）', () => {
  assert.ok(settings.includes("label: '安全与治理'"), '设置新增安全与治理分区');
  assert.ok(settings.includes("activeSection === 'security'"), '分区渲染分支存在');
  assert.ok(settings.includes('<GovernanceView'), '治理面板组件复用不重写');
  assert.ok(settings.includes('initialTab={initialGovernanceTab}'), '守卫入口指令式落 tab 仍生效');
});

check('日志并入状态（组件复用）', () => {
  assert.ok(app.includes('活动与调用日志'), '状态抽屉含日志区块');
  assert.ok(app.includes('<ActivityView {config} {capabilities} />'), '活动视图组件复用不重写');
  assert.ok(!app.includes("rail-label\">日志"), '侧栏不再单列日志');
});

// ---- ⑤ SSE 重连 ----
check('SSE：端点变了重建订阅（不再冻在首次 URL）', () => {
  assert.ok(app.includes('base !== presenceBaseUrl'), 'baseUrl 变化触发重建订阅');
  assert.ok(app.includes('unsubscribePresence = subscribePresence(config.baseUrl)'), '新订阅用当前端点');
  assert.ok(!app.includes('let presenceStarted'), '一次性闩已拆除（端点变了会重建）');
});

check('SSE：徽章点击 = 真重建事件订阅（说到做到）', () => {
  const click = app.match(/onSseClick=\{\(\) => \{([\s\S]{0,300}?)\}\}/);
  assert.ok(click, 'onSseClick 处理器存在');
  assert.ok(click[1].includes('restartPresenceSubscription()'), '点击真重建订阅');
  assert.ok(click[1].includes('refreshConnection()'), '点击顺带健康检查');
});

check('SSE：EventSource 构造抛出不死链', () => {
  assert.ok(
    /try \{\s*es = new EventSource\(url\);\s*\} catch \{/.test(presence),
    '构造抛出被接住并按退避续排（链路不死）',
  );
});

console.log(`${checks} shell wiring checks passed`);
