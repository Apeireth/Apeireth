// 「推荐配置」一键预设映射一致性（P0 #5 核心记忆默认开 + 推荐配置预设）。
// import 真实 types.ts / desktop-bridge.ts，并对 SettingsView 能力注册表的
// env 芯片、CLI 旋钮名做源码级镜像校验。
//
// 契约：
//   1. 预设 = 记忆核心族三件（proactiveRecall / preferenceLearning /
//      memoryInjection）+ consolidation / reflexion / organs，共 6 件；
//      **绝不包含** shell / fetch（危险能力不进任何预设）。
//   2. 三个记忆开关与预设映射一致：预设键 ↔ SettingsView env 芯片 ↔
//      CLI `APEIRETH_ENABLE_*` 旋钮 ↔ capabilityEnvFromConfig 注入字段，
//      四处同名对齐。
//   3. 记忆核心族三件默认开：未配 capabilities 时映射为开（与 CLI
//      「未设=开」语义一致），显式 false 才是关。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const srcDir = join(testsDir, '..', 'src');
const repoRoot = join(testsDir, '..', '..', '..');

const {RECOMMENDED_CAPABILITY_PRESET, DEFAULT_CAPABILITY_TOGGLES} = await import('../src/lib/types.ts');
const {capabilityEnvFromConfig} = await import('../src/lib/desktop-bridge.ts');

// 预设键 ↔ canonical CLI 旋钮名（镜像 crates/adapters/cli/src/lib.rs 的 env 常量）。
const CANONICAL_ENV = {
  proactiveRecall: 'APEIRETH_ENABLE_PROACTIVE_RECALL',
  preferenceLearning: 'APEIRETH_ENABLE_PREFERENCE_LEARNING',
  memoryInjection: 'APEIRETH_ENABLE_MEMORY_INJECTION',
  consolidation: 'APEIRETH_ENABLE_CONSOLIDATION',
  reflexion: 'APEIRETH_ENABLE_REFLEXION',
  organs: 'APEIRETH_ENABLE_ORGANS',
};
const MEMORY_CORE_KEYS = ['proactiveRecall', 'preferenceLearning', 'memoryInjection'];
const INJECT_FIELD = {
  proactiveRecall: 'enable_proactive_recall',
  preferenceLearning: 'enable_preference_learning',
  memoryInjection: 'enable_memory_injection',
  consolidation: 'enable_consolidation',
  reflexion: 'enable_reflexion',
  organs: 'enable_organs',
};

console.log('--- Starting Recommended Preset Mapping Check ---');

// ---------------------------------------------------------------------------
// 1. 预设集合恰为 6 件：记忆核心族三件全在；危险项绝不进
// ---------------------------------------------------------------------------
{
  const preset = [...RECOMMENDED_CAPABILITY_PRESET].sort();
  const expected = Object.keys(CANONICAL_ENV).sort();
  assert.deepEqual(preset, expected, `预设必须恰为这 6 件: ${expected}`);
  for (const key of MEMORY_CORE_KEYS) {
    assert.ok(RECOMMENDED_CAPABILITY_PRESET.includes(key), `${key} 必须在推荐配置里`);
  }
  for (const banned of ['shell', 'fetch', 'localReadTools']) {
    assert.ok(!RECOMMENDED_CAPABILITY_PRESET.includes(banned), `${banned} 绝不进推荐配置`);
  }
  console.log('  -> PASS: 推荐配置 = 记忆核心三件 + 固化/反思/器官链，无 shell/fetch');
}

// ---------------------------------------------------------------------------
// 2. 默认值：记忆核心族三件默认开，其余开关默认值不动
// ---------------------------------------------------------------------------
{
  for (const key of MEMORY_CORE_KEYS) {
    assert.equal(DEFAULT_CAPABILITY_TOGGLES[key], true, `${key} 默认必须是开`);
  }
  for (const key of ['consolidation', 'reflexion', 'organs']) {
    assert.equal(DEFAULT_CAPABILITY_TOGGLES[key], false, `${key} 默认必须保持关`);
  }
  for (const key of ['shell', 'fetch']) {
    assert.equal(DEFAULT_CAPABILITY_TOGGLES[key], false, `${key} 危险项默认必须保持关`);
  }
  console.log('  -> PASS: 记忆核心族默认开；consolidation/reflexion/organs/shell/fetch 默认关不动');
}

// ---------------------------------------------------------------------------
// 3. SettingsView 能力注册表 env 芯片 ↔ CLI 旋钮名逐键一致
// ---------------------------------------------------------------------------
{
  const settingsSrc = readFileSync(join(srcDir, 'lib/views/SettingsView.svelte'), 'utf8');
  for (const [key, env] of Object.entries(CANONICAL_ENV)) {
    const match = new RegExp(`key: '${key}'[^}]*env: '([A-Z_]+)'`).exec(settingsSrc);
    assert.ok(match, `SettingsView 能力注册表必须有 ${key} 行`);
    assert.equal(match[1], env, `${key} 的 env 芯片必须等于 CLI 旋钮名 ${env}`);
  }
  // 同名也必须出现在 CLI 侧（旋钮解析 + 推荐配置镜像测试）。
  const cliLib = readFileSync(join(repoRoot, 'crates/adapters/cli/src/lib.rs'), 'utf8');
  const cliKnobs = readFileSync(join(repoRoot, 'crates/adapters/cli/tests/production_knobs.rs'), 'utf8');
  for (const [key, env] of Object.entries(CANONICAL_ENV)) {
    assert.ok(cliLib.includes(env), `cli lib.rs 必须解析 ${env} (${key})`);
    assert.ok(cliKnobs.includes(env), `cli production_knobs 必须镜像 ${env} (${key})`);
  }
  console.log('  -> PASS: 预设键/env 芯片/CLI 旋钮名三处同名对齐');
}

// ---------------------------------------------------------------------------
// 4. capabilityEnvFromConfig：预设应用后六件注入为开，shell/fetch 不被带起；
//    记忆核心族未配 = 开（与 CLI「未设=开」对齐），显式 false = 关
// ---------------------------------------------------------------------------
{
  const toggles = {...DEFAULT_CAPABILITY_TOGGLES};
  for (const key of RECOMMENDED_CAPABILITY_PRESET) toggles[key] = true;
  const env = capabilityEnvFromConfig(toggles);
  for (const key of RECOMMENDED_CAPABILITY_PRESET) {
    assert.equal(env[INJECT_FIELD[key]], true, `预设应用后 ${INJECT_FIELD[key]} 必须为开`);
  }
  assert.equal(env.enable_shell, false, '推荐配置绝不带起 shell');
  assert.equal(env.enable_fetch, false, '推荐配置绝不带起 fetch');

  const unset = capabilityEnvFromConfig(undefined);
  for (const key of MEMORY_CORE_KEYS) {
    assert.equal(unset[INJECT_FIELD[key]], true, `未配 capabilities 时 ${INJECT_FIELD[key]} 缺省 = 开`);
  }
  assert.equal(unset.enable_consolidation, false, 'consolidation 缺省仍为关');

  const off = capabilityEnvFromConfig({...DEFAULT_CAPABILITY_TOGGLES, proactiveRecall: false, preferenceLearning: false, memoryInjection: false});
  for (const key of MEMORY_CORE_KEYS) {
    assert.equal(off[INJECT_FIELD[key]], false, `显式 false 必须映射为关: ${key}`);
  }
  console.log('  -> PASS: 预设映射到注入字段一致；缺省=开、显式 false=关');
}

console.log('--- All Recommended Preset Mapping Checks PASSED! ---');
