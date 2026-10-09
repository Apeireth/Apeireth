// 受控文件写入（apply_patch）能力开关映射一致性（主开关 + 依赖子开关）。
// import 真实 types.ts / capability-apply.ts / desktop-bridge.ts / runtime.ts，
// 并对 SettingsView 能力注册表与 Rust 侧 backend_supervisor.rs 的注入源做
// 源码级镜像校验。
//
// 契约：
//   1. fileWrite / fileWriteAutoPass 默认关（fail-closed）。
//   2. 拨动 fileWrite 开 → enable_file_write:true；连同 fileWriteAutoPass 开 →
//      enable_file_write_auto_pass:true。
//   3. 依赖语义：fileWrite 关时即使 fileWriteAutoPass 开也不注入子开关变量
//      （enable_file_write_auto_pass:false，绝不带起子开关）。
//   4. 未配 capabilities（undefined / null）两个注入字段都 false（fail-closed）。
//   5. env 名三处同名对齐：SettingsView env 芯片 ↔ capabilityEnvFromConfig
//      注入字段 ↔ backend_supervisor env_pairs（false 不注入 "0"）。
//   6. parseCapabilityToggles 容错解析：白名单往返照读；脏值一律关（fail-closed）。
//   7. SettingsView 注册表/嵌套子行接线：requires/capDisabled 依赖语义 +
//      即点即生效（toggleCap → handleCapabilityToggle → apply 路径）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const srcDir = join(testsDir, '..', 'src');

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
const {toggledCapability, configWithCapabilities} = await import('../src/lib/capability-apply.ts');
const {capabilityEnvFromConfig} = await import('../src/lib/desktop-bridge.ts');
const {loadConfig, saveConfig} = await import('../src/lib/runtime.ts');

// 主开关 / 子开关 ↔ 注入字段 ↔ env 名（键名到后端 env 的映射与 SettingsView
// 能力注册表逐键一致，此处镜像校验两侧）。
const MAIN = {key: 'fileWrite', field: 'enable_file_write', env: 'APEIRETH_ENABLE_FILE_WRITE'};
const SUB = {
  key: 'fileWriteAutoPass',
  field: 'enable_file_write_auto_pass',
  env: 'APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS',
  requires: 'fileWrite',
};

const settingsSrc = readFileSync(join(srcDir, 'lib/views/SettingsView.svelte'), 'utf8');

console.log('--- Starting Controlled File-Write Toggle Mapping Check ---');

// ---------------------------------------------------------------------------
// 1. 默认值：两件默认关（fail-closed），待推送配置不带起任何写文件能力
// ---------------------------------------------------------------------------
{
  assert.equal(DEFAULT_CAPABILITY_TOGGLES.fileWrite, false, 'fileWrite 默认必须关（fail-closed）');
  assert.equal(DEFAULT_CAPABILITY_TOGGLES.fileWriteAutoPass, false, 'fileWriteAutoPass 默认必须关（fail-closed）');
  const env = capabilityEnvFromConfig(DEFAULT_CAPABILITY_TOGGLES);
  assert.equal(env.enable_file_write, false, '默认不得注入受控文件写入');
  assert.equal(env.enable_file_write_auto_pass, false, '默认不得注入自动放行子开关');
  console.log('  -> PASS: fileWrite / fileWriteAutoPass 默认关，注入字段默认 false');
}

// ---------------------------------------------------------------------------
// 2. 拨动主开关 → 待推送配置 → 注入字段；连同子开关一起开才注入子开关变量
// ---------------------------------------------------------------------------
{
  const on = toggledCapability(DEFAULT_CAPABILITY_TOGGLES, 'fileWrite', true);
  assert.equal(on.fileWrite, true, '拨动后主开关必须为开');
  assert.equal(DEFAULT_CAPABILITY_TOGGLES.fileWrite, false, 'toggledCapability 不可变更新，原值不动');

  const cfg = configWithCapabilities({baseUrl: 'http://127.0.0.1:1', model: 'm'}, on);
  assert.equal(cfg.capabilities.fileWrite, true, '待推送配置必须携带开关后的值');
  assert.equal(cfg.baseUrl, 'http://127.0.0.1:1', '其余配置字段不被覆盖');

  const env = capabilityEnvFromConfig(cfg.capabilities);
  assert.equal(env.enable_file_write, true, '主开关开 → enable_file_write 必须为 true');
  assert.equal(env.enable_file_write, cfg.capabilities.fileWrite === true, '注入字段与开关同真同假');
  assert.equal(env.enable_file_write_auto_pass, false, '子开关未开不得注入');

  const both = capabilityEnvFromConfig(toggledCapability(on, 'fileWriteAutoPass', true));
  assert.equal(both.enable_file_write, true, '主开关保持开');
  assert.equal(both.enable_file_write_auto_pass, true, '主开关 + 子开关都开 → enable_file_write_auto_pass 必须为 true');

  const off = capabilityEnvFromConfig(toggledCapability(both, 'fileWrite', false));
  assert.equal(off.enable_file_write, false, '主开关关 → enable_file_write 必须为 false');
  assert.equal(off.enable_file_write_auto_pass, false, '主开关关 → 子开关注入必须跟着关（依赖语义）');
  console.log('  -> PASS: 拨动 fileWrite / fileWriteAutoPass → 注入字段映射正确（含依赖回落）');
}

