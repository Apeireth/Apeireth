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

import {readFileSync, writeFileSync} from 'node:fs';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {dirname, join, resolve} from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
// crates-ui/apeireth-ui-demo/harness/rules ⇒ 仓库根（4 级）。
const REPO = resolve(HERE, '..', '..', '..', '..');
const CASES_PATH = join(HERE, 'cases.json');

const CHECK_ONLY = process.argv.includes('--check');

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
  return {ledger, format, runtime, telemetry};
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

/** 生产侧规则实现表（rule 名 ⇒ (args) => 归一化输出）。 */
function buildImpls({ledger, format, runtime, telemetry}) {
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
    find_capability: ({manifest, id}) =>
      normalizeCapability(runtime.findCapability(manifest ?? null, id)),
    capability_available: ({manifest, id}) =>
      runtime.capabilityAvailable(manifest ?? null, id),
    capability_unavailable_reason: ({manifest, id}) =>
      runtime.capabilityUnavailableReason(manifest ?? null, id) ?? null,
    format_duration: ({ms}) => telemetry.formatDuration(ms ?? null),
    format_session_totals: ({totals}) =>
      normalizeSessionTotals(telemetry.formatSessionTotals(totals)),
  };
}

// ── 主流程 ────────────────────────────────────────────────────────────────

async function main() {
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
    production:
      'frontend/companion-desktop/src/lib/{governance/ledger.ts,governance/format.ts,runtime.ts,chat-shell/turn-telemetry.ts}',
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
