// 瞬时落底纯逻辑（打开/切换会话的首帧定位）。
//
// 病灶（内测实测）：打开/切换会话时消息区可见地"从顶部快速滚到底"——滚动
// 容器上无条件的平滑滚动声明把任何 scrollTop 赋值动画成一串中间滚动帧，再
// 叠加渲染后异步定位的延迟感。修法：落底必须同步赋值、一次到位，函数返回
// 时容器已在底部，首帧即贴底（无中间滚动帧）。
//
// 职责边界：本模块只管"怎么落"（同步、一次赋值）；"何时落/落不落"的判定
// 口径一字不动地留在 scroll-policy.ts（NEAR_BOTTOM_PX=80 / SHOW_JUMP_PX=150 /
// scrollOnOpen→'bottom' / scrollOnAppend 贴底 follow、上翻 hold）。

/** 落底所需的最小容器度量（滚动容器元素的结构子集，便于纯函数单测）。 */
export interface ScrollMetrics {
  scrollTop: number;
  scrollHeight: number;
  clientHeight: number;
}

/** 距底距离（px）：内容不足一屏时为负，视作已到底。 */
export function distanceToBottom(container: ScrollMetrics): number {
  return container.scrollHeight - container.scrollTop - container.clientHeight;
}

/**
 * 同步落底：一次赋值把容器钉在底部（scrollTop = scrollHeight - clientHeight，
 * 内容不足一屏时钉 0，不产生负值）。无 await、无延时、无中间滚动帧——函数
 * 返回前赋值已完成，返回落底后的 scrollTop 值。
 */
export function landAtBottom(container: ScrollMetrics): number {
  const target = Math.max(0, container.scrollHeight - container.clientHeight);
  container.scrollTop = target;
  return target;
}

/**
 * 首帧后复核：上次落底的赋值仍是当前 scrollTop（用户没动过滚动位置）且内容
 * 又长高了（距底 > 0）→ 需要再钉一次底。用户已上翻（scrollTop 被改动）则
 * 一律 false——复核绝不把人拽回底部（上翻不拽回）。
 */
export function needsReland(container: ScrollMetrics, landedTop: number): boolean {
  return container.scrollTop === landedTop && distanceToBottom(container) > 0;
}
