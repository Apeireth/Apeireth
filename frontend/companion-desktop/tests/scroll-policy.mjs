// 对话流滚动策略纯逻辑单测（真实模块 src/lib/chat-shell/scroll-policy.ts）。
// 病灶（内测实测）：点开会话停在最上面——应自动滚到底部（最新消息 + 输入框 =
// 继续工作的位置）；新开/切换会话都滚底，用户上翻后绝不强行拽回。
import assert from 'node:assert/strict';

import {
  NEAR_BOTTOM_PX,
  SHOW_JUMP_PX,
  isNearBottom,
  scrollOnAppend,
  scrollOnOpen,
  showJumpButton,
} from '../src/lib/chat-shell/scroll-policy.ts';

// ---- 打开/切换会话：一律滚到底部 ----
assert.equal(scrollOnOpen('new'), 'bottom', '新开会话落到底部（空会话也要落在输入框一侧）');
assert.equal(scrollOnOpen('switch'), 'bottom', '切换会话落到底部（最新消息 = 第一眼）');

// ---- 上翻不拽回：贴底才跟随 ----
assert.equal(scrollOnAppend(true), 'follow', '用户贴底时流式追加跟随');
assert.equal(scrollOnAppend(false), 'hold', '用户已上翻 → 原地不动，不强行拽回');

// ---- 距底口径（与 handleScroll 同一常量，抽出即测）----
assert.equal(NEAR_BOTTOM_PX, 80);
assert.equal(SHOW_JUMP_PX, 150);
assert.equal(isNearBottom(0), true);
assert.equal(isNearBottom(NEAR_BOTTOM_PX - 1), true, '贴底阈值内 = 贴底');
assert.equal(isNearBottom(NEAR_BOTTOM_PX), false, '阈值本身不算贴底（宁可不跟随）');
assert.equal(showJumpButton(SHOW_JUMP_PX), false);
assert.equal(showJumpButton(SHOW_JUMP_PX + 1), true, '离得够远才亮「回到底部」浮钮');

console.log('scroll policy: all assertions passed');
