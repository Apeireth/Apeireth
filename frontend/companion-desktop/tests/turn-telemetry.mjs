// 对话回合遥测（真实模块 src/lib/chat-shell/turn-telemetry.ts）。
// 契约：
//   1. 有数据：formatTurnTelemetry 对完整 usage（含命中数）产出四项真值串，
//      hover 详情含本轮总 token / 命中数 / 命中率 / 省下口径 / 耗时 / 模型。
//   2. 无数据「—」：usage 空/缺字段时对应位一律「—」诚实占位，绝不编数
//      （连 0 都不虚构——流里真报 0 才显示 0）。
//   3. usage 接线：parseUsageChunk/mergeUsage 对代表性流块解析正确
//      （末块总量 / prompt_cache_hit_tokens / prompt_tokens_details.cached_tokens /
//      只有 input_tokens/output_tokens / cache_read_input_tokens / message 事件体），
//      后到非空者胜。
//   4. source-mirror：App.svelte 状态行确实渲染遥测（title hover + 「—」占位），
//      runtime.ts 三条流式路径确实把 usage 接到 onUsage。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  DASH,
  formatDuration,
  formatTurnTelemetry,
  mergeUsage,
  parseUsageChunk,
} from '../src/lib/chat-shell/turn-telemetry.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const appSrc = readFileSync(join(testsDir, '..', 'src', 'App.svelte'), 'utf8');
const runtimeSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'runtime.ts'), 'utf8');

console.log('--- chat turn telemetry ---');

// ---------------------------------------------------------------------------
// 1. 遥测条渲染有数据：四项真值串 + hover 含总 token / 命中数
// ---------------------------------------------------------------------------
{
  const view = formatTurnTelemetry({
    usage: {promptTokens: 1200, completionTokens: 80, totalTokens: 1280, cacheHitTokens: 800},
    durationMs: 3200,
    model: 'm-x',
  });
  assert.equal(view.tokens, '入 1,200 / 出 80', '本轮 token 位 = 输入/输出真值串');
  assert.equal(view.cache, '命中 800 · 67%', '提示缓存命中位 = 命中数 + 命中率（800/1200）');
  assert.equal(view.duration, '3.2s', '回合耗时位 = 真实测量值');
  assert.equal(view.model, 'm-x', '模型位 = 回包模型名');
  for (const [key, value] of Object.entries(view)) {
    assert.ok(value && value !== DASH, `有数据时 ${key} 不得是占位（实际=${value}）`);
  }
  assert.ok(view.hover.includes('本轮总 token 1,280'), 'hover 含本轮总 token');
  assert.ok(view.hover.includes('命中 800 token'), 'hover 含命中 token 数');
  assert.ok(view.hover.includes('命中率 67%'), 'hover 含命中率');
  assert.ok(
    view.hover.includes('即省下 800 个输入 token 的重复处理'),
    'hover 说清命中 = 省下重复处理的输入 token',
  );
  assert.ok(view.hover.includes('回合耗时 3.2s'), 'hover 含回合耗时');
  assert.ok(view.hover.includes('模型 m-x'), 'hover 含模型');
  console.log('  -> PASS: 有数据 → 四项真值串 + hover 详情齐全');
}

// ---------------------------------------------------------------------------
// 2. 无数据「—」：usage 空/缺字段 → 对应位一律「—」，不编数
// ---------------------------------------------------------------------------
{
  const empty = formatTurnTelemetry({});
  assert.equal(empty.tokens, DASH, 'usage 空 → token 位「—」');
  assert.equal(empty.cache, DASH, 'usage 空 → 命中位「—」');
  assert.equal(empty.duration, DASH, '耗时缺 → 「—」');
  assert.equal(empty.model, DASH, '模型缺 → 「—」');
  assert.ok(empty.hover.includes('本轮总 token —'), 'hover 总 token 缺 → 「—」');
  assert.ok(empty.hover.includes(`提示缓存命中 ${DASH}`), 'hover 命中缺 → 「—」');

  const nullUsage = formatTurnTelemetry({usage: null, durationMs: null, model: null});
  assert.deepEqual(
    [nullUsage.tokens, nullUsage.cache, nullUsage.duration, nullUsage.model],
    [DASH, DASH, DASH, DASH],
    '显式 null 与缺省同口径',
  );

  // 缺字段逐位占位：只有输入 → 输出位「—」；命中缺 → 命中位「—」。
  const partial = formatTurnTelemetry({usage: {promptTokens: 500}, durationMs: 900});
  assert.equal(partial.tokens, '入 500 / 出 —', '输出缺 → 输出位「—」');
  assert.equal(partial.cache, DASH, '命中缺 → 命中位「—」');

  // 命中有、输入缺/为 0 → 率位「—」（不做除法也不编 0%）。
  const noInput = formatTurnTelemetry({usage: {cacheHitTokens: 300}});
  assert.equal(noInput.cache, `命中 300 · ${DASH}`, '输入缺 → 命中率位「—」');
  const zeroInput = formatTurnTelemetry({usage: {promptTokens: 0, cacheHitTokens: 0}, durationMs: 0});
  assert.equal(zeroInput.cache, `命中 0 · ${DASH}`, '输入为 0 → 命中率位「—」');
  assert.equal(zeroInput.tokens, '入 0 / 出 —', '流里真报 0 显示 0（不虚构成「—」也不虚构成别的数）');
  assert.equal(zeroInput.duration, '0ms', '真实 0 耗时显示 0ms');
  console.log('  -> PASS: 无数据一律「—」；真实 0 照显示 0；率位缺输入即「—」');
}

