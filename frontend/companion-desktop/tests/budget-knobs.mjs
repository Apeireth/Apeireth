// 预算面板「旋钮面」（真实模块 src/lib/budget.ts / desktop-bridge.ts / types.ts），
// 并对 SettingsView / backend_supervisor.rs / CLI 旋钮解析做源码级镜像校验。
// 契约：
//   1. 钳制/非法回默认语义与后端解析同源：回合预算两旋钮越界钳 1..=64、非法
//      回默认 8/16；上下文字符预算正整数直通、越界/非法回默认 24000。
//   2. 提交反馈：钳制 / 回默认各有对应人话反馈（钳到 N / 回默认 N），
//      直通值不打扰。
//   3. 实际生效值徽标：source=configured/constant（自述预算节同款诚实语法，
//      镜像 self_status_source.rs 口径常量）。
//   4. 即效接线（env 注入字段）：config → capabilityEnvFromConfig 注入字段 →
//      backend_supervisor env_pairs → CLI 旋钮名，四处同名同源。
//   5. 配额维度注册表与读码事实一致：token/步数/花费/深度四维全部「暂无接口」，
//      不硬造假旋钮（镜像调度配额类型的字段与诚实注记）。
//   6. 预算耗尽行为：后端无可配置语义 → 设置面不出选择器，只出固定语义说明。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  BUDGET_EXHAUSTION,
  BUDGET_KNOB_SPECS,
  BUDGET_SOURCE_CONSTANT,
  BUDGET_SOURCE_CONFIGURED,
  DEFAULT_CONTEXT_BUDGET_CHARS,
  DEFAULT_MAX_ROUNDS,
  DEFAULT_MAX_TOOL_CALLS,
  QUOTA_DIMENSIONS,
  QUOTA_DIMENSION_KEYS,
  QUOTA_DIMENSION_STATUS_LABEL,
  budgetKnobBadge,
  effectiveBudget,
  resolveBudgetLimitInput,
} from '../src/lib/budget.ts';
import {capabilityEnvFromConfig} from '../src/lib/desktop-bridge.ts';
import {DEFAULT_CAPABILITY_TOGGLES} from '../src/lib/types.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(testsDir, '..', '..', '..');
const settingsSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'views', 'SettingsView.svelte'), 'utf8');
const runtimeRust = readFileSync(
  join(repoRoot, 'crates', 'engine', 'runtime', 'src', 'canonical', 'runtime.rs'),
  'utf8',
);
const executeRust = readFileSync(
  join(repoRoot, 'crates', 'engine', 'runtime', 'src', 'canonical', 'execute.rs'),
  'utf8',
);
const cliRust = readFileSync(join(repoRoot, 'crates', 'adapters', 'cli', 'src', 'lib.rs'), 'utf8');
const supervisorRust = readFileSync(
  join(testsDir, '..', 'src-tauri', 'src', 'backend_supervisor.rs'),
  'utf8',
);
const selfStatusRust = readFileSync(
  join(repoRoot, 'crates', 'engine', 'runtime-assembly', 'src', 'canonical', 'self_status_source.rs'),
  'utf8',
);
const quotaSchedulerRust = readFileSync(
  join(repoRoot, 'crates', 'foundation', 'orchestration', 'src', 'cognitive_quota_scheduler.rs'),
  'utf8',
);

const budgetSection =
  settingsSrc.split("activeSection === 'budget'")[1]?.split("activeSection === 'runtime'")[0] ?? '';

console.log('--- budget knob face ---');

