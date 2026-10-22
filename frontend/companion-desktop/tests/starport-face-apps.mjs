// 星港面装配集（face-apps.ts / faces-install.ts）——源镜像接线事实钉子。
//
// 仿 shell-ia.mjs 成例（源镜像不 import 运行时，只钉接线事实）：
//   1. 深望：准入闸（normalizeBrowseUrl + checkBrowsePolicy）先于 iframe 放行；
//   2. 通译：走真实模型通道（streamChat）且失败如实外显；
//   3. 六面工厂齐全、mount 返 cleanup、安装器统一下挂 9 面。
// 行为证据在各面纯件测试（browse/chart/organize/lifeflow/notes/text/doc）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

console.log('--- Starting Starport Face Apps Source Mirror ---');

const here = dirname(fileURLToPath(import.meta.url));
const faceApps = readFileSync(join(here, '../src/lib/starport/apps/face-apps.ts'), 'utf8');
const installer = readFileSync(join(here, '../src/lib/starport/faces-install.ts'), 'utf8');
const terminalApp = readFileSync(join(here, '../src/lib/starport/apps/terminal-app.ts'), 'utf8');
const portableApp = readFileSync(join(here, '../src/lib/starport/apps/portable-app.ts'), 'utf8');
const memoryFlowApp = readFileSync(join(here, '../src/lib/starport/apps/memory-flow-app.ts'), 'utf8');
const dbApp = readFileSync(join(here, '../src/lib/starport/apps/db-app.ts'), 'utf8');
const folderTilesApp = readFileSync(join(here, '../src/lib/starport/apps/folder-tiles-app.ts'), 'utf8');
const voiceApp = readFileSync(join(here, '../src/lib/starport/apps/voice-app.ts'), 'utf8');
const docApp = readFileSync(join(here, '../src/lib/starport/apps/doc-app.ts'), 'utf8');
const mailApp = readFileSync(join(here, '../src/lib/starport/apps/mail-app.ts'), 'utf8');

// 1. 深望准入闸先于放行（顺序事实：gate() 定义先于 frame.src 赋值）。
{
  const gateDef = faceApps.indexOf('function gate()');
  const frameSrc = faceApps.indexOf('frame.src = target.href');
  assert.ok(gateDef > 0 && frameSrc > gateDef, '准入闸定义先于 iframe 放行');
  assert.ok(faceApps.includes('normalizeBrowseUrl'), '协议白名单在装配层强制走');
  assert.ok(faceApps.includes('checkBrowsePolicy'), '站点策略在装配层强制走');
  assert.ok(
    faceApps.includes(`frame.setAttribute('sandbox'`),
    '内嵌视图带沙箱属性',
  );
  assert.ok(faceApps.includes('extractSemanticTree('), '深望语义树接线（真源留存后提取）');
  console.log('  [1] browse gate wiring OK');
}

// 2. 通译走真实模型通道 + 失败如实外显。
{
  assert.ok(faceApps.includes('streamChat('), '翻译走真实模型通道');
  assert.ok(faceApps.includes('翻译失败（如实报）'), '翻译失败如实外显');
  console.log('  [2] translate wiring OK');
}

