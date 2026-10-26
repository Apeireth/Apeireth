#!/usr/bin/env node
// 规则层差分装置 — `expected` 生成器（**生产 TypeScript 侧真值**）。
//
// 用法（**必须**带 --experimental-strip-types，Node 22.x 才能直接 import 生产 .ts）：
//
//   node --experimental-strip-types crates-ui/apeireth-ui-demo/harness/rules/gen_expected.mjs
//   node --experimental-strip-types crates-ui/apeireth-ui-demo/harness/rules/gen_expected.mjs --check
//
// --check：只重算并与磁盘上的 cases.json 比对（有差异 ⇒ 退出码 1）⇒ 可当 CI 门禁。
//
// 纪律：
// - 生产前端**只读引入**（`frontend/companion-desktop/src/lib/**`），本脚本零改动生产文件。
// - `expected` 一律由**生产 TS 实算**；某条规则在 TS 侧跑不通（依赖浏览器 API 等）
//   ⇒ **跳过该条并显式登记**（`ts_status: "skipped"`），**绝不伪造**。
// - 归一化规则与 cases.json 的 `normalization` 字段一一对应；Rust 侧适配器按同一口径实现
//   （见 crates-ui/apeireth-ui-kit/tests/rules_parity.rs）。
//
// ── 时区口径（**两侧对齐**，见 README「时区口径」一节）────────────────────────
//
// 生产 `formatClockMs` / `formatDateTimeMs` 走 `toLocaleTimeString('zh-CN')`
// ⇒ 产出依赖**本地时区** ⇒ Node 侧必须与 Rust 侧**同处一个时区**才有可比性。
// ⇒ 本脚本在**进程最早期**（任何 `Date` / `Intl` 调用之前）设 `process.env.TZ`：
//     Node ≥ 16 支持**运行中**改 `process.env.TZ`（会重置 ICU 的时区缓存）
//     ⇒ 实测 `Intl.DateTimeFormat().resolvedOptions().timeZone` 变为 `Asia/Shanghai`
//     ⇒ 且 `toLocaleTimeString` 的输出真的变成 **+08:00** 的值（见 `assertTimeZone`）。
// ⇒ Rust 侧在测试里注入**固定 +08:00** 的 `TimeTextFormatter`（不改动引擎默认的
//   `UtcTimeText` 语义）⇒ 两侧同在 `Asia/Shanghai` ⇒ 比的是「非 UTC 时区下也逐字一致」。

import {readFileSync, writeFileSync} from 'node:fs';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {dirname, join, resolve} from 'node:path';

// ── 时区（**必须在任何 Date / Intl 调用之前**） ─────────────────────────────

/** 装置固定的时区（两侧同口径）。 */
const TIME_ZONE = 'Asia/Shanghai';

process.env.TZ = TIME_ZONE;

const HERE = dirname(fileURLToPath(import.meta.url));
// crates-ui/apeireth-ui-demo/harness/rules ⇒ 仓库根（4 级）。
const REPO = resolve(HERE, '..', '..', '..', '..');
const CASES_PATH = join(HERE, 'cases.json');

const CHECK_ONLY = process.argv.includes('--check');

/**
 * 自检：**时区真的切到 +08:00 了吗**（不是「设了环境变量就算数」）。
 *
 * 两道判据：
 * ① `Intl` 解析出的时区名 == `Asia/Shanghai`；
 * ② 一个**已知时刻**（`2026-10-24T09:30:00Z`）的 `toLocaleTimeString` 必须给
 *    `17:30:00`（= UTC + 8 小时）⇒ 证明输出真的按 +08:00 渲染。
 *
 * 任一不满足 ⇒ 退出码 1（**不产出可能错的期望值**）。
 */
function assertTimeZone() {
  const resolved = Intl.DateTimeFormat().resolvedOptions().timeZone;
  if (resolved !== TIME_ZONE) {
    console.error(
      `[rules] 时区未生效：期望 ${TIME_ZONE}，实测 ${resolved} ⇒ 退出码 1`,
    );
    process.exit(1);
  }
  const probe = new Date(1792834200000).toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
  if (probe !== '17:30:00') {
    console.error(
      `[rules] 时区自检失败：2026-10-24T09:30:00Z 应渲染成 17:30:00（+08:00），实测 ${probe} ⇒ 退出码 1`,
    );
    process.exit(1);
  }
  console.log(`[rules] 时区自检通过：TZ=${TIME_ZONE}（2026-10-24T09:30:00Z ⇒ ${probe}）`);
}