// ---------------------------------------------------------------------------
// 1. 钳制/非法回默认语义与后端解析同源（回合预算两旋钮）
// ---------------------------------------------------------------------------
{
  // 常量同源：镜像后端解析常量（数值逐一对得上，不另立第二套数）。
  assert.ok(runtimeRust.includes('pub const DEFAULT_MAX_ROUNDS: u32 = 8;'), '后端默认轮数常量在案');
  assert.ok(runtimeRust.includes('pub const MIN_TURN_ROUNDS: u32 = 1;'), '轮数下界常量在案');
  assert.ok(runtimeRust.includes('pub const MAX_TURN_ROUNDS: u32 = 64;'), '轮数上界常量在案');
  assert.ok(runtimeRust.includes('pub const MIN_TOOL_CALL_LIMIT: usize = 1;'), '工具上限下界常量在案');
  assert.ok(runtimeRust.includes('pub const MAX_TOOL_CALL_LIMIT: usize = 64;'), '工具上限上界常量在案');
  assert.ok(executeRust.includes('pub const MAX_TOOL_CALLS_PER_ROUND: usize = 16;'), '单轮工具默认常量在案');
  assert.equal(DEFAULT_MAX_ROUNDS, 8, '前端默认轮数与后端常量同值');
  assert.equal(DEFAULT_MAX_TOOL_CALLS, 16, '前端单轮工具默认与后端常量同值');

  const rounds = BUDGET_KNOB_SPECS.maxTurnRounds;
  assert.equal(resolveBudgetLimitInput('4', rounds).value, 4, '直通值原样落位');
  assert.equal(resolveBudgetLimitInput(' 12 ', rounds).value, 12, '前后空白不算非法');
  const clampedHigh = resolveBudgetLimitInput('999', rounds);
  assert.equal(clampedHigh.value, 64, '越界钳制到上界');
  assert.equal(clampedHigh.status, 'clamped');
  assert.equal(clampedHigh.effective, 64);
  const clampedLow = resolveBudgetLimitInput('0', rounds);
  assert.equal(clampedLow.value, 1, '越界钳制到下界');
  assert.equal(resolveBudgetLimitInput('-7', rounds).value, 1);

  for (const raw of ['', '   ', 'abc', '3.5', null, undefined, Number.NaN]) {
    const bad = resolveBudgetLimitInput(raw, rounds);
    assert.equal(bad.value, null, `非法输入 ${String(raw)} 必须落回默认口径（null = 未配置）`);
    assert.equal(bad.effective, DEFAULT_MAX_ROUNDS, '非法回默认 8');
    assert.equal(bad.status, 'fallback');
  }

  const calls = BUDGET_KNOB_SPECS.maxToolCalls;
  assert.equal(resolveBudgetLimitInput('9999', calls).value, 64, '工具上限同语义钳制');
  assert.equal(resolveBudgetLimitInput('x', calls).effective, DEFAULT_MAX_TOOL_CALLS, '非法回默认 16');
  console.log('  -> PASS: 回合预算两旋钮 = 越界钳 1..=64、非法回默认 8/16（与后端解析同源）');
}

// ---------------------------------------------------------------------------
// 2. 上下文字符预算：正整数直通、越界/非法回默认（无上界钳制）
// ---------------------------------------------------------------------------
{
  const ctx = BUDGET_KNOB_SPECS.contextBudgetChars;
  const ok = resolveBudgetLimitInput('12000', ctx);
  assert.equal(ok.value, 12000);
  assert.equal(ok.status, 'ok');
  for (const raw of ['0', '-5', 'abc', '3.5', '']) {
    const bad = resolveBudgetLimitInput(raw, ctx);
    assert.equal(bad.value, null, `越界/非法 ${raw} 必须回默认口径`);
    assert.equal(bad.effective, DEFAULT_CONTEXT_BUDGET_CHARS, '非法回默认 24000');
    assert.equal(bad.status, 'fallback');
  }
  assert.equal(resolveBudgetLimitInput('9999999', ctx).value, 9999999, '无上界钳制：大正整数直通');
  assert.ok(runtimeRust.includes('pub const DEFAULT_CONTEXT_BUDGET_CHARS: usize = 24_000;'), '默认常量同源');
  assert.ok(/chars > 0/.test(cliRust), '后端解析 = 正数过滤（越界/非法回默认）');
  console.log('  -> PASS: 上下文字符预算 = 正整数直通、越界/非法回默认（无上界钳制）');
}

