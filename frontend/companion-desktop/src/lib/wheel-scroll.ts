// 全局滚轮路由（wheel router）——"鼠标滚轮上下移动应用操作界面"的确定性保障。
//
// 病灶（CDP 实测复现，scripts/wheel-scroll-probe.mjs）：
//   滚轮只在命中链正好落在滚动面板上时才生效（内容列中心 ✅）；
//   一旦光标落在"死区"——聊天区留白（#chatScroll 整链 pointer-events:none 穿透）、
//   设置子导航列（非面板的同页旁路）、抽屉头等——滚轮事件到不了任何滚动面板，
//   页面只能拖右侧滚动条。小窗模式下死区占比大，体验尤甚。
//
// 路由规则（确定性、可测）：
//   R1 命中链自底向上第一个滚动面板 → **原生处理**（不干预，内层优先天然正确）；
//   R2 命中元素的【后代】里按几何（矩形含光标）找最深滚动面板 → 路由
//      （pe:none 穿透死区：面板是命中元素的后代）；
//   R4 从命中元素向上，找"子树内唯一可见滚动面板"的祖先 → 路由
//      （设置子导航列/抽屉头 → 整页唯一面板 = 内容列）；
//   无解 → 不干预（如抽屉外遮罩区，滚轮本来就无处可滚）。
// 到界链式：面板滚到底且仍有增量 → 交给下一个外层面板；overscroll-behavior-y:
// contain 的面板到界即停（如 .session-col）。ctrl+滚轮保留浏览器缩放，不拦截。
//
// 纯核心（resolveWheelDelta/pickPaneUnderPoint/pickUniquePane/scrollWithChain）与
// DOM 适配分离，Node 直测（tests/wheel-scroll.mjs）。路由接管时才 preventDefault；
// R1 原生路径零干预。

/** 滚动面板候选（DOM 适配层收集后交给纯核心裁决）。 */
export interface PaneCandidate {
  /** 稳定标识（调试/测试用）。 */
  id: string;
  /** DOM 深度（越深越优先）。 */
  depth: number;
  /** 视口坐标矩形。 */
  rect: {left: number; top: number; right: number; bottom: number};
  /** 当前滚动位置与可滚范围。 */
  scrollTop: number;
  scrollHeight: number;
  clientHeight: number;
  /** overscroll-behavior-y 是否为 contain（到界即停）。 */
  overscrollContain: boolean;
  /** 真实节点（DOM 适配层持有；测试用假件可放占位对象）。 */
  node: unknown;
  /** 后代判定（测试注入；DOM 适配层为 el.contains）。 */
  contains?(other: unknown): boolean;
}

/** DOM 候选集（R2/R4 的几何裁决输入）。 */
export interface PaneIndex {
  panes: PaneCandidate[];
  /** 命中元素深度/祖先链（R4 用：hit 与候选的 contains 关系）。 */
  hit: {id: string; depth: number; node: unknown; contains?(other: unknown): boolean; parentOf?(): HitNode | null};
}

export interface HitNode {
  id: string;
  depth: number;
  node: unknown;
  contains?(other: unknown): boolean;
  parentOf?(): HitNode | null;
}

/** 滚轮增量归一化：deltaMode 1=行（×16px）/ 2=页（×视口高）。 */
export function resolveWheelDelta(deltaY: number, deltaMode: number, viewportHeight: number): number {
  if (deltaMode === 1) return deltaY * 16;
  if (deltaMode === 2) return deltaY * Math.max(1, viewportHeight);
  return deltaY;
}

function paneIsScrollable(p: PaneCandidate): boolean {
  return p.scrollHeight > p.clientHeight + 1;
}

function rectContains(p: PaneCandidate, x: number, y: number): boolean {
  return x >= p.rect.left && x <= p.rect.right && y >= p.rect.top && y <= p.rect.bottom;
}

/** 面板参与路由的可见性判定：**矩形中心**须落在视口内。
 *  离屏/滑出的面板（如窄窗折叠的 .session-col translateX(-105%)，右缘仍可能
 *  压在视口里）不参与 R4 唯一性，否则会把"同页唯一面板"判定打成多面板。 */
