// 待批准卡浮出链文案锁定（内测整改）：
//   apply_patch 等任意写入工具走 RequireApproval 冻结时，聊天内必须出现
//   「等待批准：<工具>」帧且待批准卡跟着挂起记录浮出——这条链不许按工具名
//   白名单特判（否则某类工具冻结了界面却无卡，人批无门）。锁三处：
//   ① 等待标记按待批工具名通用拼装（无工具白名单）；
//   ② 标记数据源是挂起记录的 tool_name（不是常量文案）；
//   ③ 待批准卡按 approval_id 跟随挂起记录（同一视图模型，任何工具同链）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const appSrc = readFileSync(join(testsDir, '..', 'src', 'App.svelte'), 'utf8');
const runtimeSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'runtime.ts'), 'utf8');

console.log('--- Starting Pending Approval Marker Check ---');

// ---- ① 等待标记按工具名通用拼装 ----
{
  const marker = /const marker = `等待批准：\$\{toolName \|\| '工具'\}`/.exec(appSrc);
  assert.ok(marker, '等待标记必须按待批工具名通用拼装（缺省兜底「工具」），不得硬编码工具名');
  // 白名单反证：标记构造处不许出现具体工具名特判。
  const fn = /function withPendingMarker\([\s\S]{0,400}?\n {2}\}/.exec(appSrc)?.[0] ?? '';
  assert.ok(fn, '必须有 withPendingMarker 构造函数');
  for (const banned of ['shell', 'apply_patch', 'fetch']) {
    assert.ok(
      !fn.includes(banned),
      `等待标记构造不得按工具名特判：${banned}`,
    );
  }
}

// ---- ② 标记数据源是挂起记录的 tool_name ----
{
  const calls = [...appSrc.matchAll(/withPendingMarker\(([^)]*)\)/g)].map((m) => m[1]);
  assert.ok(calls.length > 0, 'App.svelte 必须调用 withPendingMarker');
  assert.ok(
    calls.some((args) => /tool_name|toolName/i.test(args)),
    `withPendingMarker 的实参必须来自挂起记录的工具名字段：${JSON.stringify(calls)}`,
  );
}

// ---- ③ 待批准卡按 approval_id 跟随挂起记录 ----
{
  assert.ok(
    /pendingCanonical\?\.approval_id|pendingCanonical\.approval_id/.test(appSrc),
    '待批准卡必须按挂起记录的 approval_id 定位（同一视图模型, 任何工具同链）',
  );
  // 挂起载荷来源：canonical pending 记录（202 载荷/事件），字段含 tool_name。
  assert.ok(
    /tool_name/.test(runtimeSrc),
    'runtime.ts 的挂起载荷必须带 tool_name（等待标记的数据源）',
  );
}

console.log('--- Pending Approval Marker Check Passed ---');
