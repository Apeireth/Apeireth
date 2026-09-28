// 「保存按钮已删除」源码镜像断言：设置面三级即效语义落地后，每页的批量
// 「保存设置」按钮（页头按钮 + 底部保存栏）必须删干净，且动作类按钮（测试
// 连接 / 深度诊断 / 恢复默认类预设 / 危险动作）保留。
//
// 契约：
//   1. 页头不再挂「保存设置」按钮；全文无「保存设置」字样；
//   2. 底部批量保存栏（含样式）已删除；
//   3. 任何 <button> 都不再触发批量 apply 缝 handleSaveSettings（该缝保留为
//      服务商组失焦/回车提交与密钥动作共用的 apply 路径）；
//   4. 动作类按钮保留且仍在（不是被误删）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const srcDir = join(testsDir, '..', 'src');

const settingsSrc = readFileSync(join(srcDir, 'lib/views/SettingsView.svelte'), 'utf8');
const themePanelSrc = readFileSync(join(srcDir, 'lib/components/ThemeSettingsPanel.svelte'), 'utf8');

console.log('--- Starting Save-Button Removal (source mirror) Check ---');

/** 花括号感知的 <button> 开标签扫描（表达式里的 `=>` 不截断标签）。 */
function buttonTags(src) {
  const tags = [];
  let idx = 0;
  while ((idx = src.indexOf('<button', idx)) !== -1) {
    let i = idx + '<button'.length;
    let depth = 0;
    while (i < src.length) {
      const ch = src[i];
      if (ch === '{') depth += 1;
      else if (ch === '}') depth -= 1;
      else if (ch === '>' && depth === 0) break;
      i += 1;
    }
    tags.push(src.slice(idx, i + 1));
    idx = i + 1;
  }
  return tags;
}

// ---------------------------------------------------------------------------
// 1. 页头保存按钮已删除；全文无「保存设置」
// ---------------------------------------------------------------------------
{
  assert.ok(!settingsSrc.includes('保存设置'), '「保存设置」字样必须清干净（按钮与文案都删）');
  const header = /<PageHeader[\s\S]*?\/>/.exec(settingsSrc) ?? /<PageHeader[\s\S]*?>/.exec(settingsSrc);
  assert.ok(header, '设置页仍有 PageHeader');
  assert.ok(!header[0].includes('<button'), '页头不再挂任何按钮（原「保存设置」按钮已删除）');
  assert.ok(!settingsSrc.includes('saveSuccess'), '「已保存！」瞬时态随保存按钮一起退役');
  console.log('  -> PASS: 页头「保存设置」按钮已删除');
}

// ---------------------------------------------------------------------------
// 2. 底部批量保存栏（含样式）已删除
// ---------------------------------------------------------------------------
{
  for (const gone of ['settings-save-bar', 'save-bar-btn', 'save-bar-hint']) {
    assert.ok(!settingsSrc.includes(gone), `底部保存栏残留 ${gone} 必须清干净`);
  }
  assert.ok(!settingsSrc.includes('activeSection !== \'runtime\'') || !settingsSrc.includes('save-bar'), '保存栏分支已随按钮删除');
  console.log('  -> PASS: 底部批量保存栏（含样式）已删除');
}

// ---------------------------------------------------------------------------
// 3. 无按钮触发批量 apply 缝；apply 缝保留给组提交/密钥动作
// ---------------------------------------------------------------------------
{
  const tags = buttonTags(settingsSrc);
  assert.ok(tags.length > 20, `按钮扫描器应能扫到设置页按钮（实得 ${tags.length}）`);
  for (const tag of tags) {
    assert.ok(!tag.includes('handleSaveSettings'), `「保存」按钮残留：${tag.slice(0, 80)}…`);
    assert.ok(!tag.includes('保存设置'), `「保存设置」按钮残留：${tag.slice(0, 80)}…`);
  }
  assert.ok(settingsSrc.includes('async function handleSaveSettings'), '批量 apply 缝保留（组提交/密钥动作复用）');
  assert.ok(
    /async function submitProviderGroup[\s\S]*?await handleSaveSettings\(\);/.test(settingsSrc),
    '服务商组失焦/回车提交复用同一条 apply 缝',
  );
  assert.ok(
    settingsSrc.includes('runLiveApply(') && settingsSrc.includes('void submitProviderGroup()'),
    '提交都走即效外壳（pending 行内态，不弹模态）',
  );

  const themeTags = buttonTags(themePanelSrc);
  for (const tag of themeTags) {
    assert.ok(!tag.includes('保存设置') && !tag.includes('handleSaveSettings'), `主题面板不许有保存按钮：${tag.slice(0, 80)}…`);
  }
  console.log('  -> PASS: 任何按钮都不再触发批量保存；apply 缝由组提交复用');
}

// ---------------------------------------------------------------------------
// 4. 动作类按钮保留（防止误删：恢复默认类 / 探测类 / 危险动作类）
// ---------------------------------------------------------------------------
{
  const REQUIRED_ACTIONS = [
    ['测试连接并拉取模型', 'handleTestProviderConnection'],
    ['深度诊断', 'checkDiagnostics'],
    ['应用推荐配置', 'applyRecommendedPreset'],
    ['恢复基线', "applyDispositionPreset('恢复基线')"],
    ['清空本地会话数据', "requestDanger('clearLocalData')"],
    ['学习日志刷新', 'refreshTuningLog()'],
    ['工作区选择器', 'openWorkspacePicker()'],
  ];
  for (const [label, needle] of REQUIRED_ACTIONS) {
    assert.ok(settingsSrc.includes(needle), `动作类按钮「${label}」必须保留（${needle}）`);
  }
  for (const tag of buttonTags(settingsSrc)) {
    assert.ok(!tag.includes('handleTestProviderConnection') || tag.includes('primary-button') || tag.includes('quiet-button'), '动作按钮样式不被误删');
  }
  console.log('  -> PASS: 动作类按钮（恢复默认/探测/危险动作）全部保留');
}

console.log('--- All Save-Button Removal (source mirror) Checks PASSED! ---');
