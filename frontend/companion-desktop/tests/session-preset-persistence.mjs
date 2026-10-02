// 会话权限档持久化接线源码镜像（App.svelte / SettingsView.svelte + 治理同源的
// Rust 源码镜像）——.svelte 无法被 node 直接 import，接线事实用源码断言钉死
// （同一手法见 shell-ia.mjs / settings-save-buttons-removed.mjs）。
//
// 覆盖（会话权限档持久化核查批）：
//   ① 选档即持久：输入栏选档走既有会话 settings PATCH 面（不另立通道）
//   ② 重开按会话恢复：切回/重开会话按会话读回同一档（GET 会话 settings）
//   ③ 新会话继承全局默认档：创建时读全局预设（localStorage），首个回合后投递落库
//   ④ 回归钉：读回 settings 不撤销未投递初始档（心跳重拉曾把继承静默冲掉）；
//      重开既有后端会话不 mark 初始档（不覆盖按会话恢复的真值）；
//      会话创建前的显式选档失败留待投递（PATCH 404 不得丢选择）
//   ⑤ 治理同源：权限预设治理钩子与会话存储共用同一份持久化档（不读进程 env）
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
// 源码镜像断言按 \n 写多行片段；Windows 检出的 CRLF 先归一，避免行尾差异假失败。
const read = (p) => readFileSync(p, 'utf8').replace(/\r\n/g, '\n');
const src = (p) => read(join(here, '..', 'src', p));
const workspaceSrc = (p) => read(join(here, '..', '..', '..', ...p.split('/')));

const app = src('App.svelte');
const settings = src('lib/views/SettingsView.svelte');
const runtime = src('lib/runtime.ts');

/** 取一个 2 空格缩进函数的函数体（源码镜像抽取，同 settings-live-apply.mjs 手法）。 */
function body(fnName) {
  const m = app.match(new RegExp(`function ${fnName}\\([^)]*\\)[^{]*\\{([\\s\\S]*?)\\n  \\}`));
  assert.ok(m, `必须有 ${fnName} 实现`);
  return m[1];
}

let checks = 0;
const check = (name, fn) => {
  fn();
  checks += 1;
  console.log(`  ok  ${name}`);
};

console.log('--- session approval tier persistence (source mirrors) ---');

// ---- ① 选档即持久（既有会话 settings PATCH 面）----
check('输入栏选档接线：弹层选择 → selectSessionPreset', () => {
  assert.ok(app.includes('onclick={() => void pickComposerPreset(preset.id)}'), '档位弹层项接 pickComposerPreset');
  const picker = body('pickComposerPreset');
  assert.ok(picker.includes('await selectSessionPreset(preset)'), '选择即走 selectSessionPreset（不做组件本地 state 了事）');
});

check('选档即持久：PATCH /v1/sessions/{id}/settings 带 permission_preset + approval_remember', () => {
  const select = body('selectSessionPreset');
  assert.ok(
    select.includes('await patchSessionSettings(config, sessionId, {'),
    '选档打既有会话 settings PATCH 面（不发明新通道）',
  );
  assert.ok(select.includes('permission_preset: strategy.permission_preset'), '档位进 PATCH body');
  assert.ok(select.includes('approval_remember: strategy.approval_remember'), '会话内记住开关随档进 PATCH body');
  assert.ok(
    runtime.includes('/v1/sessions/${encodeURIComponent(sessionId)}/settings'),
    'runtime.ts 的 PATCH 面指向会话 settings 端点',
  );
  assert.ok(runtime.includes("{method: 'PATCH', body: patch}"), 'patchSessionSettings 真发 PATCH');
});

// ---- ② 重开按会话恢复 ----
check('重开按会话恢复：会话切换/激活即 GET 会话 settings 并回填', () => {
  assert.ok(
    app.includes('sessionSettings = null;\n    void loadSessionSettings(id);'),
    'activeId 变化（切换/重开）即拉取会话级设置',
  );
  const load = body('loadSessionSettings');
  assert.ok(load.includes('await getSessionSettings(config, sessionId)'), '读回走既有 GET 会话 settings 面');
  assert.ok(load.includes('sessionSettings = settings'), '读回结果回填会话设置（按会话恢复的显示真值）');
  assert.ok(
    app.includes("const preset = sessionSettings?.permission_preset ?? pending?.permission_preset ?? 'standard';"),
    '档位芯片 active 态由会话设置驱动（未就绪才回落待投递初始档）',
  );
});

