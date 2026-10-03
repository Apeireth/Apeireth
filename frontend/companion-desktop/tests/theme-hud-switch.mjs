// 星舰主题档（科幻 HUD 档）切换接线 + 持久化恢复 + 默认零回归单测
// —— import 真实实现 ../src/lib/theme.ts 与 ../src/lib/runtime.ts。
//
// 锁定三条纪律：
//   ① 即点即效：applyDocumentTheme 同步改写 html[data-theme]；设置面板 pick()
//      先落 DOM 再走 onSave apply 缝（不等任何保存按钮）；
//   ② 持久化恢复：config.theme 经 localStorage 往返后可直接还原主题档；
//   ③ 默认零回归：新增档位不改既有主题行为；默认位 = origin（2026-10-03
//      主人拍板，取代旧 heritage-void 默认位锁定）
//      （night 仍移除 data-theme、静态背景判定不变）。
// localStorage / document 内存 shim 先于任何真实调用就位（模块导入无副作用）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const store = new Map();
globalThis.localStorage = {
  getItem: (k) => (store.has(k) ? store.get(k) : null),
  setItem: (k, v) => void store.set(k, String(v)),
  removeItem: (k) => void store.delete(k),
  clear: () => store.clear(),
};

const attrs = new Map();
globalThis.document = {
  documentElement: {
    setAttribute: (k, v) => void attrs.set(k, String(v)),
    removeAttribute: (k) => void attrs.delete(k),
    getAttribute: (k) => (attrs.has(k) ? attrs.get(k) : null),
  },
};

const theme = await import('../src/lib/theme.ts');
const {loadConfig, saveConfig} = await import('../src/lib/runtime.ts');
const testsDir = dirname(fileURLToPath(import.meta.url));

console.log('--- Starting Theme HUD-Tier Switch Check ---');

// ---------------------------------------------------------------------------
// 1. 切换即效接线：apply 缝同步写 DOM；面板 pick() 先落 DOM 再走 onSave
// ---------------------------------------------------------------------------
{
  theme.applyDocumentTheme('starship');
  assert.equal(attrs.get('data-theme'), 'starship', 'applyDocumentTheme 必须同步写 html[data-theme]（即点即效）');

  theme.applyDocumentTheme('heritage-void');
  assert.equal(attrs.get('data-theme'), 'heritage-void', '切回既有主题同样即时换档');

  const panelSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'components', 'ThemeSettingsPanel.svelte'), 'utf8');
  const pickBody = /function pick\(theme: Theme\)[\s\S]*?\n  \}/.exec(panelSrc);
  assert.ok(pickBody, '设置面板仍有 pick() 即效入口');
  const applyIdx = pickBody[0].indexOf('applyDocumentTheme(theme)');
  const saveIdx = pickBody[0].indexOf('onSave({...config, theme})');
  assert.ok(applyIdx >= 0, 'pick() 必须调用 applyDocumentTheme(theme)');
  assert.ok(saveIdx > applyIdx, '先落 DOM 即效，再走 onSave apply 缝（顺序不许倒）');
  assert.ok(panelSrc.includes('THEME_CATALOG as item'), '主题选择器遍历目录——新档自动进「外观」区，无特例分支');
  const settingsSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'views', 'SettingsView.svelte'), 'utf8');
  assert.ok(settingsSrc.includes('ThemeSettingsPanel'), '主题选择器仍挂在设置页「外观」区');
  console.log('  -> PASS: 切换即效接线（apply 缝 + 设置面板 pick 顺序）');
}

// ---------------------------------------------------------------------------
// 2. 持久化恢复：theme 往返 localStorage 后可直接还原主题档
// ---------------------------------------------------------------------------
{
  const cfg = loadConfig();
  cfg.theme = 'starship';
  saveConfig(cfg);

  const raw = JSON.parse(store.get('apeireth-config'));
  assert.equal(raw.theme, 'starship', 'persistedConfig 必须携带 theme（白名单纪律）');

  const back = loadConfig();
  assert.equal(back.theme, 'starship', 'theme 必须活着读回');
  const restored = theme.resolveTheme(back.theme);
  assert.equal(restored, 'starship');
  theme.applyDocumentTheme(restored);
  assert.equal(attrs.get('data-theme'), 'starship', '重启后按持久化值还原主题档');

  // 脏数据守卫：非法持久化值回落默认，不半路换档
  // @ts-expect-error 故意传非法值
  assert.equal(theme.resolveTheme('bogus', null), 'origin', '非法持久化主题回落默认档');
  console.log('  -> PASS: 持久化恢复（localStorage 往返 + 脏值回落）');
}

// ---------------------------------------------------------------------------
// 3. 默认零回归：默认回落 / 目录首项 / 既有主题行为一律不动
//    （2026-10-03 主人拍板：origin 原初取默认位——推翻"新档不许插队"对默认
//    位的旧锁定；其后新增档位仍不得插队，既有主题行为一律不动。）
// ---------------------------------------------------------------------------
{
  assert.equal(theme.resolveTheme(undefined, null), 'origin', '默认主题 = origin（2026-10-03 拍板）');
  assert.equal(theme.resolveTheme(undefined, 'starship'), 'starship', '?theme= 覆写纪律对新档同样成立');
  assert.equal(theme.THEME_CATALOG[0].id, 'origin', '目录首项 = 默认主题 origin');
  assert.equal(theme.VALID_THEMES[0], 'origin', '合法主题表首项 = 默认主题');
  assert.equal(theme.isStaticBgTheme('night'), false, '既有实时场景主题判定不变');
  assert.equal(theme.isStaticBgTheme('heritage-void'), true);
  // 新档自身完整性
  assert.equal(theme.VALID_THEMES.includes('starship'), true);
  assert.equal(theme.isStaticBgTheme('starship'), true, '科幻 HUD 档是静态背景（无实时场景层）');
  const entry = theme.THEME_CATALOG.find((t) => t.id === 'starship');
  assert.ok(entry, '新档必须有 THEME_CATALOG 条目');
  assert.ok(entry.label && entry.desc && entry.swatch, '新档条目字段不全');
  // night 专属行为保留：移除 data-theme 属性（既有语义零改动）
  theme.applyDocumentTheme('night');
  assert.equal(attrs.has('data-theme'), false, 'night 的 data-theme 移除行为不变');
  console.log('  -> PASS: 默认零回归（回落 / 目录首项 / 既有主题行为）');
}

console.log('--- All Theme HUD-Tier Switch Checks PASSED! ---');