// ---------------------------------------------------------------------------
// 3. usage 接线：代表性流块解析 + 后到非空者胜
// ---------------------------------------------------------------------------
{
  // 末块总量块（本地网关：usage 在流末块顶层）。
  const tail = parseUsageChunk({
    choices: [{delta: {content: 'x'}}],
    usage: {prompt_tokens: 1200, completion_tokens: 80, total_tokens: 1280},
    model: 'm-2026-01',
  });
  assert.deepEqual(tail, {promptTokens: 1200, completionTokens: 80, totalTokens: 1280, model: 'm-2026-01'});

  // prompt_cache_hit_tokens 块。
  const hit = parseUsageChunk({
    usage: {prompt_tokens: 1000, completion_tokens: 50, total_tokens: 1050, prompt_cache_hit_tokens: 800},
  });
  assert.equal(hit.cacheHitTokens, 800, 'prompt_cache_hit_tokens → 命中数');
  assert.equal(hit.promptTokens, 1000);

  // prompt_tokens_details.cached_tokens 块。
  const details = parseUsageChunk({
    usage: {prompt_tokens: 1000, completion_tokens: 40, prompt_tokens_details: {cached_tokens: 700}},
  });
  assert.equal(details.cacheHitTokens, 700, 'prompt_tokens_details.cached_tokens → 命中数');

  // cache_read_input_tokens 块。
  const cacheRead = parseUsageChunk({
    usage: {input_tokens: 500, output_tokens: 5, cache_read_input_tokens: 400},
  });
  assert.equal(cacheRead.cacheHitTokens, 400, 'cache_read_input_tokens → 命中数');

  // 只有 input_tokens / output_tokens 的块（无总量、无命中 → 不编）。
  const ioOnly = parseUsageChunk({type: 'message_delta', usage: {input_tokens: 30, output_tokens: 12}});
  assert.equal(ioOnly.promptTokens, 30, 'input_tokens → 输入 token');
  assert.equal(ioOnly.completionTokens, 12, 'output_tokens → 输出 token');
  assert.equal(ioOnly.totalTokens, undefined, 'total_tokens 缺 → 未知（不做加法虚构）');
  assert.equal(ioOnly.cacheHitTokens, undefined, '命中缺 → 未知');

  // usage 在事件体 message 里（含回包模型名）。
  const wrapped = parseUsageChunk({type: 'message_start', message: {model: 'm-y', usage: {input_tokens: 10}}});
  assert.equal(wrapped.promptTokens, 10);
  assert.equal(wrapped.model, 'm-y', '回包 model 字段随块归一');

  // 无 usage 的块 / 非对象 → null（不上报、不编数）。
  assert.equal(parseUsageChunk({choices: [{delta: {content: 'a'}}]}), null, '纯文本块无遥测 → null');
  assert.equal(parseUsageChunk('data: [DONE]'), null);
  assert.equal(parseUsageChunk(null), null);

  // mergeUsage：逐字段后到非空者胜（null/undefined 不覆盖；真实 0 是值照胜）。
  assert.deepEqual(mergeUsage({promptTokens: 10, completionTokens: 3}, {completionTokens: 9}), {
    promptTokens: 10,
    completionTokens: 9,
  });
  assert.equal(mergeUsage({promptTokens: 10}, {}).promptTokens, 10, '后块空字段不覆盖前值');
  assert.equal(mergeUsage({promptTokens: 10}, {promptTokens: 0}).promptTokens, 0, '后到真实 0 照胜');
  assert.equal(mergeUsage(null, {promptTokens: 7}).promptTokens, 7);
  assert.equal(mergeUsage(null, null), null, '两侧全空 → null');
  assert.equal(mergeUsage({}, {}), null, '空对象对空对象 → null');

  // 多块流式攒量：逐块 merge 后总量块赢、早块缺字段保留。
  let acc = null;
  for (const chunk of [
    {usage: {prompt_tokens: 1000, completion_tokens: 10}},
    {usage: {prompt_cache_hit_tokens: 600}},
    {usage: {prompt_tokens: 1000, completion_tokens: 42, total_tokens: 1042}},
  ]) {
    acc = mergeUsage(acc, parseUsageChunk(chunk));
  }
  assert.deepEqual(acc, {promptTokens: 1000, completionTokens: 42, totalTokens: 1042, cacheHitTokens: 600});
  console.log('  -> PASS: 代表性流块解析正确；mergeUsage 后到非空者胜');
}