// ── 生产侧模块（只读引入） ────────────────────────────────────────────────

async function loadProduction() {
  const lib = join(REPO, 'frontend', 'companion-desktop', 'src', 'lib');
  const governance = join(lib, 'governance');
  const ledger = await import(pathToFileURL(join(governance, 'ledger.ts')).href);
  const format = await import(pathToFileURL(join(governance, 'format.ts')).href);
  const runtime = await import(pathToFileURL(join(lib, 'runtime.ts')).href);
  const telemetry = await import(
    pathToFileURL(join(lib, 'chat-shell', 'turn-telemetry.ts')).href
  );
  const budget = await import(pathToFileURL(join(lib, 'budget.ts')).href);
  return {ledger, format, runtime, telemetry, budget};
}

// ── 归一化（与 Rust 侧适配器同口径；cases.json 的 `normalization` 字段登记同一句话） ──

/** 账本条目 ⇒ 归一化条目（缺失与 null 一律归一为 null；键序固定）。 */
function normalizeEntry(entry) {
  return {
    approvalId: entry.approval.approval_id,
    session: entry.approval.session,
    sessionTitle: entry.sessionTitle ?? null,
    createdAtMs: entry.createdAtMs ?? null,
    expiresAtMs: entry.expiresAtMs ?? null,
  };
}

/** 条目数组 ⇒ {length, items}（顺序敏感）。 */
function normalizeEntries(entries) {
  return {length: entries.length, items: entries.map(normalizeEntry)};
}

/** 能力声明 ⇒ 归一化对象（未命中 ⇒ found=false 且其余全 null）。 */
function normalizeCapability(capability) {
  if (!capability) {
    return {
      found: false,
      id: null,
      supported: null,
      available: null,
      reason: null,
      aliasOf: null,
    };
  }
  return {
    found: true,
    id: capability.id,
    supported: capability.supported ?? null,
    available: capability.available ?? null,
    reason: capability.reason ?? null,
    aliasOf: capability.alias_of ?? null,
  };
}

/** 会话消耗显示视图 ⇒ 定长五字段对象。 */
function normalizeSessionTotals(view) {
  return {
    tokens: view.tokens,
    cache: view.cache,
    turns: view.turns,
    duration: view.duration,
    toolCalls: view.toolCalls,
  };
}

/** 生效值徽标 ⇒ 定长四字段对象。 */
function normalizeBadge(badge) {
  return {
    effective: badge.effective,
    source: badge.source,
    note: badge.note,
    label: badge.label,
  };
}

/** 预算余量行 ⇒ 定长七字段对象（ratio 可为 null）。 */
function normalizeRemainingRow(row) {
  return {
    key: row.key,
    label: row.label,
    scope: row.scope,
    cap: row.cap,
    used: row.used,
    remaining: row.remaining,
    ratio: row.ratio ?? null,
  };
}

/**
 * 把生产的「当前时刻」钉到用例给的 `nowMs`（**生产文件零改动**）。
 *
 * 生产的 `formatDateTimeMs(ts)` 内部用 `new Date()`（= **真实当前时刻**）判「同日」
 * ⇒ 若不钉住，「同日」判定随运行时刻漂移 ⇒ 期望值不可复现（假红 / 假绿）。
 * ⇒ 这里在**调用期间**把全局 `Date` 换成「**无参构造 ⇒ 返回固定 now**」的子类
 * ⇒ ⇒ 生产函数**逐字未改**，只是「现在」被钉住 ⇒ 期望值可复现。
 */
function withPinnedNow(nowMs, fn) {
  const RealDate = globalThis.Date;
  class PinnedDate extends RealDate {
    constructor(...args) {
      if (args.length === 0) {
        super(nowMs);
      } else {
        super(...args);
      }
    }
  }
  globalThis.Date = PinnedDate;
  try {
    return fn();
  } finally {
    globalThis.Date = RealDate;
  }
}

