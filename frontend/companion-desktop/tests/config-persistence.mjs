// config 持久化往返单测（规范 §8 增补④⑤）— import 真实实现 ../src/lib/runtime.ts
//
// 2026-09-22 真实教训：persistedConfig 是显式键白名单，accent/customBg 曾漏列，
// 设置面板保存后被静默丢弃、reload 即失忆——B3 截图自查的 accent/custom-bg 探针
// 当场抓获。此套件锁死往返：个性化字段必须活下来，密钥字段必须继续被清洗。
//
// localStorage 内存 shim：runtime.ts 顶层不触 DOM（仅函数内引用），
// 动态 import 保证 shim 先于任何 loadConfig/saveConfig 调用就位。
import assert from 'node:assert/strict';

const store = new Map();
globalThis.localStorage = {
  getItem: (k) => (store.has(k) ? store.get(k) : null),
  setItem: (k, v) => void store.set(k, String(v)),
  removeItem: (k) => void store.delete(k),
  clear: () => store.clear(),
};

const {loadConfig, saveConfig} = await import('../src/lib/runtime.ts');

console.log('--- Starting Config Persistence Check ---');

// ---------------------------------------------------------------------------
// 1. 个性化字段往返：theme/accent/customBg 保存后必须活着读回
// ---------------------------------------------------------------------------
{
  const cfg = loadConfig(); // 默认配置（store 为空）
  cfg.theme = 'essence';
  cfg.accent = 'deep-space';
  cfg.customBg = true;
  saveConfig(cfg);

  const raw = JSON.parse(store.get('apeireth-config'));
  assert.equal(raw.accent, 'deep-space', 'persistedConfig 必须携带 accent（白名单纪律）');
  assert.equal(raw.customBg, true, 'persistedConfig 必须携带 customBg');
  assert.equal(raw.theme, 'essence');

  const back = loadConfig();
  assert.equal(back.accent, 'deep-space', 'accent 必须活着读回');
  assert.equal(back.customBg, true, 'customBg 必须活着读回');
  assert.equal(back.theme, 'essence');
}

// ---------------------------------------------------------------------------
// 2. 密钥清洗不回退：provider.apiKey 绝不落盘（安全不变量）
// ---------------------------------------------------------------------------
{
  const cfg = loadConfig();
  cfg.provider = {...cfg.provider, apiKey: 'sk-live-secret-probe'};
  cfg.apiKey = 'sk-gateway-transient';
  saveConfig(cfg);

  const rawText = store.get('apeireth-config');
  assert.ok(!rawText.includes('sk-live-secret-probe'), 'provider.apiKey 不得落盘');
  assert.ok(!rawText.includes('sk-gateway-transient'), '顶层 apiKey 不得落盘');
  const back = loadConfig();
  assert.equal(back.provider.apiKey, '', '读回时 provider.apiKey 必须为空串');
  assert.equal(back.apiKey, '', '读回时顶层 apiKey 必须为空串');
}

// ---------------------------------------------------------------------------
// 3. 脏数据守卫：非布尔 customBg / 非字符串 accent 一律按缺省处理
// ---------------------------------------------------------------------------
{
  const raw = JSON.parse(store.get('apeireth-config'));
  raw.customBg = 'yes'; // 脏值
  raw.accent = 42; // 脏值
  store.set('apeireth-config', JSON.stringify(raw));

  const back = loadConfig();
  assert.equal(back.customBg, undefined, '非 true 的 customBg 必须按缺省（关）处理');
  assert.equal(back.accent, undefined, '非字符串 accent 必须丢弃（resolveAccent 再兜底回落）');
}

console.log('Config persistence check passed.');
