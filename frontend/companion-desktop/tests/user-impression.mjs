// 用户印象契约（lib/user-impression.ts 真模块）——AI 智能体独立设置栏
// 「用户印象」的两条验收边：
//   ① 用户阅读项：按伙伴分档存取往返 / 归一守门（读侧不炸）/ 订阅广播 /
//      写侧如实抛（不静默丢改动）/ 清空只清正文；
//   ② AI 自我参考项：印象非空才进 system 注入块（空印象不注水），文案自报
//      「用户可读可改」的可信边界；
//   ③ 总结缝：提示词带旧印象+对话、回复解析剥壳钳长、合并落盘、空回复如实抛；
//   ④ 自动总结节流三条判定：开关关=恒不追 / 无印象看门槛 / 有印象看增量；
//   ⑤ 接线镜像：runtime run() 注入+后台总结、设置页分区上导航与渲染分支。
// localStorage 内存 shim 先于动态 import 就位（与 user-profile.mjs 同纪律）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const store = new Map();
globalThis.localStorage = {
  getItem: (k) => (store.has(k) ? store.get(k) : null),
  setItem: (k, v) => void store.set(k, String(v)),
  removeItem: (k) => void store.delete(k),
  clear: () => store.clear(),
};

const {
  AUTO_REFRESH_USER_MESSAGES,
  IMPRESSION_MAX_CHARS,
  MIN_USER_MESSAGES_FOR_FIRST_IMPRESSION,
  USER_IMPRESSION_STORAGE_KEY,
  buildImpressionPrompt,
  clearUserImpression,
  emptyUserImpression,
  impressionSystemBlock,
  loadUserImpression,
  normalizeUserImpression,
  parseImpressionReply,
  saveUserImpression,
  shouldRefreshImpression,
  subscribeUserImpression,
  summarizeUserImpression,
  transcriptForSummary,
  transcriptFromConversations,
} = await import('../src/lib/user-impression.ts');

console.log('--- Starting user impression (agent settings column) check ---');

// ① 空印象缺省 + 按伙伴分档隔离（每个伙伴一份「它眼中的你」）
{
  const empty = loadUserImpression('p1', '阿佩');
  assert.deepEqual(empty, {...emptyUserImpression('p1'), personaName: '阿佩'}, '空账本回落空印象');
  assert.equal(empty.autoRefresh, true, '自动总结缺省开');
  empty.summary = 'mutated';
  assert.equal(loadUserImpression('p1').summary, '', '读回是副本，改动不污染');

  saveUserImpression({...emptyUserImpression('p1'), personaName: '阿佩', summary: '喜欢简洁回答'});
  saveUserImpression({...emptyUserImpression('p2'), personaName: '小助手', summary: '爱问底层原理'});
  assert.equal(loadUserImpression('p1').summary, '喜欢简洁回答', 'p1 档活着读回');
  assert.equal(loadUserImpression('p2').summary, '爱问底层原理', 'p2 档活着读回');
  assert.ok(store.has(USER_IMPRESSION_STORAGE_KEY), '落盘到本地单键');
}

// ② 归一守门：脏 JSON / 超长正文 / 非法枚举 / 负计数一律收敛，读侧不炸
{
  store.set(USER_IMPRESSION_STORAGE_KEY, '{not json');
  assert.equal(loadUserImpression('p1').summary, '', '脏 JSON 回落空印象');

  const dirty = normalizeUserImpression(
    {
      personaName: 42,
      summary: 'x'.repeat(IMPRESSION_MAX_CHARS + 100),
      updatedAt: -5,
      seenUserCount: Number.NaN,
      source: 'telepathy',
      autoRefresh: 'yes',
    },
    'p1',
  );
  assert.equal(dirty.summary.length, IMPRESSION_MAX_CHARS, '正文截断到上限');
  assert.equal(dirty.updatedAt, 0, '负时间归零');
  assert.equal(dirty.seenUserCount, 0, '脏计数归零');
  assert.equal(dirty.source, 'manual', '非法来源归 manual');
  assert.equal(dirty.autoRefresh, true, '脏开关归默认开');
  assert.equal(normalizeUserImpression({autoRefresh: false}, 'p1').autoRefresh, false, '显式关保留');
  assert.equal(normalizeUserImpression(null, 'p1').personaId, 'p1', '非对象归一不炸');
}

// ③ 订阅广播 + 退订即停（设置栏实时跟随）
{
  const seen = [];
  const unsubscribe = subscribeUserImpression((e) => seen.push(e.summary));
  saveUserImpression({...emptyUserImpression('p1'), summary: '一响'});
  saveUserImpression({...emptyUserImpression('p1'), summary: '再响'});
  unsubscribe();
  saveUserImpression({...emptyUserImpression('p1'), summary: '不响'});
  assert.deepEqual(seen, ['一响', '再响'], '保存广播 + 退订即停');
}

