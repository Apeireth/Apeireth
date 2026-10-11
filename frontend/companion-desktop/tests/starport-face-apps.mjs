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

console.log('--- Starport Face Apps Source Mirror PASSED ---');