// ---------------------------------------------------------------------------
// 3. 提交反馈 + 实际生效值徽标（source=configured/constant 同款诚实语法）
// ---------------------------------------------------------------------------
{
  const rounds = BUDGET_KNOB_SPECS.maxTurnRounds;
  assert.ok(resolveBudgetLimitInput('999', rounds).feedback.includes('钳制到 64'), '钳制反馈点名实际生效值');
  assert.ok(resolveBudgetLimitInput('abc', rounds).feedback.includes('回默认 8'), '非法反馈点名回默认值');
  assert.equal(resolveBudgetLimitInput('5', rounds).feedback, '', '直通值无反馈（不打扰）');

  // 徽标口径与自述预算节同款（镜像口径常量）。
  assert.ok(selfStatusRust.includes('pub const BUDGET_SOURCE_CONSTANT: &str = "constant";'), 'constant 口径常量在案');
  assert.ok(selfStatusRust.includes('pub const BUDGET_SOURCE_CONFIGURED: &str = "configured";'), 'configured 口径常量在案');
  assert.equal(BUDGET_SOURCE_CONSTANT, 'constant');
  assert.equal(BUDGET_SOURCE_CONFIGURED, 'configured');

  const unconfigured = budgetKnobBadge(null, rounds);
  assert.equal(unconfigured.effective, DEFAULT_MAX_ROUNDS);
  assert.equal(unconfigured.source, 'constant');
  assert.equal(unconfigured.label, '生效 8 · constant');
  assert.ok(unconfigured.note.length > 0, 'constant 口径必须带注记');

  const configured = budgetKnobBadge(4, rounds);
  assert.equal(configured.effective, 4);
  assert.equal(configured.source, 'configured');
  assert.equal(configured.label, '生效 4 · configured');
  assert.ok(configured.note.length > 0, 'configured 口径必须带注记');

  assert.equal(budgetKnobBadge(999, rounds).effective, 64, '越界配置值经同一口径取生效值（钳制语义保留）');
  assert.equal(budgetKnobBadge(3.5, rounds).source, 'constant', '非法配置值 = 诚实回落 constant 默认');

  // 三枚旋钮的徽标集同一口径。
  const all = effectiveBudget({...DEFAULT_CAPABILITY_TOGGLES, maxToolCalls: 8});
  assert.equal(all.maxTurnRounds.source, 'constant');
  assert.equal(all.maxToolCalls.source, 'configured');
  assert.equal(all.maxToolCalls.effective, 8);
  assert.equal(all.contextBudgetChars.source, 'constant');
  console.log('  -> PASS: 反馈点名实际生效值；徽标 source=configured/constant + 口径注记');
}

// ---------------------------------------------------------------------------
// 4. 即效接线（env 注入字段）：四处同名同源
// ---------------------------------------------------------------------------
{
  const base = {...DEFAULT_CAPABILITY_TOGGLES};
  const unset = capabilityEnvFromConfig(base);
  assert.equal(unset.turn_round_limit, undefined, '未配置不得注入（后端默认现行为）');
  assert.equal(unset.tool_call_limit, undefined);
  assert.equal(unset.context_budget_chars, undefined);

  const wired = capabilityEnvFromConfig({
    ...base,
    maxTurnRounds: 4,
    maxToolCalls: 8,
    contextBudgetChars: 12000,
  });
  assert.equal(wired.turn_round_limit, 4, '旋钮即效接线：注入字段随配置走');
  assert.equal(wired.tool_call_limit, 8);
  assert.equal(wired.context_budget_chars, 12000);

  assert.equal(capabilityEnvFromConfig({...base, maxTurnRounds: 999}).turn_round_limit, 64, '注入面同源钳制');
  assert.equal(capabilityEnvFromConfig({...base, maxTurnRounds: 'x'}).turn_round_limit, undefined, '非法不注入 = 回默认');

  const ENV_NAMES = {
    maxTurnRounds: 'APEIRETH_MAX_TURN_ROUNDS',
    maxToolCalls: 'APEIRETH_MAX_TOOL_CALLS',
    contextBudgetChars: 'APEIRETH_CONTEXT_BUDGET_CHARS',
  };
  const envPairsIdx = supervisorRust.lastIndexOf('fn env_pairs');
  assert.ok(envPairsIdx > 0, 'backend_supervisor 必须有 env_pairs 实现');
  for (const [key, env] of Object.entries(ENV_NAMES)) {
    assert.equal(BUDGET_KNOB_SPECS[key].env, env, `${key} 注册表 env 名 = ${env}`);
    assert.ok(supervisorRust.includes(`("${env}",`), `backend_supervisor env_pairs 必须注入 ${env}`);
    assert.ok(cliRust.includes(`"${env}"`), `CLI 旋钮名必须同名：${env}`);
    assert.ok(settingsSrc.includes('<code class="cap-env">{spec.env}</code>'), '设置页 env 芯片走注册表（单一来源）');
  }
  console.log('  -> PASS: config → 注入字段 → env_pairs → CLI 旋钮名四处同名同源');
}