// ---------------------------------------------------------------------------
// 3. 依赖语义：子开关单独开（主开关关）绝不注入子开关 env
// ---------------------------------------------------------------------------
{
  const subOnly = capabilityEnvFromConfig(
    toggledCapability(DEFAULT_CAPABILITY_TOGGLES, 'fileWriteAutoPass', true),
  );
  assert.equal(subOnly.enable_file_write, false, '主开关关 → enable_file_write 必须为 false');
  assert.equal(
    subOnly.enable_file_write_auto_pass,
    false,
    '子开关单独开（fileWrite 关）绝不注入——依赖语义 fail-closed',
  );

  // 字段存在性：两个注入字段都恒为布尔（映射面完整暴露）。
  const keys = Object.keys(capabilityEnvFromConfig(DEFAULT_CAPABILITY_TOGGLES));
  assert.ok(keys.includes(MAIN.field), `注入字段必须有 ${MAIN.field}`);
  assert.ok(keys.includes(SUB.field), `注入字段必须有 ${SUB.field}`);
  assert.equal(typeof subOnly[SUB.field], 'boolean', `${SUB.field} 必须是布尔`);
  console.log('  -> PASS: 子开关单独开不注入（依赖语义收口在映射层）');
}

// ---------------------------------------------------------------------------
// 4. 未配 capabilities（undefined / null）= 两件都 fail-closed
// ---------------------------------------------------------------------------
{
  for (const unset of [capabilityEnvFromConfig(undefined), capabilityEnvFromConfig(null)]) {
    assert.equal(unset.enable_file_write, false, '未配 capabilities 时 enable_file_write 必须 fail-closed');
    assert.equal(
      unset.enable_file_write_auto_pass,
      false,
      '未配 capabilities 时 enable_file_write_auto_pass 必须 fail-closed',
    );
  }
  console.log('  -> PASS: 未配 capabilities 两件都 fail-closed（不注入）');
}

// ---------------------------------------------------------------------------
// 5. env 名三处同名对齐：SettingsView 注册表 ↔ 注入字段 ↔ backend_supervisor env_pairs
// ---------------------------------------------------------------------------
{
  // SettingsView 能力注册表（主行 + 依赖子行）的 env 芯片。
  for (const {key, env} of [MAIN, SUB]) {
    const match = new RegExp(`key: '${key}'[^}]*env: '([A-Z_]+)'`).exec(settingsSrc);
    assert.ok(match, `SettingsView 能力注册表必须有 ${key} 行`);
    assert.equal(match[1], env, `${key} 的 env 芯片必须等于注入旋钮名 ${env}`);
  }

  // Rust 侧注入源（backend_supervisor.rs 的 capability env_pairs 实现体）：
  // 两个 env 字符串必须真的出现在 env_pairs 里（不是只在文档注释里提到），
  // 且文件写入族恰好这两个名（false 不注入 "0"，fail-closed）。
  const supervisorSrc = readFileSync(join(testsDir, '..', 'src-tauri/src/backend_supervisor.rs'), 'utf8');
  const envPairsIdx = supervisorSrc.lastIndexOf('fn env_pairs');
  assert.ok(envPairsIdx > 0, 'backend_supervisor 必须有 capability env_pairs 实现');
  const nextFn = supervisorSrc.indexOf('\n    pub fn ', envPairsIdx);
  const envPairsBody = supervisorSrc.slice(envPairsIdx, nextFn === -1 ? supervisorSrc.length : nextFn);
  const supervisorWrite = [...new Set(envPairsBody.match(/APEIRETH_ENABLE_FILE_WRITE[A-Z_]*/g) ?? [])].sort();
  assert.deepEqual(
    supervisorWrite,
    [MAIN.env, SUB.env].sort(),
    `backend_supervisor env_pairs 必须恰好注入这两个 ${MAIN.env} / ${SUB.env}`,
  );
  console.log('  -> PASS: env 芯片/注入字段/后端 env_pairs 三处同名对齐');
}

