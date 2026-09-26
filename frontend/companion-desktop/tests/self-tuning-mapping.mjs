// 「性格养成」体验参数映射一致性（性格养成第一铲）。
// import 真实 types.ts / desktop-bridge.ts / runtime.ts，并对 SettingsView
// 滑杆与预设、Rust 侧 backend_supervisor.rs / self_tuning.rs 做源码级镜像校验。
//
// 契约：
//   1. 「从使用中学习」默认关（selfTuning === false，fail-closed）。
//   2. 四个体验旋钮 env 名四处同名对齐：SettingsView env 芯片 ↔
//      capabilityEnvFromConfig 注入字段 ↔ backend_supervisor env_pairs ↔
//      self_tuning.rs 引擎侧解析。自学习开关 = APEIRETH_ENABLE_SELF_TUNING。
//   3. 基线/上下界与 Rust 侧同数：基线 1.0/1.0/1.0/1，范围
//      [0.25,4] / [0.25,4] / [0,2] / [1,10]。
//   4. 偏离值正确映射；未配 = 基线 + 自学习关（fail-closed）；越界钳制。
//   5. 预设恰为 省心/均衡/深度记忆 三档（只动 4 个体验参数），恢复基线 = 基线
//      + 自学习关。
//   6. 滑杆 min/max/step 与参数表逐项一致（源码镜像）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const srcDir = join(testsDir, '..', 'src');
const repoRoot = join(testsDir, '..', '..', '..');

// localStorage 内存 shim（同 config-persistence.mjs）：runtime.ts 顶层不触 DOM，
// 动态 import 保证 shim 先于任何 loadConfig/saveConfig 调用就位。
const store = new Map();
globalThis.localStorage = {
  getItem: (k) => (store.has(k) ? store.get(k) : null),
  setItem: (k, v) => void store.set(k, String(v)),
  removeItem: (k) => void store.delete(k),
  clear: () => store.clear(),
};

const {DEFAULT_CAPABILITY_TOGGLES} = await import('../src/lib/types.ts');
const {capabilityEnvFromConfig} = await import('../src/lib/desktop-bridge.ts');
const {loadConfig, saveConfig} = await import('../src/lib/runtime.ts');

// 四个体验旋钮：env 名 / 注入字段 / 基线 / 取值域（镜像 Rust 侧 self_tuning.rs
// 与契约表；label = SettingsView 滑杆 aria-label，chip = env 芯片文案）。
const KNOBS = {
  memoryFade: {
    env: 'APEIRETH_TUNE_MEMORY_FADE',
    field: 'tune_memory_fade',
    baseline: 1.0,
    min: 0.25,
    max: 4.0,
    step: 0.25,
    label: '遗忘衰减强度',
    sliderAttrs: ['min="0.25"', 'max="4"', 'step="0.25"'],
  },
  curiosityStrength: {
    env: 'APEIRETH_TUNE_CURIOSITY_STRENGTH',
    field: 'tune_curiosity_strength',
    baseline: 1.0,
    min: 0.25,
    max: 4.0,
    step: 0.25,
    label: '好奇心强度',
    sliderAttrs: ['min="0.25"', 'max="4"', 'step="0.25"'],
  },
  toneSaturation: {
    env: 'APEIRETH_TUNE_TONE_SATURATION',
    field: 'tune_tone_saturation',
    baseline: 1.0,
    min: 0.0,
    max: 2.0,
    step: 0.2,
    label: '语气情绪饱和度',
    sliderAttrs: ['min="0"', 'max="2"', 'step="0.2"'],
  },
  consolidationCadence: {
    env: 'APEIRETH_TUNE_CONSOLIDATION_CADENCE',
    field: 'tune_consolidation_cadence',
    baseline: 1,
    min: 1,
    max: 10,
    step: 1,
    label: '整合节奏',
    sliderAttrs: ['min="1"', 'max="10"', 'step="1"'],
  },
};
const KNOB_KEYS = Object.keys(KNOBS);
const TUNE_ENVS = KNOB_KEYS.map((k) => KNOBS[k].env);
const SELF_TUNING_ENV = 'APEIRETH_ENABLE_SELF_TUNING';
const TUNE_FIELDS = KNOB_KEYS.map((k) => KNOBS[k].field).concat('enable_self_tuning');

// 数值字面量容差：接受 4.0 / 4 / 4. 等写法（Rust f64 字面量风格不锁死）。
const hasNum = (src, lit) =>
  src.includes(lit) || src.includes(lit.replace(/\.0$/, '')) || src.includes(lit.replace(/\.0$/, '.'));

