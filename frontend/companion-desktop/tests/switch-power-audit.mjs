// 开关权力审计表入档锁定（内测整改④）：
//   「开关名 = 开关权」的核查产出必须入档成表（开关名/声称范围/实际范围/处置
//   四列），逐开关给出核查结论与处置，摆设开关与幽灵权力都在表里留痕。本套件
//   锁：表在档、四列结构齐、每个权力面开关都有行、关键行的结论与处置到位。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const auditPath = join(testsDir, '..', '..', '..', 'docs', '02-guides', 'capability-switch-power-audit.md');
const audit = readFileSync(auditPath, 'utf8');

console.log('--- Starting Switch Power Audit Table Check ---');

// ---- 1. 表在档 + 四列结构 ----
{
  const header = audit.split('\n').find((line) => line.startsWith('| 开关名'));
  assert.ok(header, '审计表必须有「开关名」表头行');
  for (const column of ['开关名', '声称范围', '实际范围', '处置']) {
    assert.ok(header.includes(column), `审计表列缺「${column}」：${header}`);
  }
  console.log('  -> PASS: 审计表在档（开关名/声称范围/实际范围/处置 四列）');
}

// ---- 2. 每个权力面开关都有行 ----
{
  for (const switchName of [
    'APEIRETH_ENABLE_FILE_WRITE',
    'APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS',
    'APEIRETH_ENABLE_SHELL',
    'APEIRETH_SHELL_SANDBOX',
    'APEIRETH_ENABLE_FETCH',
    'APEIRETH_ENABLE_MCP',
    'APEIRETH_ENABLE_LOCAL_READ_TOOLS',
    'APEIRETH_ENABLE_EDUCATION',
  ]) {
    const row = audit.split('\n').find((line) => line.startsWith('| ') && line.includes(switchName));
    assert.ok(row, `审计表缺开关行：${switchName}`);
  }
  // 无开关的恒授权面也要如实登记。
  for (const alwaysOn of ['tool.repo', 'tool.self_status']) {
    assert.ok(audit.includes(alwaysOn), `审计表必须登记恒授权面：${alwaysOn}`);
  }
  console.log('  -> PASS: 权力面开关逐行入档（含恒授权面登记）');
}

// ---- 3. 关键行结论与处置到位 ----
{
  const fileWriteRow = audit
    .split('\n')
    .find((line) => line.startsWith('| ') && line.includes('`APEIRETH_ENABLE_FILE_WRITE`'));
  assert.ok(fileWriteRow, 'file_write 行必须在表内');
  assert.ok(fileWriteRow.includes('唯一写总闸'), 'file_write 行必须写明「唯一写总闸」结论');
  assert.ok(fileWriteRow.includes('shell'), 'file_write 行必须写明 shell 写命令同受此闸');
  assert.ok(fileWriteRow.includes('拒绝即帧'), 'file_write 行必须写明关闸语义');
  assert.ok(fileWriteRow.includes('已整改'), 'file_write 行处置必须是「已整改」');

  // 摆设开关病灶本身要在表里留痕（整改前有洞的事实记录）。
  assert.ok(audit.includes('整改前有洞'), '审计表必须留痕整改前的摆设开关病灶');

  // 名实不符的小口与反向缺口都要有处置（描述对齐 / 挂账），不许静默略过。
  assert.ok(audit.includes('描述对齐'), 'localReadTools 小口必须有「描述对齐」处置');
  assert.ok(audit.includes('挂账'), 'education 反向缺口必须有「挂账」处置');

  // 复查问题的正面回答（fetch 管不管住 fetch / shell 全部行为分层）。
  assert.ok(audit.includes('fetch 管不管住'), '审计表必须回答「fetch 管不管住 fetch」');
  assert.ok(audit.includes('三个开关分层'), '审计表必须写明 shell 行为的开关分层事实');
  console.log('  -> PASS: 关键行结论/处置/病灶留痕到位');
}

console.log('--- All Switch Power Audit Table Checks PASSED! ---');
