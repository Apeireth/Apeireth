// 快捷助手窗口生命周期一致性（P0 修复：关闭残留幽灵窗）。
// 对 QuickWindowView（窗内 ×）与 src-tauri toggle_quick_window（托盘）做
// 源码级镜像校验；决策表本身由 src-tauri 的 quick_window_action 单测覆盖。
//
// 契约：
//   1. 窗内 × = 真正关闭（销毁窗）而不是 hide——hide 透明窗会残留一层点不掉
//      的透明残影（幽灵窗）。
//   2. 托盘「快捷助手」共用同一决策表：可见 = 关闭（销毁）、存在不可见 =
//      唤起、不存在 = 新开；关闭后不存在隐藏中的窗口，两条路径状态一致。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const srcDir = join(testsDir, '..', 'src');
const rustLib = join(testsDir, '..', 'src-tauri', 'src', 'lib.rs');

console.log('--- Starting Quick Window Close/Destroy Lifecycle Check ---');

// ---------------------------------------------------------------------------
// 1. 窗内 ×：close（销毁），不是 hide
// ---------------------------------------------------------------------------
{
  const quickSrc = readFileSync(join(srcDir, 'features/quick/QuickWindowView.svelte'), 'utf8');
  const closeRegion = /async function handleClose[\s\S]*?\n  \}/.exec(quickSrc);
  assert.ok(closeRegion, 'QuickWindowView 必须有 handleClose');
  assert.ok(
    closeRegion[0].includes('getCurrentWebviewWindow().close()'),
    '× 必须真正关闭（销毁窗）',
  );
  assert.ok(!closeRegion[0].includes('.hide()'), '× 不允许 hide（会残留透明幽灵窗）');
  console.log('  -> PASS: × = 真正关闭（销毁窗），无 hide 残影路径');
}

// ---------------------------------------------------------------------------
// 2. 托盘切换：close（销毁）/ 唤起 / 新开，绝不 hide
// ---------------------------------------------------------------------------
{
  const rustSrc = readFileSync(rustLib, 'utf8');

  // 决策表镜像（与 src-tauri quick_window_toggle_decision_table 单测同表）。
  for (const arm of [
    '(true, true) => QuickWindowAction::Close',
    '(true, false) => QuickWindowAction::Show',
    '(false, _) => QuickWindowAction::Create',
  ]) {
    assert.ok(rustSrc.includes(arm), `quick_window_action 决策表必须含 ${arm}`);
  }

  const toggleRegion = /fn toggle_quick_window[\s\S]*?\n\}/.exec(rustSrc);
  assert.ok(toggleRegion, 'src-tauri 必须有 toggle_quick_window');
  assert.ok(
    toggleRegion[0].includes('let _ = window.close();'),
    '托盘关闭快捷窗必须销毁窗口',
  );
  assert.ok(
    toggleRegion[0].includes('build_quick_window(&app, true)'),
    '窗口不存在时必须新开（托盘再点 = 新开/唤起）',
  );
  assert.ok(
    !toggleRegion[0].includes('window.hide()'),
    '快捷窗生命周期不允许 hide（隐藏透明窗会残留幽灵窗）',
  );
  console.log('  -> PASS: 托盘路径 = 销毁关闭 / 唤起 / 新开，无隐藏残留态');
}

console.log('--- All Quick Window Close/Destroy Lifecycle Checks PASSED! ---');
