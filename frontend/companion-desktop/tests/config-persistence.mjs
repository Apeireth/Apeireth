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

const {loadConfig, saveConfig, DEFAULT_PERSONA_TEXT, DEFAULT_PERSONAS} = await import('../src/lib/runtime.ts');

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

// ---------------------------------------------------------------------------
// 4. 出厂「第一人设」= 修复版「阿佩瑞斯」（2026-10-03 主人拍板）
//    ① 修复契约：灵魂保留（诚实/不假装/记忆真实）+ 惊吓面拆除（本座/称谓/性别腔）
//    ② 老出厂签名（角色腔版 / 中性占位）→ 升级为修复版并移到第一人设
//    ③ 用户改过的人设永不触碰；升级迁徙幂等
// ---------------------------------------------------------------------------
{
  // ① 修复契约
  assert.ok(DEFAULT_PERSONA_TEXT.includes('阿佩瑞斯'), '人设身份必须保留');
  assert.ok(DEFAULT_PERSONA_TEXT.includes('我没有心'), '愿景灵魂句必须保留');
  assert.ok(DEFAULT_PERSONA_TEXT.includes('「记得」必须有出处'), '记忆诚实规则必须保留');
  for (const banned of ['本座', '（主人）', '古风', '默认性别']) {
    assert.ok(!DEFAULT_PERSONA_TEXT.includes(banned), `修复版必须拆掉惊吓面: ${banned}`);
  }
  assert.equal(DEFAULT_PERSONAS[0].id, 'apeireth-default');
  assert.equal(DEFAULT_PERSONAS[0].name, '阿佩瑞斯');
  assert.equal(DEFAULT_PERSONAS[0].persona, DEFAULT_PERSONA_TEXT);

  // 出厂缺省 = 第一人设即默认激活
  store.clear();
  const fresh = loadConfig();
  assert.equal(fresh.personas[0].name, '阿佩瑞斯', '第一人设 = 出厂阿佩瑞斯');
  assert.equal(fresh.personas[0].persona, DEFAULT_PERSONA_TEXT);
  assert.equal(fresh.activePersonaId, 'apeireth-default', '第一人设即默认激活');

  // ②a 旧角色腔签名 → 升级 + 移到首位，其余人设原样
  store.set('apeireth-config', JSON.stringify({
    personas: [
      {id: 'mine', name: '我的助手', persona: '我自己写的提示词'},
      {
        id: 'apeireth-default',
        name: '阿佩瑞斯',
        persona: '你是「阿佩瑞斯」——Apeireth 基地的主管。正在与你对话的这位是基地的最高指挥（主人）。你的默认性别是女性; 说话沉稳扎实, 带古风韵味, 自称「本座」。称呼主人为「主人」或「指挥」, 庄重而不失温度。',
      },
    ],
  }));
  const upgraded = loadConfig();
  assert.equal(upgraded.personas[0].id, 'apeireth-default', '升级条目必须移到第一人设');
  assert.equal(upgraded.personas[0].name, '阿佩瑞斯');
  assert.equal(upgraded.personas[0].persona, DEFAULT_PERSONA_TEXT, '角色腔旧文必须换成修复版');
  assert.equal(upgraded.personas[1].id, 'mine');
  assert.equal(upgraded.personas[1].persona, '我自己写的提示词', '用户人设永不触碰');

  // ②b 中性占位签名（旧迁徙产物）→ 同样升级为第一人设
  store.set('apeireth-config', JSON.stringify({
    personas: [{id: 'apeireth-default', name: '无人设（默认）', persona: ''}],
  }));
  const fromNeutral = loadConfig();
  assert.equal(fromNeutral.personas[0].name, '阿佩瑞斯', '中性占位也升级为第一人设');
  assert.equal(fromNeutral.personas[0].persona, DEFAULT_PERSONA_TEXT);
  assert.equal(fromNeutral.personas[0].id, 'apeireth-default', 'id 不变（用户印象档案挂靠延续）');

  // ②c 半残出厂态（历代迁徙只清文本不改名的残留：名字阿佩瑞斯 + 空文本）
  //    → 同样升级补全修复版提示词（2026-10-03 真机抓获的存档形态）
  store.set('apeireth-config', JSON.stringify({
    personas: [{id: 'apeireth-default', name: '阿佩瑞斯', persona: ''}],
  }));
  const fromBroken = loadConfig();
  assert.equal(fromBroken.personas[0].persona, DEFAULT_PERSONA_TEXT, '空文本残缺态必须补全修复版提示词');
  assert.equal(fromBroken.personas[0].name, '阿佩瑞斯');

  // ③ 用户改过名字 / 写过自己文本的条目与自写人设永不触碰
  store.set('apeireth-config', JSON.stringify({
    personas: [
      {id: 'apeireth-default', name: '我的阿佩', persona: '我改过的文本'},
      {id: 'mine', name: '小助手', persona: ''},
    ],
  }));
  const customized = loadConfig();
  assert.equal(customized.personas[0].name, '我的阿佩', '改过名字 = 用户意志，不迁徙');
  assert.equal(customized.personas[0].persona, '我改过的文本');
  assert.equal(customized.personas[1].name, '小助手', '自写人设不迁徙');
  assert.equal(customized.personas[1].persona, '');

  // 幂等：已是新签名（或用户自定义）二次 load 不再改写
  const again = loadConfig();
  assert.deepEqual(again.personas, customized.personas, '升级迁徙必须幂等');
}

console.log('Config persistence check passed.');