// ④ 清空只清正文与游标，账本键与自动总结开关保留
{
  const entry = saveUserImpression({
    ...emptyUserImpression('p3'),
    summary: '要清掉的旧印象',
    seenUserCount: 9,
    autoRefresh: false,
  });
  const cleared = clearUserImpression(entry);
  assert.equal(cleared.summary, '', '正文清空');
  assert.equal(cleared.seenUserCount, 0, '节流游标重置');
  assert.equal(cleared.autoRefresh, false, '自动总结开关保留');
  assert.equal(loadUserImpression('p3').autoRefresh, false, '清空后账本键仍在');
}

// ⑤ 写侧如实抛：localStorage 不可用/满不让改动静默丢失
{
  globalThis.localStorage = {
    getItem: () => null,
    setItem: () => {
      throw new Error('quota');
    },
    removeItem: () => {},
    clear: () => {},
  };
  assert.throws(
    () => saveUserImpression({...emptyUserImpression('p1'), summary: '存不下'}),
    /quota/,
    '写失败如实抛（UI 显示可读理由）',
  );
  globalThis.localStorage = {
    getItem: (k) => (store.has(k) ? store.get(k) : null),
    setItem: (k, v) => void store.set(k, String(v)),
    removeItem: (k) => void store.delete(k),
    clear: () => store.clear(),
  };
}

// ⑥ AI 自我参考注入块：空印象不注水；有印象自报「用户可读可改」边界
{
  assert.equal(impressionSystemBlock(loadUserImpression('none')), '', '空印象不注水');
  assert.equal(impressionSystemBlock(null), '', 'null 印象不注水');
  const block = impressionSystemBlock({
    ...emptyUserImpression('p1'),
    personaName: '阿佩',
    summary: '喜欢简洁回答',
  });
  assert.ok(block.includes('用户印象 · 自我参考'), '注入块自报名目');
  assert.ok(block.includes('阿佩'), '带伙伴自称');
  assert.ok(block.includes('喜欢简洁回答'), '印象正文进块');
  assert.ok(block.includes('可读可改'), '自报可信边界（用户可读可改）');
}

// ⑦ 总结提示词：旧印象 + 对话都在场；无旧印象自报第一次
{
  const prompt = buildImpressionPrompt({
    personaName: '阿佩',
    previousSummary: '喜欢简洁回答',
    transcript: '用户：讲讲 Rust 的所有权\n伙伴：好的，先从借用说起……',
  });
  assert.ok(prompt.includes('阿佩'), '提示词带伙伴自称');
  assert.ok(prompt.includes('喜欢简洁回答'), '提示词带旧印象（合并而非推倒）');
  assert.ok(prompt.includes('讲讲 Rust 的所有权'), '提示词带对话转写');
  assert.ok(prompt.includes('200 字'), '长度约束在场');
  const first = buildImpressionPrompt({personaName: '', previousSummary: '  ', transcript: 'x'});
  assert.ok(first.includes('（还没有印象，这是第一次）'), '首次总结自报无旧印象');
}

// ⑧ 回复解析：剥代码围栏/引号壳、钳长、非字符串归空（如实）
{
  assert.equal(parseImpressionReply('```\n印象正文\n```'), '印象正文', '剥代码围栏');
  assert.equal(parseImpressionReply('「印象正文」'), '印象正文', '剥引号壳');
  assert.equal(parseImpressionReply('  多余空白  '), '多余空白', '去空白');
  assert.equal(parseImpressionReply('x'.repeat(IMPRESSION_MAX_CHARS + 50)).length, IMPRESSION_MAX_CHARS, '钳长');
  assert.equal(parseImpressionReply(null), '', '非字符串归空');
  assert.equal(parseImpressionReply('   '), '', '空回复归空');
}

// ⑨ 转写构建：只留 user/assistant、尾部截断、新对话优先留下
{
  const transcript = transcriptForSummary([
    {role: 'system', text: '系统提示不进转写'},
    {role: 'user', text: '第一条'},
    {role: 'assistant', text: '回复一'},
  ]);
  assert.ok(transcript.includes('第一条'), '用户消息在场');
  assert.ok(!transcript.includes('系统提示不进转写'), 'system 不进转写');
  assert.ok(transcript.startsWith('用户：'), '角色标签可读');

  const tight = transcriptForSummary(
    [
      {role: 'user', text: 'a'.repeat(50)},
      {role: 'user', text: '新消息最重要'},
    ],
    60,
  );
  assert.ok(tight.includes('新消息最重要'), '尾部（新消息）必留');
  assert.ok(!tight.includes('a'.repeat(50)), '头部旧内容被截断');

  const fromConvs = transcriptFromConversations([
    {updatedAt: 2, messages: [{role: 'user', text: '新对话内容'}]},
    {updatedAt: 1, messages: [{role: 'user', text: '旧对话内容'}]},
  ]);
  assert.ok(fromConvs.includes('新对话内容') && fromConvs.includes('旧对话内容'), '两段会话都铺开');
  assert.ok(
    fromConvs.indexOf('旧对话内容') < fromConvs.indexOf('新对话内容'),
    '旧会话在前、新会话在尾（尾部截断保新）',
  );
}

