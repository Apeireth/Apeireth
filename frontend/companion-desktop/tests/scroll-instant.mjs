// 对话流瞬时落底（真实模块 src/lib/chat-shell/scroll-landing.ts + scroll-policy.ts）。
// 病灶（内测实测）：打开/切换会话时消息区可见地"从顶部快速滚到底"——任何
// scrollTop 赋值被滚动容器的平滑声明动画成中间滚动帧，再叠加渲染后异步定位
// 的延迟感。契约：
//   1. 瞬时落底：landAtBottom 同步一次赋值到位，函数返回前赋值已完成；落底后
//      第一帧 scrollTop 即 scrollHeight - clientHeight，绝不出现中间滚动帧。
//   2. 复核不拽回：needsReland 只在"用户没动过滚动 + 内容长高"时再钉一次；
//      用户上翻过（scrollTop 被改动）一律不动。
//   3. 上翻不拽回零回归：scrollOnAppend(false)==='hold' / (true)==='follow' /
//      scrollOnOpen 恒 'bottom'（口径一字不改，仍由 scroll-policy.ts 判定）。
//   4. source-mirror：App.svelte 打开/切换落底不再用渲染后异步定位；shell.css
//      的 .scroll 不再无条件平滑；「回到底部」按钮的平滑跳转能力保留。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {distanceToBottom, landAtBottom, needsReland} from '../src/lib/chat-shell/scroll-landing.ts';
import {
  NEAR_BOTTOM_PX,
  SHOW_JUMP_PX,
  scrollOnAppend,
  scrollOnOpen,
} from '../src/lib/chat-shell/scroll-policy.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const appSrc = readFileSync(join(testsDir, '..', 'src', 'App.svelte'), 'utf8');
const shellCss = readFileSync(join(testsDir, '..', 'src', 'lib', 'design', 'shell.css'), 'utf8');

// 假滚动容器：记录 scrollTop 的每次赋值（赋值序列）与每"帧"渲染结束时的
// scrollTop 采样（帧序列）——中间滚动帧在这两个序列里无处遁形。
function fakeContainer({scrollHeight, clientHeight, scrollTop = 0}) {
  const assignments = [];
  const frames = [];
  let top = scrollTop;
  const box = {
    scrollHeight,
    clientHeight,
    get scrollTop() {
      return top;
    },
    set scrollTop(value) {
      assignments.push(value);
      top = value;
    },
  };
  return {
    box,
    assignments,
    frames,
    endFrame() {
      frames.push(top);
    },
  };
}

console.log('--- chat scroll instant landing ---');

// ---------------------------------------------------------------------------
// 1. 瞬时落底：一次赋值、首帧即贴底、无中间滚动帧、返回前赋值已完成
// ---------------------------------------------------------------------------
{
  const {box, assignments, frames} = fakeContainer({scrollHeight: 5000, clientHeight: 800, scrollTop: 0});
  const bottom = 5000 - 800;

  frames.push(box.scrollTop); // 落底前的一帧：停在顶部
  const landed = landAtBottom(box);
  frames.push(box.scrollTop); // 落底动作后的第一帧

  // 同步语义：函数返回前赋值已完成（无 await、无延时队列）。
  assert.equal(landed, bottom, '返回值 = 落底后的 scrollTop');
  assert.equal(box.scrollTop, bottom, '函数返回时容器已在底部（赋值同步完成）');
  assert.deepEqual(assignments, [bottom], '一次赋值直到底部——不产生任何中间 scrollTop 赋值');

  // 帧序列：落底动作后第一帧即底部，中间滚动帧不存在。
  assert.equal(frames[0], 0, '落底前帧在顶部（复现病灶起点）');
  assert.equal(frames[1], bottom, '落底动作后第一帧的 scrollTop 即 scrollHeight - clientHeight');
  for (const frame of frames.slice(1)) {
    assert.ok(
      frame === bottom,
      `不允许中间滚动帧（0 < 距底 < 全高）：帧内 scrollTop=${frame} 应直接=${bottom}`,
    );
  }
  console.log('  -> PASS: 瞬时落底一次赋值、首帧即贴底（无中间滚动帧、同步返回）');
}

// ---------------------------------------------------------------------------
// 2. 内容不足一屏 / 已在底部 / 幂等：不产生负值，重复调用仍是底部
// ---------------------------------------------------------------------------
{
  const short = fakeContainer({scrollHeight: 500, clientHeight: 800, scrollTop: 120});
  assert.equal(landAtBottom(short.box), 0, '内容不足一屏 → 钉 0（不产生负 scrollTop）');
  assert.equal(short.box.scrollTop, 0);

  const tall = fakeContainer({scrollHeight: 3000, clientHeight: 600, scrollTop: 2400});
  assert.equal(landAtBottom(tall.box), 2400, '已在底部 → 幂等');
  assert.equal(tall.assignments.length, 1);
  assert.equal(distanceToBottom(tall.box), 0, '落底后距底 = 0');
  console.log('  -> PASS: 短内容钉 0 / 落底幂等 / 距底归零');
}