export function paneVisibleInViewport(
  rect: {left: number; top: number; right: number; bottom: number},
  viewportWidth: number,
  viewportHeight: number,
): boolean {
  const cx = (rect.left + rect.right) / 2;
  const cy = (rect.top + rect.bottom) / 2;
  return cx >= 0 && cx <= viewportWidth && cy >= 0 && cy <= viewportHeight;
}

function deepest(list: PaneCandidate[]): PaneCandidate {
  return list.reduce((a, b) => (b.depth > a.depth ? b : a));
}

/** R2：命中元素后代里几何含光标、可滚的最深面板。 */
export function pickPaneUnderPoint(index: PaneIndex, x: number, y: number): PaneCandidate | null {
  const under = index.panes.filter(
    (p) => paneIsScrollable(p) && rectContains(p, x, y) && containsNode(index.hit, p.node),
  );
  return under.length > 0 ? deepest(under) : null;
}

/** R4：从命中元素向上，找"子树内唯一可滚面板"的祖先 → 该面板。 */
export function pickUniquePane(index: PaneIndex): PaneCandidate | null {
  const scrollables = index.panes.filter(paneIsScrollable);
  let cursor: HitNode | null = index.hit;
  while (cursor) {
    const inSubtree = scrollables.filter((p) => containsNode(cursor!, p.node));
    if (inSubtree.length === 1) return inSubtree[0];
    cursor = cursor.parentOf ? cursor.parentOf() : null;
  }
  return null;
}

function containsNode(container: {node: unknown; contains?(other: unknown): boolean}, other: unknown): boolean {
  if (container.node === other) return true;
  return typeof container.contains === 'function' ? container.contains(other) : false;
}

/** 路由决策结果。 */
export interface RouteDecision {
  kind: 'native' | 'route';
  pane: PaneCandidate | null;
  rule: 'R1' | 'R2' | 'R4' | 'none';
}

/** 路由裁决：R1 原生（含命中链内面板），R2/R4 接管，none 不干预。 */
export function routeWheel(
  chain: PaneCandidate[],
  index: PaneIndex,
  x: number,
  y: number,
): RouteDecision {
  // R1：命中链（自底向上）已有面板 → 原生滚动，零干预。
  const nativePane = chain.find(paneIsScrollable);
  if (nativePane) return {kind: 'native', pane: nativePane, rule: 'R1'};
  // R2：穿透死区 → 后代几何面板。
  const under = pickPaneUnderPoint(index, x, y);
  if (under) return {kind: 'route', pane: under, rule: 'R2'};
  // R4：同页旁路 → 最近"唯一面板"容器。
  const unique = pickUniquePane(index);
  if (unique) return {kind: 'route', pane: unique, rule: 'R4'};
  return {kind: 'native', pane: null, rule: 'none'};
}

export interface ScrollOutcome {
  /** 实际滚动的总像素（≤ |delta|）。 */
  moved: number;
  /** 因 overscroll-contain 或无外层面板而未消费的剩余量。 */
  remaining: number;
  /** 触达的面板链（调试）。 */
  used: string[];
}

export interface ChainPane extends PaneCandidate {
  /** 外层面板（链式传递）。 */
  outer: ChainPane | null;
}

/**
 * 带链式的滚动执行：面板到界且仍有增量 → 传给 outer（除非 contain 到界即停）。
 * 纯函数式（不改 DOM）：返回每步的落点清单，DOM 适配层按 moved 落实 scrollTop。
 */
export function scrollWithChain(
  start: ChainPane,
  delta: number,
  apply: (pane: ChainPane, by: number) => number,
): ScrollOutcome {
  const used: string[] = [];
  let remaining = delta;
  let pane: ChainPane | null = start;
  while (pane && Math.abs(remaining) > 0.5) {
    const moved = apply(pane, remaining);
    if (Math.abs(moved) > 0.5) {
      used.push(pane.id);
      remaining -= moved;
      continue; // 本层消化了（部分或全部），继续喂同一层吃剩余量
    }
    // 本层到界：contain 即停；否则传外层。
    if (pane.overscrollContain) break;
    pane = pane.outer;
  }
  return {moved: delta - remaining, remaining, used};
}

// ---------------------------------------------------------------------------
// DOM 适配层（浏览器侧；测试不覆盖——纯核心已单测，本层保持薄）。
// ---------------------------------------------------------------------------

interface DomPane extends PaneCandidate {
  el: Element;
  outer: DomPane | null;
}

