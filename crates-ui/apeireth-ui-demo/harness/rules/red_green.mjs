#!/usr/bin/env node
// 规则层差分装置 —— **红-绿自证**（脚本化；可复跑）。
//
// ## 它证明什么
//
// 装置的「真的在比」不是形式 ⇒ 每条规则都要**实测**一次红-绿：
//
// - **普通规则**（`parity: "same"`）：临时篡改某个 `expected`
//   ⇒ 对应 Rust 测试**必须变红**（退出码 101，且打印**两侧值**）
//   ⇒ 恢复后**必须回绿**（退出码 0）。
// - **已知漂移**（`parity: "known_drift"`）：把 `expected` **改成 `rust_expected`**
//   （= 模拟「漂移被修好 / 两侧一致」）
//   ⇒ 装置**必须变红**并提示**「已知漂移已被修好 ⇒ 请更新 cases.json」**
//   ⇒ 恢复后**必须回绿**。
//
// ## 用法
//
// ```bash
// # 普通规则（可给多条）
// node crates-ui/apeireth-ui-demo/harness/rules/red_green.mjs \
//   short_id span_depth guard_event_row
//
// # 已知漂移用例（漂移「被修好」必须变红）
// node crates-ui/apeireth-ui-demo/harness/rules/red_green.mjs --drift
// ```
//
// ## 纪律
//
// - `expected` 由机器写 ⇒ 本脚本**只临时改** ⇒ 每条跑完立刻从备份恢复
//   ⇒ 跑完的 `cases.json` **md5 必须回到原值**（脚本自检）。
// - 本脚本**只读**生产前端与 Rust 侧 ⇒ 不改任何生产代码。