// ---------------------------------------------------------------------------
// 3. 首帧后复核：内容长高再钉一次；用户上翻过一律不动（不拽回）
// ---------------------------------------------------------------------------
{
  // 内容长高（图片/公式/长文在首帧后撑开）：scrollTop 没被动过 → 再钉一次。
  const grown = fakeContainer({scrollHeight: 5000, clientHeight: 800, scrollTop: 0});
  const landedTop = landAtBottom(grown.box);
  grown.box.scrollHeight = 6000; // 首帧后长高
  assert.equal(needsReland(grown.box, landedTop), true, '内容长高且用户没动 → 需要复核落底');
  assert.equal(landAtBottom(grown.box), 6000 - 800, '复核再钉一次仍贴底');

  // 用户上翻（scrollTop 被改动）→ 复核绝不拽回。
  const scrolled = fakeContainer({scrollHeight: 6000, clientHeight: 800, scrollTop: 0});
  const top2 = landAtBottom(scrolled.box);
  scrolled.box.scrollTop = top2 - 500; // 用户上翻 500px
  scrolled.box.scrollHeight = 6500;
  assert.equal(needsReland(scrolled.box, top2), false, '用户上翻过 → 不复核（上翻不拽回）');
  assert.equal(scrolled.box.scrollTop, top2 - 500, '滚动位置保持在用户停下的地方');

  // 没长高 → 不打扰。
  const steady = fakeContainer({scrollHeight: 5000, clientHeight: 800, scrollTop: 0});
  const top3 = landAtBottom(steady.box);
  assert.equal(needsReland(steady.box, top3), false, '内容没长高 → 不做多余赋值');
  console.log('  -> PASS: 首帧后复核只补长高、用户上翻不拽回');
}

// ---------------------------------------------------------------------------
// 4. 上翻不拽回零回归（真实模块 scroll-policy.ts，口径一字不改）
// ---------------------------------------------------------------------------
{
  assert.equal(scrollOnAppend(false), 'hold', '用户已上翻 → 原地不动，不强行拽回');
  assert.equal(scrollOnAppend(true), 'follow', '用户贴底时流式追加跟随');
  assert.equal(scrollOnOpen('new'), 'bottom', '新开会话恒落底');
  assert.equal(scrollOnOpen('switch'), 'bottom', '切换会话恒落底');
  assert.equal(NEAR_BOTTOM_PX, 80, '贴底口径 80px 不变');
  assert.equal(SHOW_JUMP_PX, 150, '浮钮口径 150px 不变');
  console.log('  -> PASS: scrollOnAppend hold/follow + scrollOnOpen 恒 bottom（零回归）');
}

// ---------------------------------------------------------------------------
// 5. source-mirror：落底接线换同步、滚动容器不再无条件平滑、按钮平滑保留
// ---------------------------------------------------------------------------
{
  // App.svelte：打开/切换落底不再出现渲染后异步定位形态。
  assert.ok(
    !appSrc.includes('tick().then(() => scrollToBottom'),
    '打开/切换落底不得再用渲染后异步定位（tick().then 形态已退场）',
  );
  assert.ok(
    /scrollOnOpen\(kind\) !== 'bottom'[\s\S]{0,800}landAtBottom\(container\)/.test(appSrc),
    '打开/切换落底走 scrollOnOpen 判定后同步 landAtBottom',
  );
  assert.ok(
    /requestAnimationFrame\(\(\) => \{[\s\S]{0,400}needsReland\(container, landedTop\)[\s\S]{0,120}landAtBottom\(container\)/.test(appSrc),
    '首帧后复核接线在案（needsReland → 再钉一次）',
  );
  // 显式平滑跳转能力保留（「回到底部」按钮）。
  assert.ok(
    /scrollTo\(\{[\s\S]{0,120}behavior: 'smooth'/.test(appSrc),
    '「回到底部」按钮的显式平滑跳转能力保留',
  );

  // shell.css：.scroll 块不再无条件声明平滑（否则 scrollTop 赋值被动画成中间帧）。
  const blockMatch = shellCss.match(/^\.scroll \{[\s\S]*?\n\}/m);
  assert.ok(blockMatch, '.scroll 样式块在案');
  const scrollBlock = blockMatch[0];
  assert.ok(
    !/scroll-behavior:\s*smooth/.test(scrollBlock),
    '.scroll 不再无条件平滑滚动（瞬时落底的前提）',
  );
  assert.ok(/overflow-y:\s*auto/.test(scrollBlock), '.scroll 滚动容器本体不变');
  console.log('  -> PASS: source-mirror——同步落底接线 + .scroll 不平滑 + 按钮平滑保留');
}

console.log('scroll instant landing: all assertions passed');
