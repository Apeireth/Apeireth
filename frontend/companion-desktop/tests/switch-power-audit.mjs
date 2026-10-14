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

// ---- 4. 开关生效链审计（续篇）：六列表 + 逐开关行覆盖 + 断点处置到位 ----
{
  assert.ok(
    audit.includes('开关生效链审计（续篇）'),
    '审计表必须有「开关生效链审计（续篇）」节',
  );
  const chainHeader = audit
    .split('\n')
    .find((line) => line.startsWith('| 开关 |') && line.includes('断点结论'));
  assert.ok(chainHeader, '续篇表必须有「开关 → env/配置接线 → 装配消费 → 运行时生效 → 自报读值 → 断点结论/修法」表头');
  for (const column of ['env / 配置接线', '装配消费', '运行时生效', '自报读值', '断点结论 / 修法']) {
    assert.ok(chainHeader.includes(column), `续篇表列缺「${column}」：${chainHeader}`);
  }

  // 逐开关一行（点名的 12 个开关全部入表）。
  const chainSection = audit.slice(audit.indexOf('开关生效链审计（续篇）'));
  for (const switchName of [
    'APEIRETH_COGNITIVE_COUNCIL',
    'APEIRETH_COGNITIVE_JUDGE',
    'APEIRETH_ENABLE_EDUCATION',
    'APEIRETH_ENABLE_ORGANS',
    'APEIRETH_ENABLE_PARTNER_BOND',
    'APEIRETH_ENABLE_REFLEXION',
    'APEIRETH_ENABLE_SELF_TUNING',
    'APEIRETH_ENABLE_MCP',
    'APEIRETH_ENABLE_MORPHOLOGY_RECALL',
    'APEIRETH_ENABLE_ABSORPTION_INSIGHT',
    'APEIRETH_ENABLE_COMMUNITY_TRIAGE',
    'APEIRETH_ENABLE_CONSOLIDATION',
  ]) {
    const row = chainSection
      .split('\n')
      .find((line) => line.startsWith('| ') && line.includes(switchName));
    assert.ok(row, `续篇表缺开关行：${switchName}`);
  }

  // 断点处置口径逐条在档（不许静默略过）。
  assert.ok(chainSection.includes('按实际注册条件取值'), '自报读值断点必须写明按实际注册条件取值');
  assert.ok(chainSection.includes('同源'), 'council 数值旋钮 / education 授权修法必须写明同源口径');
  assert.ok(chainSection.includes('不硬造开关'), 'MCP 无开关面必须写明不硬造开关');
  assert.ok(chainSection.includes('需另行配置'), 'MCP 行必须写明「需另行配置」');
  assert.ok(
    chainSection.includes('已启用但无服务器配置 = 无外部工具'),
    'MCP 自报语义必须如实入档',
  );
  assert.ok(chainSection.includes('桌面面无此开关，CLI 旋钮'), 'MCP 必须明示桌面无此开关面');
  console.log('  -> PASS: 开关生效链审计（续篇）六列结构 + 12 开关行 + 断点处置到位');
}

console.log('--- All Switch Power Audit Table Checks PASSED! ---');