// 3. 六面工厂齐全 + mount 返 cleanup + 安装器统一下挂。
{
  for (const factory of [
    'createBrowseApp',
    'createChartApp',
    'createOrganizeApp',
    'createLifeflowApp',
    'createTranslateApp',
    'createEcosystemApp',
  ]) {
    assert.ok(faceApps.includes(`export function ${factory}(`), `缺工厂：${factory}`);
  }
  assert.equal(
    (faceApps.match(/return \(\) => \{/g) ?? []).length >= 6,
    true,
    '每面 mount 返回 cleanup',
  );
  for (const face of ['createNotesApp', 'createTextEditorApp', 'createDocApp']) {
    assert.ok(installer.includes(face), `安装器缺面：${face}`);
  }
  assert.ok(installer.includes('registry.registerApp'), '安装面统一下挂注册表');
  console.log('  [3] factory + installer wiring OK');
}

// 4. 原生面与记忆面接线事实（真件调用 + 桌面壳守卫 + disposed 清理）。
{
  assert.ok(terminalApp.includes('run_workspace_command'), '终端走真实执行命令');
  assert.ok(terminalApp.includes('truncated'), '截断如实外显');
  assert.ok(terminalApp.includes('智能体驱动走审批族'), '审批族接线缝明示');
  // 权限边界明示（安全隐患分析 #2：不许让用户以为受审批链/沙箱保护）。
  assert.ok(terminalApp.includes('以当前 Windows 用户权限执行命令'), '权限边界明示：当前用户权限');
  assert.ok(terminalApp.includes('不受智能体审批链保护'), '权限边界明示：不走审批链');
  assert.ok(terminalApp.includes('不是操作系统沙箱'), '权限边界明示：非沙箱');
  assert.ok(terminalApp.includes('关闭的是智能体工具面'), '权限边界明示：enable_shell 只关智能体工具面');
  assert.ok(portableApp.includes('get_portable_mode'), '便携走真实判定命令');
  assert.ok(memoryFlowApp.includes('appendMemoryEpisode('), '深流沉淀走真实记忆通道');
  assert.ok(memoryFlowApp.includes('fetchMemoryEpisodes('), '深流召回走真实记忆通道');
  assert.ok(memoryFlowApp.includes('disposed'), '深流清理链含 disposed 丢弃晚到结果');
  for (const face of ['createVoiceApp', 'createTerminalApp', 'createPortableApp', 'createMemoryFlowApp', 'createMailApp']) {
    assert.ok(installer.includes(face), `安装器缺面：${face}`);
  }
  console.log('  [4] native/memory wiring OK');
}

// 5. 矿脉接线事实（真引擎调用 + 前端只读口径先行）。
{
  assert.ok(dbApp.includes('query_sqlite_file'), '矿脉走真实查询命令');
  assert.ok(dbApp.includes('query_postgres'), '矿脉 PG 模式走真实协议驱动命令');
  assert.ok(dbApp.includes('query_mysql'), '矿脉 MySQL 模式走真实协议驱动命令');
  assert.ok(dbApp.includes('classifySql('), '前端只读口径先行（纵深第一道）');
  assert.ok(dbApp.includes('toCsv('), 'CSV 导出走纯件转义');
  assert.ok(installer.includes('createDbApp'), '安装器缺面：createDbApp');
  console.log('  [5] db wiring OK');
}

// 6. 磁贴与合成接线事实（真列举命令 + 真合成引擎 + 真停清理）。
{
  assert.ok(folderTilesApp.includes('list_workspace_dir'), '磁贴走真实目录列举命令');
  assert.ok(folderTilesApp.includes('disposed'), '磁贴清理链丢弃晚到结果');
  assert.ok(folderTilesApp.includes('FOLDER_TILES_STORAGE_KEY'), '磁贴持久化走真落存适配器');
  assert.ok(folderTilesApp.includes('corrupt-persistence'), '坏载荷如实报不静默清库');
  assert.ok(voiceApp.includes('speechSynthesis.speak('), '合成走系统语音引擎真件');
  assert.ok(voiceApp.includes('speechSynthesis.cancel()'), '清理链真停合成');
  assert.ok(voiceApp.includes('recognize_speech'), '转写走真实识别引擎命令');
  assert.ok(voiceApp.includes('detect_speech_engine'), '引擎探测真件先行');
  assert.ok(installer.includes('createFolderTilesApp'), '安装器缺面：createFolderTilesApp');
  assert.ok(docApp.includes('mapRenderedEdit('), '星笺渲染态直改走真事务映射');
  assert.ok(docApp.includes('IslandRegistry'), '星笺岛生命周期账本接线');
  assert.ok(docApp.includes('allow-scripts'), '岛沙箱仅开脚本不开同源');
  assert.ok(folderTilesApp.includes('create_dir_entry'), '磁贴写盘建目录真件');
  assert.ok(folderTilesApp.includes('rename_entry'), '磁贴写盘改名真件');
  assert.ok(folderTilesApp.includes('delete_entry'), '磁贴写盘删条目真件');
  assert.ok(mailApp.includes('send_smtp'), '信使发送走真实 SMTP 命令');
  assert.ok(mailApp.includes('imap_fetch'), '信使收信走真实 IMAP 命令');
  assert.ok(mailApp.includes('toEml('), '信使发送经 .eml 进出闸');
  assert.ok(mailApp.includes('不设假发送'), '非桌面壳不设假发送');
  console.log('  [6] tiles/speech wiring OK');
}

console.log('--- Starport Face Apps Source Mirror PASSED ---');