// 源码对象字面量 → 可解析 JSON（仅测试解析用）：单引号键换双引号，
// 裸标识符键（memoryFade: 1.5）补双引号，尾逗号去掉。
const parseSourceObject = (raw) =>
  JSON.parse(
    raw
      .replace(/'/g, '"')
      .replace(/([{,]\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:/g, '$1"$2":')
      .replace(/,(\s*[}\]])/g, '$1'),
  );

const settingsSrc = readFileSync(join(srcDir, 'lib/views/SettingsView.svelte'), 'utf8');

console.log('--- Starting Self-Tuning Mapping Check ---');

// ---------------------------------------------------------------------------
// 1. 默认值：四旋钮恰为基线（1.0/1.0/1.0/1），自学习默认关（重设计·轻默认）
// ---------------------------------------------------------------------------
{
  for (const key of KNOB_KEYS) {
    assert.equal(
      DEFAULT_CAPABILITY_TOGGLES[key],
      KNOBS[key].baseline,
      `${key} 默认必须是基线 ${KNOBS[key].baseline}（未设 = 现行为，零变化）`,
    );
  }
  assert.equal(DEFAULT_CAPABILITY_TOGGLES.selfTuning, false, '「从使用中学习」默认必须关（fail-closed）');
  console.log('  -> PASS: 四旋钮默认 = 基线；selfTuning 默认关（自学习 fail-closed）');
}

// ---------------------------------------------------------------------------
// 2. env 名一致性：SettingsView env 芯片 ↔ 注入字段 ↔ backend_supervisor env_pairs
// ---------------------------------------------------------------------------
{
  for (const key of KNOB_KEYS) {
    const {env} = KNOBS[key];
    assert.ok(settingsSrc.includes(`>${env}</code>`), `SettingsView 必须带 env 芯片 ${env} (${key})`);
  }
  assert.ok(
    settingsSrc.includes(`>${SELF_TUNING_ENV}</code>`),
    `SettingsView 必须带 env 芯片 ${SELF_TUNING_ENV}（从使用中学习）`,
  );

  const env = capabilityEnvFromConfig(DEFAULT_CAPABILITY_TOGGLES);
  const tuneFields = Object.keys(env)
    .filter((k) => k.startsWith('tune_') || k === 'enable_self_tuning')
    .sort();
  assert.deepEqual(tuneFields, [...TUNE_FIELDS].sort(), `注入字段必须恰好是 ${TUNE_FIELDS.join(' / ')}`);
  for (const key of KNOB_KEYS) {
    assert.equal(typeof env[KNOBS[key].field], 'number', `${KNOBS[key].field} 必须恒发数值`);
  }
  assert.equal(typeof env.enable_self_tuning, 'boolean', 'enable_self_tuning 必须是布尔');

  // Rust 侧注入源（backend_supervisor.rs 的 capability env_pairs 实现体）：
  // 五个 env 字符串必须真的出现在 env_pairs 里（不是只在文档注释里提到），
  // 且 TUNE 族恰好这四个名。
  const supervisorSrc = readFileSync(join(testsDir, '..', 'src-tauri/src/backend_supervisor.rs'), 'utf8');
  const envPairsIdx = supervisorSrc.lastIndexOf('fn env_pairs');
  assert.ok(envPairsIdx > 0, 'backend_supervisor 必须有 capability env_pairs 实现');
  const nextFn = supervisorSrc.indexOf('\n    pub fn ', envPairsIdx);
  const envPairsBody = supervisorSrc.slice(envPairsIdx, nextFn === -1 ? supervisorSrc.length : nextFn);
  const supervisorTune = [...new Set(envPairsBody.match(/APEIRETH_TUNE_[A-Z_]+/g) ?? [])].sort();
  assert.deepEqual(supervisorTune, [...TUNE_ENVS].sort(), `backend_supervisor env_pairs 必须恰好注入这四个 ${TUNE_ENVS.join(' / ')}`);
  for (const envName of TUNE_ENVS) {
    assert.ok(envPairsBody.includes(envName), `backend_supervisor env_pairs 必须注入 ${envName}`);
  }
  assert.ok(envPairsBody.includes(SELF_TUNING_ENV), `backend_supervisor env_pairs 必须注入 ${SELF_TUNING_ENV}`);
  console.log('  -> PASS: env 芯片/注入字段/后端 env_pairs 三处同名对齐（含自学习开关）');
}

// ---------------------------------------------------------------------------
// 3. Rust 引擎侧镜像：self_tuning.rs 声明四个 env 名 + 同一组基线/上下界数值
// ---------------------------------------------------------------------------
{
  const engineSrc = readFileSync(join(repoRoot, 'crates/foundation/orchestration/src/self_tuning.rs'), 'utf8');
  const engineTune = [...new Set(engineSrc.match(/APEIRETH_TUNE_[A-Z_]+/g) ?? [])].sort();
  assert.deepEqual(engineTune, [...TUNE_ENVS].sort(), `self_tuning.rs 必须恰好声明这四个 ${TUNE_ENVS.join(' / ')}`);
  for (const lit of ['1.0', '0.25', '4.0', '2.0', '10.0']) {
    assert.ok(hasNum(engineSrc, lit), `self_tuning.rs 必须出现基线/边界数值 ${lit}（与前端参数表同数）`);
  }
  console.log('  -> PASS: 引擎侧 env 名与基线/上下界数值（1.0/0.25/4.0/2.0/10.0）同数镜像');
}

// ---------------------------------------------------------------------------
// 4. capabilityEnvFromConfig 映射：偏离值直传；未配 = 基线 + 自学习关；越界钳制
// ---------------------------------------------------------------------------
{
  const deviating = {
    ...DEFAULT_CAPABILITY_TOGGLES,
    memoryFade: 0.5,
    curiosityStrength: 1.5,
    toneSaturation: 0.8,
    consolidationCadence: 3,
    selfTuning: true,
  };
  const env = capabilityEnvFromConfig(deviating);
  assert.equal(env.tune_memory_fade, 0.5, 'memoryFade 0.5 必须映射到 tune_memory_fade');
  assert.equal(env.tune_curiosity_strength, 1.5, 'curiosityStrength 1.5 必须映射到 tune_curiosity_strength');
  assert.equal(env.tune_tone_saturation, 0.8, 'toneSaturation 0.8 必须映射到 tune_tone_saturation');
  assert.equal(env.tune_consolidation_cadence, 3, 'consolidationCadence 3 必须映射到 tune_consolidation_cadence');
  assert.equal(env.enable_self_tuning, true, 'selfTuning true 必须映射到 enable_self_tuning');

  for (const unset of [capabilityEnvFromConfig(undefined), capabilityEnvFromConfig(null)]) {
    for (const key of KNOB_KEYS) {
      assert.equal(unset[KNOBS[key].field], KNOBS[key].baseline, `未配 capabilities 时 ${KNOBS[key].field} 必须回基线`);
    }
    assert.equal(unset.enable_self_tuning, false, '未配 capabilities 时自学习必须 fail-closed（关）');
  }

  // 越界/非法钳制：超出 [min,max] 钳到边界，整合节奏取整。
  const clamped = capabilityEnvFromConfig({
    ...DEFAULT_CAPABILITY_TOGGLES,
    memoryFade: 99,
    curiosityStrength: 0.01,
    toneSaturation: 5,
    consolidationCadence: 999,
    selfTuning: true,
  });
  assert.equal(clamped.tune_memory_fade, 4.0, 'memoryFade 越界必须钳到 max 4.0');
  assert.equal(clamped.tune_curiosity_strength, 0.25, 'curiosityStrength 越界必须钳到 min 0.25');
  assert.equal(clamped.tune_tone_saturation, 2.0, 'toneSaturation 越界必须钳到 max 2.0');
  assert.equal(clamped.tune_consolidation_cadence, 10, 'consolidationCadence 越界必须钳到 max 10');
  console.log('  -> PASS: 偏离值直传；未配 = 基线 + 自学习关；越界钳到 [min,max]');
}

// ---------------------------------------------------------------------------
// 5. 预设：恰为 省心/均衡/深度记忆 三档（值在域内且在步进网格上）；
//    恢复基线 = 四基线 + 自学习关（源码正则抽取，.svelte 不可被 node import）
// ---------------------------------------------------------------------------
{
  const presetMatch = /DISPOSITION_PRESETS = (\{[\s\S]*?\}) as const/.exec(settingsSrc);
  assert.ok(presetMatch, 'SettingsView 必须导出 DISPOSITION_PRESETS');
  const presets = parseSourceObject(presetMatch[1]);
  assert.deepEqual(Object.keys(presets).sort(), ['均衡', '省心', '深度记忆'].sort(), '预设必须恰为 省心/均衡/深度记忆 三档');

  for (const [name, values] of Object.entries(presets)) {
    assert.deepEqual(
      Object.keys(values).sort(),
      [...KNOB_KEYS].sort(),
      `预设「${name}」必须恰好携带四个体验参数`,
    );
    for (const key of KNOB_KEYS) {
      const {min, max, step, baseline} = KNOBS[key];
      const v = values[key];
      assert.ok(Number.isFinite(v), `预设「${name}」.${key} 必须是有限数`);
      assert.ok(v >= min && v <= max, `预设「${name}」.${key}=${v} 必须在 [${min}, ${max}] 内`);
      assert.ok(
        Math.abs(v / step - Math.round(v / step)) < 1e-9,
        `预设「${name}」.${key}=${v} 必须落在步进 ${step} 网格上`,
      );
      assert.ok(typeof baseline === 'number', `${key} 基线表必须有数`);
    }
  }

  const baselineMatch = /DISPOSITION_BASELINE_RESET = (\{[\s\S]*?\}) as const/.exec(settingsSrc);
  assert.ok(baselineMatch, 'SettingsView 必须导出 DISPOSITION_BASELINE_RESET（恢复基线）');
  const reset = parseSourceObject(baselineMatch[1]);
  for (const key of KNOB_KEYS) {
    assert.equal(reset[key], KNOBS[key].baseline, `恢复基线必须把 ${key} 拨回基线 ${KNOBS[key].baseline}`);
  }
  assert.equal(reset.selfTuning, false, '恢复基线必须把「从使用中学习」关回默认关');
  console.log('  -> PASS: 预设恰为三档且值在域内/步进网格上；恢复基线 = 基线 + 自学习关');
}

// ---------------------------------------------------------------------------
// 6. 滑杆取值域：min/max/step 与参数表逐项一致（源码镜像）
// ---------------------------------------------------------------------------
{
  for (const key of KNOB_KEYS) {
    const {label, sliderAttrs} = KNOBS[key];
    const match = new RegExp(`<input([^>]*)aria-label="${label}"`).exec(settingsSrc);
    assert.ok(match, `SettingsView 必须有「${label}」滑杆（aria-label）`);
    for (const attr of sliderAttrs) {
      assert.ok(match[1].includes(attr), `「${label}」滑杆必须带 ${attr}（与参数表一致）`);
    }
  }
  console.log('  -> PASS: 四滑杆 min/max/step 与参数表逐项一致');
}

// ---------------------------------------------------------------------------
// 7. parseCapabilityToggles 往返：capabilities 整体白名单持久化（2026-09-22
//    教训）+ 脏值钳制/回基线 + selfTuning fail-closed
// ---------------------------------------------------------------------------
{
  const cfg = loadConfig();
  cfg.capabilities = {
    ...DEFAULT_CAPABILITY_TOGGLES,
    memoryFade: 1.5,
    curiosityStrength: 0.75,
    toneSaturation: 0.8,
    consolidationCadence: 2,
    selfTuning: true,
  };
  saveConfig(cfg);
  const raw = JSON.parse(store.get('apeireth-config'));
  assert.equal(raw.capabilities.memoryFade, 1.5, 'persistedConfig 白名单必须整体携带 capabilities（含新旋钮）');
  assert.equal(raw.capabilities.selfTuning, true);
  const back = loadConfig();
  assert.equal(back.capabilities.memoryFade, 1.5, 'memoryFade 必须活着读回');
  assert.equal(back.capabilities.curiosityStrength, 0.75, 'curiosityStrength 必须活着读回');
  assert.equal(back.capabilities.toneSaturation, 0.8, 'toneSaturation 必须活着读回');
  assert.equal(back.capabilities.consolidationCadence, 2, 'consolidationCadence 必须活着读回');
  assert.equal(back.capabilities.selfTuning, true, 'selfTuning 必须活着读回');

  // 脏值守卫：越界钳制、非整数整合节奏取整、非 true 的 selfTuning 一律关。
  raw.capabilities = {
    memoryFade: 99,
    curiosityStrength: 0,
    toneSaturation: -3,
    consolidationCadence: 3.7,
    selfTuning: 'yes',
  };
  store.set('apeireth-config', JSON.stringify(raw));
  const dirty = loadConfig();
  assert.equal(dirty.capabilities.memoryFade, 4.0, '脏 memoryFade 必须钳到 max 4.0');
  assert.equal(dirty.capabilities.curiosityStrength, 0.25, '脏 curiosityStrength 必须钳到 min 0.25');
  assert.equal(dirty.capabilities.toneSaturation, 0.0, '脏 toneSaturation 必须钳到 min 0');
  assert.equal(dirty.capabilities.consolidationCadence, 4, '脏 consolidationCadence 必须取整（round(3.7)=4）');
  assert.equal(dirty.capabilities.selfTuning, false, '非 true 的 selfTuning 必须 fail-closed（关）');

  // 缺失字段 = 全基线（未设 = 现行为，零变化）。
  raw.capabilities = {};
  store.set('apeireth-config', JSON.stringify(raw));
  const empty = loadConfig();
  for (const key of KNOB_KEYS) {
    assert.equal(empty.capabilities[key], KNOBS[key].baseline, `缺失 ${key} 必须回基线 ${KNOBS[key].baseline}`);
  }
  assert.equal(empty.capabilities.selfTuning, false, '缺失 selfTuning 必须回默认关');
  console.log('  -> PASS: capabilities 整体白名单往返 + 脏值钳制 + 缺失回基线（fail-closed）');
}

console.log('--- All Self-Tuning Mapping Checks PASSED! ---');