import {spawnSync} from 'node:child_process';
import {readFileSync, writeFileSync, copyFileSync, rmSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
// crates-ui/apeireth-ui-demo/harness/rules ⇒ 仓库根（4 级）。
const REPO = resolve(HERE, '..', '..', '..', '..');
const CASES = join(HERE, 'cases.json');
const BACKUP = '/tmp/cases.red_green.backup.json';
const CARGO_DIR = join(REPO, 'crates-ui');
const TARGET_DIR = process.env.RULES_TARGET_DIR ?? '/home/bai/Projects/apeireth-ui-target';

const md5 = (path) => createHash('md5').update(readFileSync(path)).digest('hex');

/** 已知漂移用例清单（`--drift` 模式跑这些）。 */
const DRIFT_CASES = [
  {rule: 'is_approval_expired', id: 'expires-null'},
  {rule: 'resolve_budget_limit_input', id: 'known-drift-unsafe-int'},
  {rule: 'merge_session_inbox', id: 'merge-created-at-no-offset-known-drift'},
  {rule: 'merge_session_inbox', id: 'merge-created-at-date-only-known-drift'},
  {rule: 'span_depth', id: 'span-depth-empty-parent'},
];

/** 篡改：把一个值换成**必定不等**的另一个值。 */
function tamper(value) {
  if (typeof value === 'string') return `${value}«tampered»`;
  if (typeof value === 'number') return value + 1;
  if (typeof value === 'boolean') return !value;
  if (value === null || value === undefined) return '«tampered»';
  return {__tampered__: true};
}

function runCargo(testName) {
  const result = spawnSync(
    'cargo',
    ['test', '-p', 'apeireth-ui-kit', '--test', 'rules_parity', '--locked', '--', testName],
    {
      cwd: CARGO_DIR,
      encoding: 'utf8',
      env: {...process.env, CARGO_TARGET_DIR: TARGET_DIR, RUSTC_WRAPPER: ''},
      timeout: 900_000,
    },
  );
  return {
    code: result.status,
    out: `${result.stdout ?? ''}${result.stderr ?? ''}`,
  };
}

const driftMode = process.argv.includes('--drift');
const rules = process.argv.slice(2).filter((arg) => !arg.startsWith('--'));
if (!driftMode && rules.length === 0) {
  console.error('用法：red_green.mjs <rule> [<rule>…] | red_green.mjs --drift');
  process.exit(2);
}

copyFileSync(CASES, BACKUP);
const originalMd5 = md5(CASES);
let failures = 0;

if (driftMode) {
  for (const {rule, id} of DRIFT_CASES) {
    const doc = JSON.parse(readFileSync(CASES, 'utf8'));
    const entry = doc.rules.find((r) => r.rule === rule);
    const testCase = entry?.cases.find((c) => c.id === id);
    if (!testCase) {
      console.error(`✗ 漂移用例「${rule}/${id}」不在真源里`);
      failures += 1;
      continue;
    }
    testCase.expected = testCase.rust_expected; // 假装漂移已被修好
    writeFileSync(CASES, `${JSON.stringify(doc, null, 2)}\n`, 'utf8');

    const testName = `${rule}_matches_production_typescript`;
    const red = runCargo(testName);
    const redOk = red.code === 101 && red.out.includes('已知漂移已被修好');
    copyFileSync(BACKUP, CASES);
    const green = runCargo(testName);
    const greenOk = green.code === 0;
    if (!redOk || !greenOk) failures += 1;
    console.log(`${redOk && greenOk ? '✓' : '✗'} 漂移用例「${rule}/${id}」`);
    console.log(
      `     假装漂移被修好 ⇒ 退出码 ${red.code}（期望 101，且提示「已知漂移已被修好」）${redOk ? ' ✓' : ' ✗'}`,
    );
    console.log(`     恢复后 ⇒ 退出码 ${green.code}（期望 0）${greenOk ? ' ✓' : ' ✗'}`);
  }
} else {
  for (const rule of rules) {
    const doc = JSON.parse(readFileSync(CASES, 'utf8'));
    const entry = doc.rules.find((r) => r.rule === rule);
    if (!entry) {
      console.error(`✗ 规则「${rule}」不在真源里`);
      failures += 1;
      continue;
    }
    const target = entry.cases.find((c) => c.parity !== 'known_drift');
    if (!target) {
      console.error(`✗ 规则「${rule}」没有 same 用例`);
      failures += 1;
      continue;
    }
    target.expected = tamper(target.expected);
    writeFileSync(CASES, `${JSON.stringify(doc, null, 2)}\n`, 'utf8');

    const testName = `${rule}_matches_production_typescript`;
    const red = runCargo(testName);
    const redOk =
      red.code === 101 &&
      red.out.includes('TS 侧（expected）') &&
      red.out.includes('Rust 侧（实际）');
    copyFileSync(BACKUP, CASES);
    const green = runCargo(testName);
    const greenOk = green.code === 0;
    if (!redOk || !greenOk) failures += 1;
    console.log(`${redOk && greenOk ? '✓' : '✗'} 规则「${rule}」：篡改用例「${target.id}」`);
    console.log(`     红：退出码 ${red.code}（期望 101）${redOk ? ' ✓' : ' ✗'}`);
    console.log(`     绿：退出码 ${green.code}（期望 0）${greenOk ? ' ✓' : ' ✗'}`);
    if (!redOk) {
      console.log(
        red.out
          .split('\n')
          .filter((l) => l.includes('TS 侧') || l.includes('Rust 侧') || l.includes('panicked'))
          .slice(0, 6)
          .map((l) => '     ' + l)
          .join('\n'),
      );
    }
  }
}

rmSync(BACKUP, {force: true});
const restored = md5(CASES) === originalMd5;
console.log('');
console.log(`真源 md5 恢复：${restored ? '✓ ' + originalMd5 : `✗ ${md5(CASES)} ≠ ${originalMd5}`}`);
if (!restored) failures += 1;
console.log(`汇总：失败 ${failures} 项`);
process.exit(failures === 0 ? 0 : 1);