// ⑩ 自动总结节流三条判定
{
  assert.equal(shouldRefreshImpression(null, MIN_USER_MESSAGES_FOR_FIRST_IMPRESSION), true, '无印象达门槛即追');
  assert.equal(shouldRefreshImpression(null, MIN_USER_MESSAGES_FOR_FIRST_IMPRESSION - 1), false, '无印象未达门槛不追');
  const summarized = {...emptyUserImpression('p1'), summary: '旧印象', seenUserCount: 10};
  assert.equal(shouldRefreshImpression(summarized, 10 + AUTO_REFRESH_USER_MESSAGES - 1), false, '增量不足不追');
  assert.equal(shouldRefreshImpression(summarized, 10 + AUTO_REFRESH_USER_MESSAGES), true, '增量到点即追');
  assert.equal(
    shouldRefreshImpression({...summarized, autoRefresh: false}, 100),
    false,
    '自动总结关了恒不追（手动/手写不受影响）',
  );
}

// ⑪ 总结动作：补全缝注入 → 合并落盘 → 游标推进；空回复如实抛
{
  const prompts = [];
  const saved = await summarizeUserImpression({
    personaId: 'p9',
    personaName: '阿佩',
    transcript: '用户：我讨厌冗长的解释',
    complete: async (prompt) => {
      prompts.push(prompt);
      return '```\n用户偏好简短直接的解释。\n```';
    },
    source: 'auto',
    seenUserCount: 7,
  });
  assert.equal(saved.summary, '用户偏好简短直接的解释。', '解析后落盘');
  assert.equal(saved.source, 'auto', '来源如实记');
  assert.equal(saved.seenUserCount, 7, '节流游标推进');
  assert.ok(saved.updatedAt > 0, '盖 updatedAt');
  assert.equal(loadUserImpression('p9').summary, saved.summary, '账本读回一致');
  assert.ok(prompts[0].includes('我讨厌冗长的解释'), '补全缝收到完整提示词');

  await assert.rejects(
    () =>
      summarizeUserImpression({
        personaId: 'p9',
        transcript: 'x',
        complete: async () => '   ',
      }),
    /没有返回印象内容/,
    '模型空回复如实抛，旧印象保持不变',
  );
  assert.equal(loadUserImpression('p9').summary, saved.summary, '失败后旧印象未被动过');
}

// ⑫ 接线镜像：runtime 注入 + 后台总结；设置页分区上导航与渲染分支
{
  const runtimeSrc = readFileSync('src/lib/runtime.ts', 'utf8');
  assert.ok(runtimeSrc.includes('impressionSystemBlock('), 'run() 注入用户印象自我参考块');
  assert.ok(runtimeSrc.includes('maybeAutoSummarizeImpression('), '对话后自动总结已接线');
  assert.ok(runtimeSrc.includes("from './user-impression.ts'"), 'runtime 依赖用户印象模块（单向）');

  const navSrc = readFileSync('src/lib/settings-nav-groups.ts', 'utf8');
  assert.ok(navSrc.includes("'impression'"), '用户印象进分区清单');

  const viewSrc = readFileSync('src/lib/views/SettingsView.svelte', 'utf8');
  assert.ok(viewSrc.includes("label: '用户印象'"), '用户印象分区上导航');
  assert.ok(viewSrc.includes("activeSection === 'impression'"), '分区渲染分支存在');
  assert.ok(viewSrc.includes("from '../user-impression'"), '印象走本地用户印象模块');
  assert.ok(viewSrc.includes('summarizeUserImpression('), '手动总结缝在场');
  assert.ok(viewSrc.includes('transcriptFromConversations('), '总结取数走会话账本转写');
  assert.ok(viewSrc.includes('clearUserImpression('), '清空动作在场');
  assert.ok(viewSrc.includes('自动总结'), '自动总结开关上屏');
  const impressionBlock = viewSrc.slice(viewSrc.indexOf("activeSection === 'impression'"));
  assert.ok(!impressionBlock.slice(0, 3000).includes('handleSaveSettings'), '印象分区不走批量保存缝');
}

console.log('--- All user impression checks passed ---');