// ---- ③ 新会话继承全局默认档 ----
check('新会话继承全局默认档：创建时读全局预设，首回合后投递落库', () => {
  assert.ok(
    app.includes("const GLOBAL_PRESET_KEY = 'apeireth-permission-preset-default';"),
    '全局预设键与设置页同键',
  );
  assert.ok(settings.includes("const PERMISSION_PRESET_KEY = 'apeireth-permission-preset-default';"), '设置页同键（同源）');
  assert.ok(settings.includes('localStorage.setItem(PERMISSION_PRESET_KEY, permissionPreset)'), '设置页把全局默认档写进 localStorage');
  const readGlobal = body('readGlobalPreset');
  assert.ok(readGlobal.includes('localStorage.getItem(GLOBAL_PRESET_KEY)'), '新会话继承读的就是这份全局预设');
  const mark = body('markPendingPreset');
  assert.ok(mark.includes('const preset = readGlobalPreset();'), '创建时取全局默认档为待投递初始档');
  assert.ok(mark.includes('pendingPreset = {...pendingPreset, [conversationId]: {permission_preset: preset}}'), '初始档挂到新会话名下');
  for (const entry of ['ensureConversation', 'newConversation', 'branchFromMessage']) {
    assert.ok(body(entry).includes('markPendingPreset('), `${entry} 创建会话即 mark 初始档`);
  }
  const apply = body('applyPendingPreset');
  assert.ok(apply.includes('const updated = await patchSessionSettings(config, conversationId, patch);'), '投递走既有会话 settings PATCH 面');
  assert.ok(apply.includes('permission_preset: preset.permission_preset'), '投递内容是权限档');
  assert.ok(app.includes('await applyPendingPreset(conversationId);'), 'send() 收束后投递');
});

// ---- ④ 回归钉 ----
check('回归钉：读回 settings 不撤销未投递初始档（心跳重拉不冲掉继承）', () => {
  const load = body('loadSessionSettings');
  assert.ok(
    !load.includes('clearPendingPreset'),
    'loadSessionSettings 不得清 pendingPreset——后端会在回合开始按默认档自动建会话，读回成功 ≠ 用户设置过',
  );
  assert.ok(
    !load.includes('sessionsWithSettings'),
    '不得再用"读回过 settings"当覆盖守卫（那正是长回合里继承失效的根因）',
  );
  const apply = body('applyPendingPreset');
  assert.ok(!apply.includes('sessionsWithSettings'), '投递缝只认待投递初始档的存在与否');
});

check('回归钉：重开既有后端会话不 mark 初始档（全局默认不盖按会话恢复的真值）', () => {
  const open = body('openHomeSession');
  assert.ok(
    !open.includes('markPendingPreset'),
    'openHomeSession 打开的是既有后端会话，其持久档是真值，不得挂全局默认初始档',
  );
});

check('回归钉：会话创建前的显式选档失败留待投递（PATCH 404 不丢选择）', () => {
  const select = body('selectSessionPreset');
  assert.ok(select.includes('const sessionNotCreatedYet ='), '识别"后端会话尚未创建"的失败');
  assert.ok(
    select.includes('[sessionId]: {\n          permission_preset: strategy.permission_preset,'),
    '失败时把显式选档暂存为待投递初始档',
  );
});

// ---- ⑤ 治理同源 ----
check('治理同源：权限预设治理钩子读的是同一份持久化档（不读进程 env）', () => {
  const hook = workspaceSrc('crates/engine/runtime-assembly/src/canonical/permission_preset.rs');
  assert.ok(hook.includes('let loaded = match self.sessions.load(&request.session).await'), '钩子从会话存储读档');
  assert.ok(hook.includes('session.settings.permission_preset'), '消费的是会话持久化设置里的档');
  assert.ok(hook.includes('session.settings.approval_remember'), '会话内记住开关同源');
  const cli = workspaceSrc('crates/adapters/cli/src/lib.rs');
  assert.ok(
    /PermissionPresetGovernanceHook::new\(\s*Arc::new\(governance\),\s*Arc::clone\(&session_store\),/.test(cli),
    '生产装配里钩子与运行时共用同一会话存储',
  );
  assert.ok(cli.includes('builder = builder.with_session_store(session_store);'), '运行时的会话存储同源');
});

console.log(`--- ${checks} checks passed ---`);