// ---------------------------------------------------------------------------
// 5. 配额维度注册表与读码事实一致（四维全部「暂无接口」，不硬造旋钮）
// ---------------------------------------------------------------------------
{
  assert.deepEqual([...QUOTA_DIMENSION_KEYS], ['tokens', 'steps', 'cost', 'depth'], '配额维度恰为四维');
  assert.deepEqual(QUOTA_DIMENSIONS.map((dim) => dim.key), [...QUOTA_DIMENSION_KEYS]);
  assert.equal(QUOTA_DIMENSION_STATUS_LABEL, '暂无接口');
  for (const dim of QUOTA_DIMENSIONS) {
    assert.equal(dim.interface, 'none', `${dim.key} 必须如实标注暂无接口`);
    assert.ok(dim.reason.includes(QUOTA_DIMENSION_STATUS_LABEL), `${dim.key} 原因必须写明「暂无接口」`);
    assert.ok(dim.evidence.length > 0, `${dim.key} 必须带读码出处`);
  }

  // 读码断言 ①：上限字段只在调度配额类型里（读码结论的字段级证据）。
  for (const field of ['pub max_tokens', 'pub max_tool_steps', 'pub max_cost_micros']) {
    assert.ok(quotaSchedulerRust.includes(field), `调度配额类型必须有 ${field}`);
  }
  assert.ok(!quotaSchedulerRust.includes('pub max_recursion_depth'), '深度上限字段已删（未实施不占位）');

  // 读码断言 ②：CLI/env 面没有任何 token/步数/花费/深度配额旋钮（暂无接口的证据）。
  for (const name of ['APEIRETH_MAX_TOKENS', 'APEIRETH_MAX_STEPS', 'APEIRETH_MAX_COST', 'APEIRETH_MAX_DEPTH']) {
    assert.ok(!cliRust.includes(name), `不存在的旋钮不得出现在 CLI env 面：${name}`);
  }

  // 设置面镜像：配额卡只渲染注册表行 + 「暂无接口」徽标，一个控件都不给。
  assert.ok(budgetSection.includes('{#each QUOTA_DIMENSIONS as dim (dim.key)}'), '配额卡走注册表渲染');
  assert.ok(budgetSection.includes('{QUOTA_DIMENSION_STATUS_LABEL}'), '「暂无接口」徽标按注册表文案渲染');
  const quotaCard = budgetSection.split('多维配额（Token')[1]?.split('预算耗尽行为')[0] ?? '';
  assert.ok(quotaCard.length > 0, '设置页有配额卡');
  assert.ok(!quotaCard.includes('<input') && !quotaCard.includes('<select'), '无接口维度禁止出现任何旋钮控件');
  console.log('  -> PASS: 配额四维全部「暂无接口」（读码断言 + 无假旋钮）');
}

// ---------------------------------------------------------------------------
// 6. 预算耗尽行为：无可配置语义 → 不出选择器，只出固定语义说明
// ---------------------------------------------------------------------------
{
  assert.equal(BUDGET_EXHAUSTION.configurable, false, '后端无可配置的耗尽行为语义');
  assert.ok(BUDGET_EXHAUSTION.behaviors.length > 0, '固定语义说明必须在案');
  assert.ok(budgetSection.includes('预算耗尽行为'), '设置页如实写明耗尽行为');
  assert.ok(budgetSection.includes('{#each BUDGET_EXHAUSTION.behaviors as behavior}'), '固定语义逐条渲染');
  const exhaustionCard = budgetSection.split('预算耗尽行为')[1] ?? '';
  assert.ok(exhaustionCard.length > 0);
  assert.ok(!exhaustionCard.includes('<select'), '没有语义可选 → 不出选择器');
  assert.ok(!exhaustionCard.includes('<input'), '没有语义可选 → 不出输入控件');
  assert.ok(exhaustionCard.includes('不出选择器'), '「不出」的原因为诚实写明');
  console.log('  -> PASS: 耗尽行为 = 固定语义说明（无选择器，如实不出）');
}

// ---------------------------------------------------------------------------
// 7. 旋钮即效接线（三级即效第 2 级：数字 = 失焦/回车提交）
// ---------------------------------------------------------------------------
{
  assert.ok(budgetSection.includes('type="number"'), '数字步进器（number 输入）');
  assert.ok(budgetSection.includes('onfocusout={() => void submitBudgetKnob(row.key)}'), '失焦提交');
  assert.ok(budgetSection.includes('isTextCommitKey(e.key)'), '回车提交（同一提交缝）');
  assert.ok(budgetSection.includes('{budgetFeedback[row.key]}'), '钳制/回默认反馈渲染在旋钮旁');

  const submitRegion = /async function submitBudgetKnob[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(submitRegion, 'SettingsView 必须有 submitBudgetKnob');
  assert.ok(submitRegion[0].includes('runLiveApply('), '数字旋钮走同一即效外壳（pending + 失败回填 + 横幅）');
  assert.ok(
    submitRegion[0].includes('onSave(configWithCapabilities(config, attempted))'),
    '即效写 config（叠加不覆盖其余字段）→ 同一条 apply 缝注入 env',
  );
  assert.ok(submitRegion[0].includes('budgetFeedback = '), '提交反馈在缝内收口');
  assert.ok(submitRegion[0].includes('clearTextDirty('), '失败/空提交清「未保存」标记');
  console.log('  -> PASS: 数字旋钮失焦/回车即效（同一 apply 缝 + 反馈 + 回填）');
}

console.log('budget knobs: all assertions passed');