// ---------------------------------------------------------------------------
// 4. 耗时格式（真实测量值的展示口径）
// ---------------------------------------------------------------------------
{
  assert.equal(formatDuration(0), '0ms', '真实 0 不占位');
  assert.equal(formatDuration(820), '820ms');
  assert.equal(formatDuration(3200), '3.2s');
  assert.equal(formatDuration(45000), '45s');
  assert.equal(formatDuration(125000), '2m05s');
  assert.equal(formatDuration(undefined), DASH);
  assert.equal(formatDuration(null), DASH);
  console.log('  -> PASS: 耗时展示口径（未知「—」、真实 0 显示 0）');
}

// ---------------------------------------------------------------------------
// 5. source-mirror：状态行渲染遥测（title hover + 「—」占位）+ runtime 接线
// ---------------------------------------------------------------------------
{
  // App.svelte：聊天头状态行确实渲染遥测四项 + hover 详情。
  assert.ok(/class="statusline"[\s\S]{0,2500}class="turn-telemetry"/.test(appSrc), '状态行挂遥测条');
  assert.ok(appSrc.includes('title={turnTelemetryView.hover}'), 'hover 详情走 title 属性');
  for (const slot of ['tokens', 'cache', 'duration', 'model']) {
    assert.ok(appSrc.includes(`{turnTelemetryView.${slot}}`), `状态行渲染遥测位 ${slot}`);
  }
  assert.ok(appSrc.includes('「—」占位'), '「—」诚实占位口径写明在渲染侧');
  assert.ok(appSrc.includes('formatTurnTelemetry'), '显示串由纯函数产出（不手搓数字）');

  // runtime.ts：三条流式路径（直连协议 A / B / 本地网关）都把 usage 接到 onUsage。
  assert.ok(/onUsage\?: \(usage: TurnUsage\) => void/.test(runtimeSrc), 'StreamCallbacks 有 onUsage 回调');
  const wiringCount = (runtimeSrc.match(/parseUsageChunk\(json\)/g) ?? []).length;
  assert.equal(wiringCount, 3, '三条流式路径各解析一次 usage（实际=' + wiringCount + '）');
  const emitCount = (runtimeSrc.match(/callbacks\.onUsage\?\.\(usage\)/g) ?? []).length;
  assert.equal(emitCount, 3, '三条流式路径各上报一次 onUsage（实际=' + emitCount + '）');
  assert.ok(
    runtimeSrc.includes("onUsage: (usage) => onEvent({type: 'usage', requestId, usage})"),
    'createAgentRuntime 把 onUsage 转发成 usage 事件（类型安全上行）',
  );
  assert.ok(
    runtimeSrc.includes("{type: 'usage'; requestId: string; usage: TurnUsage}"),
    'RuntimeEvent 有 usage 事件型',
  );
  assert.ok(appSrc.includes("event.type === 'usage'"), 'App.svelte 接住 usage 事件攒本回合快照');
  assert.ok(/performance\.now\(\)/.test(appSrc), '回合耗时 = 真实测量');
  console.log('  -> PASS: source-mirror——状态行遥测渲染 + 三路 usage 接线 + 事件上行');
}

console.log('turn telemetry: all assertions passed');