const CACHE_TTL_MS = 400;
let cached: {panes: DomPane[]; at: number} | null = null;

function domDepth(el: Element): number {
  let d = 0;
  for (let cur: Element | null = el; cur; cur = cur.parentElement) d += 1;
  return d;
}

function collectDomPanes(): DomPane[] {
  const now = Date.now();
  if (cached && now - cached.at < CACHE_TTL_MS) return cached.panes;
  const panes: DomPane[] = [];
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  for (const el of document.querySelectorAll('*')) {
    const cs = getComputedStyle(el);
    if (cs.overflowY !== 'auto' && cs.overflowY !== 'scroll') continue;
    if (el.scrollHeight <= el.clientHeight + 1) continue;
    const rect = el.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) continue;
    // 可见性过滤（R4 唯一性判定的防污染）：中心不在视口内的面板（离屏/滑出，
    // 如窄窗折叠的 .session-col 覆盖层）不参与。
    if (!paneVisibleInViewport(rect, vw, vh)) continue;
    panes.push({
      id: el.tagName.toLowerCase() + (el.id ? `#${el.id}` : '') + (el.className ? `.${String(el.className).trim().split(/\s+/)[0]}` : ''),
      depth: domDepth(el),
      rect: {left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom},
      scrollTop: el.scrollTop,
      scrollHeight: el.scrollHeight,
      clientHeight: el.clientHeight,
      overscrollContain: cs.overscrollBehaviorY === 'contain',
      node: el,
      el,
      contains: (other) => el.contains(other as Node),
      outer: null,
    });
  }
  // outer 链 = DOM 祖先里最近的面板。
  for (const p of panes) {
    for (let cur: Element | null = p.el.parentElement; cur; cur = cur.parentElement) {
      const outer = panes.find((q) => q.el === cur);
      if (outer) {
        p.outer = outer;
        break;
      }
    }
  }
  cached = {panes, at: now};
  return panes;
}

function hitNodeOf(el: Element): HitNode {
  return {
    id: el.tagName.toLowerCase(),
    depth: domDepth(el),
    node: el,
    contains: (other) => el.contains(other as Node),
    parentOf: () => (el.parentElement ? hitNodeOf(el.parentElement) : null),
  };
}

/** 全局滚轮路由入口：在 App 挂载时调用；返回解绑函数。 */
export function initWheelRouter(): () => void {
  const onWheel = (event: WheelEvent): void => {
    if (event.ctrlKey || event.defaultPrevented) return;
    const hit = document.elementFromPoint(event.clientX, event.clientY) ?? (event.target as Element | null);
    if (!hit || hit === document.documentElement) return;

    // R1 命中链：自底向上找第一个面板。
    const chain: PaneCandidate[] = [];
    for (let cur: Element | null = hit; cur; cur = cur.parentElement) {
      const cs = getComputedStyle(cur);
      if (cs.overflowY === 'auto' || cs.overflowY === 'scroll') {
        chain.push({
          id: cur.tagName.toLowerCase(),
          depth: domDepth(cur),
          rect: {left: 0, top: 0, right: 0, bottom: 0},
          scrollTop: cur.scrollTop,
          scrollHeight: cur.scrollHeight,
          clientHeight: cur.clientHeight,
          overscrollContain: cs.overscrollBehaviorY === 'contain',
          node: cur,
        });
      }
    }
    const panes = collectDomPanes();
    const index: PaneIndex = {panes, hit: hitNodeOf(hit)};
    const decision = routeWheel(chain, index, event.clientX, event.clientY);
    if (decision.kind !== 'route' || !decision.pane) return; // 原生/无解：零干预

    event.preventDefault(); // 只有接管时才拦，原生路径不碰。
    const delta = resolveWheelDelta(event.deltaY, event.deltaMode, window.innerHeight);
    const target = panes.find((p) => p.node === decision.pane!.node) ?? null;
    if (!target) return;
    scrollWithChain(target as ChainPane, delta, (pane, by) => {
      const el = (pane as DomPane).el;
      const before = el.scrollTop;
      el.scrollTop = before + by;
      return el.scrollTop - before;
    });
    cached = null; // 滚动改变几何/可滚性，下次重建索引。
  };
  window.addEventListener('wheel', onWheel, {passive: false});
  return () => window.removeEventListener('wheel', onWheel);
}
