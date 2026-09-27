// 能力开关「即点即生效」映射一致性（P0 修复：能力开关不生效）。
// import 真实 types.ts / capability-apply.ts / desktop-bridge.ts，并对
// SettingsView / App 的即点即生效接线做源码级镜像校验。
//
// 契约：
//   1. 拨动开关 → capability-apply 拼出待推送配置 → capabilityEnvFromConfig
//      注入字段正确：Shell 开 = enable_shell:true、关 = false（fail-closed）。
//   2. 沙箱子开关 shellSandbox:false → shell_sandbox_off:true（显式裸跑）；
//      默认 true → false（不注入 = 后端默认沙箱开）。
//   3. handleCapabilityToggle 必须拨动即走 apply（onSave），不允许只改本地
//      状态等远处的保存按钮；推送失败把开关拨回原值 + 错误横幅。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const srcDir = join(testsDir, '..', 'src');

const {DEFAULT_CAPABILITY_TOGGLES} = await import('../src/lib/types.ts');
const {toggledCapability, configWithCapabilities} = await import('../src/lib/capability-apply.ts');
const {capabilityEnvFromConfig} = await import('../src/lib/desktop-bridge.ts');

console.log('--- Starting Capability Toggle Apply-On-Click Check ---');

// ---------------------------------------------------------------------------
// 1. 开关 → 待推送配置 → 注入字段：Shell 开关的完整映射链
// ---------------------------------------------------------------------------
{
  const base = {...DEFAULT_CAPABILITY_TOGGLES};
  assert.equal(base.shell, false, 'Shell 开关默认关（fail-closed）');

  const on = toggledCapability(base, 'shell', true);
  assert.equal(on.shell, true, '拨动后开关必须为开');
  assert.equal(base.shell, false, 'toggledCapability 不可变更新，原值不动');

  const cfg = configWithCapabilities({baseUrl: 'http://127.0.0.1:1', model: 'm'}, on);
  assert.equal(cfg.capabilities.shell, true, '待推送配置必须携带开关后的值');
  assert.equal(cfg.baseUrl, 'http://127.0.0.1:1', '其余配置字段不被覆盖');

  const env = capabilityEnvFromConfig(cfg.capabilities);
  assert.equal(env.enable_shell, true, '开关开 → enable_shell 必须为 true');
  assert.equal(env.enable_shell, cfg.capabilities.shell === true, '注入字段与开关同真同假');

  const off = capabilityEnvFromConfig(toggledCapability(on, 'shell', false));
  assert.equal(off.enable_shell, false, '开关关 → enable_shell 必须为 false');
  console.log('  -> PASS: 拨动 Shell 开关 → capabilityEnvFromConfig → enable_shell 正确');
}

// ---------------------------------------------------------------------------
// 2. 沙箱子开关（shellSandbox）同语义：反向注入映射
// ---------------------------------------------------------------------------
{
  const sandboxed = capabilityEnvFromConfig(DEFAULT_CAPABILITY_TOGGLES);
  assert.equal(sandboxed.shell_sandbox_off, false, '默认沙箱开 → 不注入（后端默认沙箱开）');

  const raw = capabilityEnvFromConfig(toggledCapability(DEFAULT_CAPABILITY_TOGGLES, 'shellSandbox', false));
  assert.equal(raw.shell_sandbox_off, true, '显式关沙箱 → shell_sandbox_off 必须为 true');

  const back = capabilityEnvFromConfig(toggledCapability(DEFAULT_CAPABILITY_TOGGLES, 'shellSandbox', true));
  assert.equal(back.shell_sandbox_off, false, '拨回开 → shell_sandbox_off 必须回 false');
  console.log('  -> PASS: 沙箱子开关 ↔ shell_sandbox_off 映射正确');
}

// ---------------------------------------------------------------------------
// 3. 源码级镜像：handleCapabilityToggle 即点即生效（含 pending / 失败回滚）
// ---------------------------------------------------------------------------
{
  const settingsSrc = readFileSync(join(srcDir, 'lib/views/SettingsView.svelte'), 'utf8');

  const toggleRegion = /function handleCapabilityToggle[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(toggleRegion, 'SettingsView 必须有 handleCapabilityToggle');
  assert.ok(
    toggleRegion[0].includes('applyCapabilityNow('),
    'handleCapabilityToggle 拨动后必须立即走 apply（不允许只改本地状态等保存按钮）',
  );

  const applyRegion = /async function applyCapabilityNow[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(applyRegion, 'SettingsView 必须有 applyCapabilityNow（即点即生效 apply 缝）');
  assert.ok(
    applyRegion[0].includes('await onSave(configWithCapabilities(config, attempted))'),
    'applyCapabilityNow 必须走与保存按钮同一条 onSave apply 路径',
  );
  assert.ok(applyRegion[0].includes('liveApplyPendingKey = key'), '必须带 pending 态');
  assert.ok(
    applyRegion[0].includes('[key]: previous[key]'),
    '推送失败必须把开关拨回原值（回滚）',
  );
  assert.ok(
    applyRegion[0].includes('liveApplyError = errorBannerFrom(err)'),
    '推送失败必须亮错误横幅',
  );

  // 保存按钮保留：批量项仍走 handleSaveSettings。
  assert.ok(settingsSrc.includes('function handleSaveSettings'), '保存按钮路径（批量项）必须保留');
  console.log('  -> PASS: handleCapabilityToggle 即点即生效 + pending + 失败回滚接线在案');
}

// ---------------------------------------------------------------------------
// 4. 源码级镜像：App 侧 onSave 返回 apply 的 Promise（失败可回滚）
// ---------------------------------------------------------------------------
{
  const appSrc = readFileSync(join(srcDir, 'App.svelte'), 'utf8');
  assert.ok(
    appSrc.includes('await pushProviderEnvAndRefresh(newCfg);'),
    'App 的 onSave 必须等待 pushProviderEnvAndRefresh（apply 结果回传给开关回滚）',
  );
  assert.ok(
    appSrc.includes('applyBackendConfigOrThrow(provider, capabilities)'),
    '推送必须用失败可感知的 apply 入口（IPC 失败拒绝 → 回滚）',
  );
  console.log('  -> PASS: onSave → pushProviderEnvAndRefresh 失败可回传');
}

console.log('--- All Capability Toggle Apply-On-Click Checks PASSED! ---');
