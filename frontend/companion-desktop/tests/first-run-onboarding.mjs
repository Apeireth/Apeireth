// 首启引导（OOBE）单测 — import 真实实现 ../src/lib/onboarding.ts
//
// 四条验收边（与 FirstRunWizard.svelte 的分步弹窗配套）：
//   ① 分流步序：普通五步 / 专业三步，专业轨道无任何配置步（只做介绍）；
//   ② 普通用户预设：推荐配置六旋钮 + selfTuning（自动调参），危险能力绝不进预设；
//   ③ 自我描述词 → 用户印象：落点 = 当前伙伴 key（与 runtime 注入同口径）；
//   ④ 完成页配置：provider/openaiConfig 镜像、能力全量带上、顶层密钥不进配置。
//
// localStorage 内存 shim：与 config-persistence.mjs 同款，动态 import 保证
// shim 先于任何 loadConfig / 印象写入就位。
import assert from 'node:assert/strict';

const store = new Map();
globalThis.localStorage = {
  getItem: (k) => (store.has(k) ? store.get(k) : null),
  setItem: (k, v) => void store.set(k, String(v)),
  removeItem: (k) => void store.delete(k),
  clear: () => store.clear(),
};

const {
  CASUAL_STEPS,
  PRO_STEPS,
  stepsFor,
  casualCapabilities,
  impressionFromSelfDescription,
  writeSelfDescription,
  buildWizardConfig,
  INTRO_BULLETS,
  INTRO_PLAN,
  PRO_GUIDE_LINES,
} = await import('../src/lib/onboarding.ts');
const {loadUserImpression} = await import('../src/lib/user-impression.ts');

console.log('--- Starting First-Run Onboarding Check ---');

// ---------------------------------------------------------------------------
// ① 分流步序（依次提示）
// ---------------------------------------------------------------------------
{
  assert.deepEqual([...stepsFor('casual')], ['welcome', 'tier', 'api', 'identity', 'preset'], '普通五步依次提示');
  assert.deepEqual([...stepsFor('pro')], ['welcome', 'tier', 'pro-guide'], '专业三步：分流后只做介绍');
  assert.deepEqual([...stepsFor(null)], [...CASUAL_STEPS], '未分流按普通轨道预览');
  assert.ok(!PRO_STEPS.includes('api') && !PRO_STEPS.includes('identity') && !PRO_STEPS.includes('preset'),
    '专业轨道无任何配置步（只做介绍）');
  assert.ok(INTRO_BULLETS.length >= 3 && INTRO_PLAN.length === 4, '欢迎页介绍 + 四步预告');
  assert.ok(PRO_GUIDE_LINES.some((line) => line.includes('密钥')), '参考卡要指路密钥配置');
}

// ---------------------------------------------------------------------------
// ② 普通用户预设（推荐配置六旋钮 + 自动调参；危险项红线）
// ---------------------------------------------------------------------------
{
  const caps = casualCapabilities(true);
  for (const key of ['proactiveRecall', 'preferenceLearning', 'memoryInjection', 'consolidation', 'reflexion', 'organs']) {
    assert.equal(caps[key], true, `推荐配置必须开: ${key}`);
  }
  assert.equal(caps.selfTuning, true, '自动调参跟开关走');
  assert.equal(casualCapabilities(false).selfTuning, false, '关自动调参 = 预设固定');
  for (const danger of ['shell', 'fetch', 'fileWrite', 'fileWriteAutoPass']) {
    assert.equal(caps[danger], false, `危险能力绝不进预设: ${danger}`);
  }
  assert.equal(caps.maxTurnRounds, null, '预算族留空 = 后端默认（伸缩由自动调参承担）');
  assert.equal(caps.contextBudgetChars, null);
}

// ---------------------------------------------------------------------------
// ③ 自我描述词 → 用户印象（"他如何理解你"的初始档案）
// ---------------------------------------------------------------------------
{
  const entry = impressionFromSelfDescription('  我是学生，回答简洁。  ', 'apeireth-default', '阿佩瑞斯');
  assert.equal(entry.summary, '我是学生，回答简洁。', '去空白');
  assert.equal(entry.source, 'manual');
  assert.equal(entry.personaId, 'apeireth-default');

  // 落点 = 当前伙伴（缺省档案 = 出厂第一人设阿佩瑞斯，与 runtime 注入同 key）
  const target = writeSelfDescription('叫我小明。常用中文。');
  assert.equal(target.personaId, 'apeireth-default');
  assert.equal(target.personaName, '阿佩瑞斯');
  const loaded = loadUserImpression('apeireth-default', '阿佩瑞斯');
  assert.equal(loaded.summary, '叫我小明。常用中文。', '印象可读回（读侧永不抛）');
}

// ---------------------------------------------------------------------------
// ④ 完成页配置（App completeFirstRun 浅合并的输入面）
// ---------------------------------------------------------------------------
{
  const cfg = buildWizardConfig(
    {
      presetId: 'deepseek',
      baseUrl: 'https://api.deepseek.com/v1',
      model: 'deepseek-v4-flash',
      apiKey: 'sk-live-secret-probe',
    },
    true,
  );
  assert.equal(cfg.apiKey, '', '顶层密钥不进配置（key 只进钥匙串）');
  assert.equal(cfg.provider.preset, 'deepseek');
  assert.equal(cfg.provider.baseUrl, 'https://api.deepseek.com/v1');
  assert.equal(cfg.provider.model, 'deepseek-v4-flash');
  assert.equal(cfg.openaiConfig.model, 'deepseek-v4-flash', 'openaiConfig 与 provider 镜像');
  assert.equal(cfg.model, 'deepseek-v4-flash');
  assert.equal(cfg.capabilities.selfTuning, true, '能力面带上自动调参');
  assert.equal(cfg.capabilities.organs, true, '能力面带上推荐配置');
  assert.equal(cfg.capabilities.shell, false);
}

console.log('First-run onboarding check passed.');
