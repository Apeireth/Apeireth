// 开关描述全责文案锁定（内测整改④）：
//   文件写入总闸的全责必须明示在三处开关描述落点——前端能力开关描述（CapDef
//   desc）/ env 注释 / 用户手册——同一句话，防止描述与权力再次脱节（摆设开关
//   的另一半就是"描述说一套、权力另一套"）。同时锁 shell 撞闸时的拒绝帧文案
//   与帧 code（拒绝信息即帧）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(testsDir, '..', '..', '..');

const DUTY = '文件写入总闸：apply_patch 与 shell 写命令同受此闸';
const DENIAL = '文件写入开关未开——shell 写命令受同一总闸管辖';

const settingsSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'views', 'SettingsView.svelte'),
  'utf8',
);
const cliSrc = readFileSync(join(repoRoot, 'crates', 'adapters', 'cli', 'src', 'lib.rs'), 'utf8');
const userManual = readFileSync(join(repoRoot, 'docs', '02-guides', 'user-manual.md'), 'utf8');
const apiDoc = readFileSync(join(repoRoot, 'docs', '03-reference', 'api.md'), 'utf8');
const shellSrc = readFileSync(
  join(repoRoot, 'crates', 'capabilities', 'tools', 'src', 'shell.rs'),
  'utf8',
);
const supervisorSrc = readFileSync(
  join(testsDir, '..', 'src-tauri', 'src', 'backend_supervisor.rs'),
  'utf8',
);

console.log('--- Starting Switch Scope Copy Check ---');

// ---- 1. 前端 CapDef desc：fileWrite 行必须明示全责 ----
{
  const fileWriteEntry = /key: 'fileWrite'[\s\S]{0,600}?\},/.exec(settingsSrc)?.[0] ?? '';
  assert.ok(fileWriteEntry, '设置页必须有 fileWrite 能力行');
  assert.ok(fileWriteEntry.includes(DUTY), `fileWrite 行 desc 必须明示全责：「${DUTY}」`);
  // 开关名实相符：label 同时点名两个受闸面（apply_patch 与 shell 写命令）。
  assert.ok(
    /label: '文件写入（apply_patch · shell 写命令）'/.test(fileWriteEntry),
    'fileWrite 行 label 必须同时点名 apply_patch 与 shell 写命令',
  );
  console.log('  -> PASS: 前端 CapDef desc 明示全责');
}

// ---- 2. env 注释：开关定义处与解析函数处都落全责句 ----
{
  const envBlock = /受控文件写入旋钮[\s\S]{0,600}?ENABLE_FILE_WRITE_ENV/.exec(cliSrc)?.[0] ?? '';
  assert.ok(envBlock.includes(DUTY), `env 注释必须明示全责：「${DUTY}」`);
  const fnBlock = /受控文件写入主开关[\s\S]{0,300}?fn file_write_enabled_from_env/.exec(cliSrc)?.[0] ?? '';
  assert.ok(fnBlock.includes(DUTY), 'file_write_enabled_from_env 文档注释必须明示全责');
  // 桌面端 env 注入侧同口径。
  assert.ok(supervisorSrc.includes(DUTY), 'env 注入侧注释必须明示全责');
  console.log('  -> PASS: env 注释（定义处 + 解析处 + 注入侧）明示全责');
}

// ---- 3. 用户手册 + API 参考同口径 ----
{
  assert.ok(userManual.includes(DUTY), 'user-manual.md 必须明示全责');
  assert.ok(
    userManual.includes('拒绝即帧'),
    'user-manual.md 必须写明关闸语义（拒绝即帧）',
  );
  assert.ok(apiDoc.includes('文件写入总闸'), 'api.md 工具表同口径');
  console.log('  -> PASS: 用户手册 / API 参考明示全责');
}

// ---- 4. shell 撞闸拒绝帧文案 + 稳定 code（拒绝信息即帧）----
{
  assert.ok(shellSrc.includes(DENIAL), `shell.rs 必须携带拒绝帧原话：「${DENIAL}」`);
  assert.ok(
    /PipelineFailure::PreDenied\s*\{[^}]*shell_write_intent_gate/.test(shellSrc),
    'shell 写闸拒绝必须走 PreDenied 帧（code pipeline.pre_deny）',
  );
  console.log('  -> PASS: 拒绝帧文案与 code 锁定');
}

// ---- 5. 开关生效链审计（续篇）新增文案落点（设置页行 desc）----
{
  // MCP 无开关说明行: desc 必须写明「需另行配置」与如实自报语义（不硬造开关）。
  const mcpRow = /外部工具桥（MCP）[\s\S]{0,600}?<\/div>/.exec(settingsSrc)?.[0] ?? '';
  assert.ok(mcpRow, '设置页必须有外部工具桥（MCP）说明行');
  assert.ok(mcpRow.includes('需另行配置'), 'MCP 说明行 desc 必须写明「需另行配置」');
  assert.ok(
    mcpRow.includes('已启用但无服务器配置 = 无外部工具'),
    'MCP 说明行必须写明如实自报语义',
  );
  assert.ok(mcpRow.includes('无开关'), 'MCP 说明行必须明示无开关（不硬造开关）');

  // 挂槽开关的 desc 必须明示挂靠关系（开关名与权力相符: 记忆固化/检索深度自适应/
  // 图社区分诊/从使用中学习）。
  const dependencyRows = [
    [/key: 'consolidation'[\s\S]{0,600}?\},/, '记忆固化', '挂在记忆写入模块上'],
    [/key: 'morphologyRecall'[\s\S]{0,600}?\},/, '检索深度自适应', '挂在记忆召回模块上'],
    [/key: 'communityTriage'[\s\S]{0,600}?\},/, '图社区分诊', '需记忆召回模块与图谱后端'],
    [
      /从使用中学习<code class="cap-env">[\s\S]{0,600}?<\/small>/,
      '从使用中学习',
      '挂在记忆检索信号链上',
    ],
  ];
  for (const [pattern, rowName, needle] of dependencyRows) {
    const entry = pattern.exec(settingsSrc)?.[0] ?? '';
    assert.ok(
      entry.includes(needle),
      `${rowName} 行 desc 必须明示挂靠关系「${needle}」：${entry.slice(0, 120)}`,
    );
  }
  console.log('  -> PASS: 续篇开关文案落点锁定（MCP 说明行 + 挂槽开关 desc）');
}

console.log('--- All Switch Scope Copy Checks PASSED! ---');