// ---------------------------------------------------------------------------
// 6. parseCapabilityToggles 往返：capabilities 整体白名单持久化 + 脏值关
// ---------------------------------------------------------------------------
{
  const cfg = loadConfig();
  cfg.capabilities = {...DEFAULT_CAPABILITY_TOGGLES, fileWrite: true, fileWriteAutoPass: true};
  saveConfig(cfg);
  const raw = JSON.parse(store.get('apeireth-config'));
  assert.equal(raw.capabilities.fileWrite, true, 'persistedConfig 白名单必须整体携带 capabilities（含新开关）');
  assert.equal(raw.capabilities.fileWriteAutoPass, true);
  const back = loadConfig();
  assert.equal(back.capabilities.fileWrite, true, 'fileWrite 必须活着读回');
  assert.equal(back.capabilities.fileWriteAutoPass, true, 'fileWriteAutoPass 必须活着读回');

  // 脏值守卫：非 true 一律关（fail-closed）。
  raw.capabilities = {fileWrite: 'yes', fileWriteAutoPass: 1};
  store.set('apeireth-config', JSON.stringify(raw));
  const dirty = loadConfig();
  assert.equal(dirty.capabilities.fileWrite, false, '非 true 的 fileWrite 必须 fail-closed（关）');
  assert.equal(dirty.capabilities.fileWriteAutoPass, false, '非 true 的 fileWriteAutoPass 必须 fail-closed（关）');

  // 缺失字段 = 默认关（未设 = 不开任何写文件能力）。
  raw.capabilities = {};
  store.set('apeireth-config', JSON.stringify(raw));
  const empty = loadConfig();
  assert.equal(empty.capabilities.fileWrite, false, '缺失 fileWrite 必须回默认关');
  assert.equal(empty.capabilities.fileWriteAutoPass, false, '缺失 fileWriteAutoPass 必须回默认关');
  console.log('  -> PASS: capabilities 白名单往返 + 脏值关 + 缺失回默认关（fail-closed）');
}

// ---------------------------------------------------------------------------
// 7. SettingsView 注册表/嵌套子行接线（requires/capDisabled + 即点即生效）
// ---------------------------------------------------------------------------
{
  // 主行进 TOOL_DEFS 注册表（图标/env 芯片/描述同周边行机制，计数随注册表走）。
  assert.ok(
    /key: 'fileWrite'[^}]*env: 'APEIRETH_ENABLE_FILE_WRITE'/.test(settingsSrc),
    '文件写入主行必须在能力注册表（env 芯片 APEIRETH_ENABLE_FILE_WRITE）',
  );
  assert.ok(settingsSrc.includes('文件写入（apply_patch · shell 写命令）'), '主行标题文案在案（受闸两面都进标题：apply_patch 与 shell 写命令）');
  for (const phrase of ['创建/修改/删除须在补丁里声明', '工作区外路径与凭据/密钥面拒绝', 'git 提交等写操作不提供工具（设计边界）']) {
    assert.ok(settingsSrc.includes(phrase), `主行警告/帮助行必须写明「${phrase}」`);
  }

  // 子行进 TOOL_SUB_DEFS 注册表：依赖 fileWrite（requires）+ 嵌套子行渲染。
  assert.ok(
    /key: 'fileWriteAutoPass'[\s\S]{0,400}requires: 'fileWrite'/.test(settingsSrc),
    '「自动放行已读文件修改」子行必须声明依赖 fileWrite（requires）',
  );
  assert.ok(
    /key: 'fileWriteAutoPass'[^}]*env: 'APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS'/.test(settingsSrc),
    '子行 env 芯片必须是 APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS',
  );
  for (const phrase of ['仅修改类补丁免审批', '删除/新建永不自动放行（仍走人工审批）']) {
    assert.ok(settingsSrc.includes(phrase), `子行帮助行必须写明「${phrase}」`);
  }
  assert.ok(settingsSrc.includes('自动放行已读文件修改'), '子行标题文案在案');

  // 嵌套子行接线：同沙箱嵌套行的 cap-row-nested 视觉；依赖关闭时禁用（capDisabled）。
  assert.ok(settingsSrc.includes('class="cap-row cap-row-nested"'), '子行必须嵌套子行样式');
  assert.ok(
    settingsSrc.includes('onclick={() => toggleCap(subDef)}'),
    '子行必须拨动即生效（注册表 toggleCap）',
  );
  assert.ok(
    settingsSrc.includes('disabled={capDisabled(subDef) || liveApplyPendingKey !== null}'),
    'fileWrite 关闭时子行必须禁用（capDisabled 依赖语义）',
  );
  assert.ok(
    /function toggleCap\(def: CapDef\)[\s\S]*?handleCapabilityToggle\(/.test(settingsSrc),
    'toggleCap 必须汇到 handleCapabilityToggle（即点即生效 apply 缝）',
  );
  console.log('  -> PASS: 注册表/嵌套子行接线在案（requires 禁用 + 即点即生效）');
}

console.log('--- All Controlled File-Write Toggle Mapping Checks PASSED! ---');
