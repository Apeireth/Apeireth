// tests/pet-default-off.mjs — 桌宠默认禁用 (pet default off), source mirror.
//
// 口径 (2026-10-17): 运行时右下角的桌宠形象窗默认不出现。唯一的自动触发是
// 主窗收起时的 apply_pet_leeway(Show), 这道闸受 PET_ENABLED (默认 false) 与
// 持久化 pet-prefs.json (auto_takeover, serde default=false) 控制; 设置面板
// (AvatarSettingsPanel › 桌宠) 提供总开关, 桥接层 getPetEnabled/setPetEnabled
// 透传; 托盘「桌宠」仍是一次性手动召唤, 不翻偏好。
//
// 纪律同 pet-window-lifecycle.mjs: 真实源文件当作文本读 (source mirror),
// 断言全部是源码事实 —— 不运行 Rust。

import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const read = (rel) => readFile(path.join(here, '..', rel), 'utf8');

const libRs = await read('src-tauri/src/lib.rs');
const prefsRs = await read('src-tauri/src/pet_prefs.rs');
const bridgeTs = await read('src/lib/tauri-bridge.ts');
const panelSvelte = await read('src/lib/components/AvatarSettingsPanel.svelte');

test('PET_ENABLED defaults to false (auto takeover off out of the box)', () => {
  const m = libRs.match(/static PET_ENABLED:[^;]+;/);
  assert.ok(m, 'PET_ENABLED static must exist');
  assert.match(m[0], /AtomicBool::new\(false\)/, 'default must be false');
});

test('startup only prebuilds the pet window when the saved pref is on', () => {
  const gate = /let pet_prefs_value = pet_prefs::load\([\s\S]{0,600}?if pet_prefs_value\.auto_takeover \{\s*\n\s*let _ = build_pet_window\(app\.handle\(\), false\);/.test(libRs);
  assert.ok(gate, 'setup must prebuild only inside if pet_prefs_value.auto_takeover');
  assert.ok(!libRs.includes('桌宠常驻窗 (启动预建、初始隐藏'), 'old unconditional prebuild comment must be gone');
});

test('apply_pet_leeway Show arm returns early while pet stays disabled', () => {
  const arm = /PetLeeway::Show => \{([\s\S]{0,500}?)if let Some\(pet\) = app\.get_webview_window\("pet"\)/.exec(libRs);
  assert.ok(arm, 'Show arm not found');
  assert.match(arm[1], /if !PET_ENABLED\.load\(std::sync::atomic::Ordering::Relaxed\) \{\s*\n\s*return;\s*\n\s*\}/, 'gate must return before touching the pet window');
});

test('pet prefs file is the persistence envelope with disabled default', () => {
  assert.match(prefsRs, /pub const PET_PREFS_FILE: &str = "pet-prefs\.json";/);
  assert.match(prefsRs, /pub auto_takeover: bool/);
  assert.match(prefsRs, /#\[serde\(default\)\]/, 'missing field must deserialize to default');
  // 默认关闭的双保险: serde 缺字段走 Default, 而 Default 由 derive 生成 (bool → false),
  // 不手写 impl (clippy::derivable_impls; 2026-10-20 评审修正)。
  assert.match(prefsRs, /#\[derive\([^)]*\bDefault\b[^)]*\)\]\s*\n\s*pub struct PetPrefs/, 'Default must be derived (bool → false)');
  assert.match(prefsRs, /Ok\(None\) => PetPrefs::default\(\)/, 'missing file reads as disabled');
  assert.match(prefsRs, /DocCompat::exact\(PET_PREFS_DOC_NAME, PET_PREFS_DOC_VERSION\)/, 'envelope compat');
  assert.ok(prefsRs.includes('corrupt_file_defaults_to_disabled_without_rewrite'), 'corrupt-file test must be kept');
});

test('set_pet_enabled persists first, then builds or tears down the window', () => {
  const fn = /fn set_pet_enabled\([\s\S]*?\n\}/.exec(libRs);
  assert.ok(fn, 'set_pet_enabled command missing');
  const body = fn[0];
  const save = body.indexOf('pet_prefs::save(');
  const build = body.indexOf('build_pet_window(&app, false)');
  const hide = body.indexOf('window.hide()');
  const destroy = body.indexOf('window.destroy()');
  assert.ok(save >= 0, 'must persist');
  assert.ok(save < build, 'persist before rebuild');
  assert.ok(hide >= 0 && hide < destroy, 'hide before destroy (no ghost window)');
  assert.ok(build >= 0 && destroy >= 0, 'both branches present');
  const handlers = /generate_handler!\[([\s\S]*?)\]/.exec(libRs);
  assert.ok(handlers, 'generate_handler list missing');
  assert.match(handlers[1], /\bget_pet_enabled\b/);
  assert.match(handlers[1], /\bset_pet_enabled\b/);
});

test('settings panel exposes the pet master switch (was tray-only)', () => {
  assert.match(panelSvelte, /name="pet-auto-takeover"/, 'checkbox must exist');
  assert.match(panelSvelte, /setPetTakeover\(\(event\.currentTarget as HTMLInputElement\)\.checked\)/, 'onchange wiring');
  assert.match(panelSvelte, /petEnabled = await getPetEnabled\(\)/, 'onMount seeds state');
  assert.match(bridgeTs, /export async function getPetEnabled\(\): Promise<boolean>/);
  assert.match(bridgeTs, /export async function setPetEnabled\(enabled: boolean\): Promise<boolean>/);
  assert.match(bridgeTs, /invokeOptional<boolean>\('get_pet_enabled'\)/);
  assert.match(bridgeTs, /invokeOptional<boolean>\('set_pet_enabled', \{enabled\}\)/);
});

test('tray manual summon stays one-shot (does not flip the preference)', () => {
  const toggle = /fn toggle_pet_window\([\s\S]*?\n\}/.exec(libRs);
  assert.ok(toggle, 'toggle_pet_window missing');
  assert.ok(!toggle[0].includes('PET_ENABLED'), 'tray toggle must not flip PET_ENABLED');
  assert.ok(!toggle[0].includes('pet_prefs::save'), 'tray toggle must not persist');
});