/** 取生产的旋钮规格（未知键 ⇒ 显式失败，不静默跳过）。 */
function knobSpec(specs, key) {
  const spec = specs[key];
  if (!spec) {
    throw new Error(`未知旋钮键：${key}`);
  }
  return spec;
}

/**
 * 核对用例里的**旋钮行**（引擎侧宿主的输入）与生产常量表**逐条一致**。
 *
 * 生产 `budget.ts` 的 `effectiveBudget` / `budgetRemainingRows` **不接受**外部规格表
 * ⇒ 它们的规格**只能**来自生产常量表 `BUDGET_KNOB_SPECS`
 * ⇒ 引擎侧（宿主喂 `KnobRow`）必须与之逐条相同，否则两侧不可比。
 * ⇒ 这里**显式核对** ⇒ 不一致 ⇒ 抛错 ⇒ 退出码 1（**不产出可能错的期望值**）。
 */
function assertKnobRowsMatchProduction(rule, specs) {
  const rows = rule.knob_rows;
  if (rows === undefined) {
    return;
  }
  if (!Array.isArray(rows)) {
    throw new Error(`规则「${rule.rule}」的 knob_rows 必须是数组`);
  }
  for (const row of rows) {
    const spec = specs[row.key];
    if (!spec) {
      throw new Error(`规则「${rule.rule}」的 knob_rows 含未知旋钮键：${row.key}`);
    }
    const production = {
      min: spec.min,
      max: spec.max,
      fallback: spec.fallback,
      outOfRange: spec.outOfRange,
    };
    const fixture = {
      min: row.min,
      max: row.max ?? null,
      fallback: row.fallback,
      outOfRange: row.outOfRange,
    };
    if (JSON.stringify(production) !== JSON.stringify(fixture)) {
      throw new Error(
        `规则「${rule.rule}」的 knob_rows 与生产 BUDGET_KNOB_SPECS 不一致（${row.key}）：\n` +
          `  生产 = ${JSON.stringify(production)}\n` +
          `  用例 = ${JSON.stringify(fixture)}\n` +
          '  ⇒ 请更新 cases.json 的 knob_rows（引擎侧宿主的旋钮行必须与生产常量表同源）',
      );
    }
  }
}

/** 生产侧规则实现表（rule 名 ⇒ (args) => 归一化输出）。 */
function buildImpls({ledger, format, runtime, telemetry, budget}) {
  const specs = budget.BUDGET_KNOB_SPECS;
  return {
    is_approval_expired: ({entry, nowMs}) => ledger.isApprovalExpired(entry, nowMs),
    approval_filter_counts: ({ledger: rows, nowMs}) => {
      const counts = ledger.approvalFilterCounts(rows, nowMs);
      return {all: counts.all, open: counts.open, expired: counts.expired};
    },
    filter_approval_ledger: ({ledger: rows, filter, nowMs}) =>
      normalizeEntries(ledger.filterApprovalLedger(rows, filter, nowMs)),
    merge_session_inbox: ({ledger: rows, sessionId, inbox, sessionTitle}) =>
      normalizeEntries(ledger.mergeSessionInbox(rows, sessionId, inbox, sessionTitle)),
    format_relative_ms: ({tsMs, nowMs}) => format.formatRelativeMs(tsMs, nowMs),
    format_duration_ms: ({ms}) => format.formatDurationMs(ms),
    format_clock_ms: ({tsMs}) => format.formatClockMs(tsMs),
    format_date_time_ms: ({tsMs, nowMs}) =>
      // 「现在」被钉到用例的 `nowMs` ⇒ 同日判定可复现。
      withPinnedNow(nowMs, () => format.formatDateTimeMs(tsMs)),
    find_capability: ({manifest, id}) =>
      normalizeCapability(runtime.findCapability(manifest ?? null, id)),
    capability_available: ({manifest, id}) =>
      runtime.capabilityAvailable(manifest ?? null, id),
    capability_unavailable_reason: ({manifest, id}) =>
      runtime.capabilityUnavailableReason(manifest ?? null, id) ?? null,
    format_duration: ({ms}) => telemetry.formatDuration(ms ?? null),
    format_session_totals: ({totals}) =>
      normalizeSessionTotals(telemetry.formatSessionTotals(totals)),
    resolve_budget_limit_input: ({key, raw}) => {
      const resolution = budget.resolveBudgetLimitInput(raw, knobSpec(specs, key));
      return {
        value: resolution.value ?? null,
        effective: resolution.effective,
        status: resolution.status,
        feedback: resolution.feedback,
      };
    },
    budget_knob_badge: ({key, configured}) =>
      normalizeBadge(budget.budgetKnobBadge(configured, knobSpec(specs, key))),
    effective_budget: ({knobs}) => {
      const effective = budget.effectiveBudget(knobs ?? null);
      return {
        maxTurnRounds: normalizeBadge(effective.maxTurnRounds),
        maxToolCalls: normalizeBadge(effective.maxToolCalls),
        contextBudgetChars: normalizeBadge(effective.contextBudgetChars),
      };
    },
    budget_remaining_rows: ({knobs, totals}) => {
      const rows = budget.budgetRemainingRows(
        totals,
        budget.effectiveBudget(knobs ?? null),
      );
      return {length: rows.length, items: rows.map(normalizeRemainingRow)};
    },
  };
}

// ── 主流程 ────────────────────────────────────────────────────────────────

async function main() {
  assertTimeZone();
  const production = await loadProduction();
  const impls = buildImpls(production);
  const doc = JSON.parse(readFileSync(CASES_PATH, 'utf8'));
  const skipped = [];

  for (const rule of doc.rules ?? []) {
    const impl = impls[rule.rule];
    if (!impl) {
      // 新增规则但没有 TS 适配器 ⇒ 显式失败（不静默跳过）。
      console.error(`[rules] 规则「${rule.rule}」没有 TS 侧适配器 ⇒ 退出码 1`);
      process.exit(1);
    }
    // 预算类规则：核对用例的旋钮行与生产常量表一致（不一致 ⇒ 抛错 ⇒ 退出码 1）。
    if (rule.knob_rows !== undefined) {
      assertKnobRowsMatchProduction(rule, production.budget.BUDGET_KNOB_SPECS);
    }
    let ok = true;
    let error = null;
    for (const testCase of rule.cases ?? []) {
      try {
        testCase.expected = impl(testCase.args);
        delete testCase.ts_status;
        delete testCase.ts_error;
      } catch (err) {
        // TS 侧跑不通 ⇒ 跳过该条，**不伪造 expected**。
        ok = false;
        error = String(err && err.stack ? err.stack : err);
        delete testCase.expected;
        testCase.ts_status = 'skipped';
        testCase.ts_error = error;
      }
    }
    if (!ok) {
      skipped.push({rule: rule.rule, error});
      rule.ts_status = 'skipped';
    } else {
      delete rule.ts_status;
    }
  }

  doc.generator = {
    script: 'crates-ui/apeireth-ui-demo/harness/rules/gen_expected.mjs',
    node: process.version,
    mode: '--experimental-strip-types',
    tz: TIME_ZONE,
    production:
      'frontend/companion-desktop/src/lib/{governance/ledger.ts,governance/format.ts,runtime.ts,chat-shell/turn-telemetry.ts,budget.ts}',
  };

  const rendered = `${JSON.stringify(doc, null, 2)}\n`;
  const current = readFileSync(CASES_PATH, 'utf8');

  if (CHECK_ONLY) {
    if (rendered !== current) {
      console.error('[rules] cases.json 与生产 TS 实算结果不一致 ⇒ 退出码 1');
      process.exit(1);
    }
    console.log('[rules] cases.json 与生产 TS 实算结果一致（--check 通过）');
  } else {
    writeFileSync(CASES_PATH, rendered, 'utf8');
    const count = (doc.rules ?? []).reduce((sum, rule) => sum + (rule.cases ?? []).length, 0);
    console.log(`[rules] 已写回 cases.json：${(doc.rules ?? []).length} 条规则 / ${count} 个用例`);
  }

  if (skipped.length > 0) {
    for (const item of skipped) {
      console.error(`[rules] ⚠ 跳过规则「${item.rule}」（TS 侧跑不通 ⇒ 不伪造 expected）：`);
      console.error(item.error);
    }
    // 跳过的规则由 Rust 侧测试**显式跳过并打印提示**（不假绿）。
    process.exitCode = 0;
  }
}

main().catch((err) => {
  console.error('[rules] 生成失败：');
  console.error(err && err.stack ? err.stack : err);
  process.exit(1);
});
