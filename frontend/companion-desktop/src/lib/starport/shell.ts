// 星港 · 多标签壳（内置应用面 I 批 1，纯逻辑零依赖）
//
// 契约（I-starport-builtins.md）：
//   1. 视图即标签 —— openApp 挂载内置件并入标签体系；closeView 强制走
//      unmount 清理链（mount 返回的 cleanup 必被调用，包括异常路径）；
//   2. 单实例语义 —— openOrFocusApp 复用既有视图（内置件默认单实例，
//      重复打开只换焦点，不叠标签）；
//   3. 浮层配对 —— acquire/release 成对，release 未知令牌一律抛错
//      （防浮层孤儿泄漏）；
//   4. 安定回调 —— whenSettled 在挂载/清理队列清空后兑现（测试断言用，
//      与上游「安定回调」同一测试面思想，命名中性）；
//   5. 0 装 —— 开未注册应用/关未知视图/放未知浮层一律抛错，不猜。
//
// 宿主注入：mountApp/unmountApp 由 Svelte 层提供（真实 DOM）或测试桩提供，
// 本模块不触 DOM、不 import svelte（tests/starport-shell.mjs 直测）。

import type {AppContribution, AppMountContext, ContributionRegistry} from './contribution-registry';

export interface StarportHost {
  /** 宿主挂载：返回清理函数（可选）；可返回 Promise（挂载进入 pending 集）。 */
  mountApp: (
    app: AppContribution,
    context: AppMountContext,
  ) => void | (() => void) | Promise<void | (() => void)>;
}

export interface StarportViewSnapshot {
  viewId: string;
  appId: string;
  title: string;
  status: 'mounting' | 'mounted' | 'closing';
}

export interface StarportSnapshot {
  views: StarportViewSnapshot[];
  activeViewId: string | null;
  openOverlays: string[];
}

export class StarportShellError extends Error {
  readonly code: 'app-missing' | 'view-missing' | 'overlay-missing' | 'overlay-duplicate';

  constructor(
    code: 'app-missing' | 'view-missing' | 'overlay-missing' | 'overlay-duplicate',
    message: string,
  ) {
    super(message);
    this.name = 'StarportShellError';
    this.code = code;
  }
}

interface ViewEntry {
  viewId: string;
  appId: string;
  title: string;
  status: StarportViewSnapshot['status'];
  cleanup: (() => void) | null;
}

export function createStarportShell(options: {
  registry: ContributionRegistry;
  host: StarportHost;
  /** 测试注入时钟；默认 Date.now。 */
  now?: () => number;
}) {
  const {registry, host} = options;
  const views = new Map<string, ViewEntry>();
  const overlays = new Map<string, string>();
  const pending = new Set<Promise<unknown>>();
  let activeViewId: string | null = null;
  let viewSeq = 0;
  let overlaySeq = 0;
  const settleWaiters = new Set<() => void>();

  function track<T>(work: Promise<T>): Promise<T> {
    pending.add(work);
    void work.finally(() => {
      pending.delete(work);
      if (pending.size === 0) for (const wake of [...settleWaiters]) wake();
    });
    return work;
  }

  function closeView(viewId: string): void {
    const entry = views.get(viewId);
    if (!entry) throw new StarportShellError('view-missing', `视图不存在：${viewId}`);
    entry.status = 'closing';
    const cleanup = entry.cleanup;
    entry.cleanup = null;
    try {
      cleanup?.();
    } finally {
      views.delete(viewId);
      if (activeViewId === viewId) {
        activeViewId = [...views.keys()].at(-1) ?? null;
      }
      if (pending.size === 0) for (const wake of [...settleWaiters]) wake();
    }
  }

  function openApp(appId: string): string {
    const app = registry.getApp(appId);
    if (!app) throw new StarportShellError('app-missing', `应用未注册：${appId}`);
    viewSeq += 1;
    const viewId = `view:${viewSeq}`;
    const entry: ViewEntry = {
      viewId,
      appId,
      title: app.title,
      status: 'mounting',
      cleanup: null,
    };
    views.set(viewId, entry);
    activeViewId = viewId;
    const context: AppMountContext = {
      viewId,
      appId,
      close: () => closeView(viewId),
    };
    const mountResult = host.mountApp(app, context);
    if (mountResult instanceof Promise) {
      track(
        mountResult.then((cleanup) => {
          if (views.has(viewId) && entry.status === 'mounting') {
            entry.status = 'mounted';
            entry.cleanup = typeof cleanup === 'function' ? cleanup : null;
          } else if (typeof cleanup === 'function') {
            // 挂载在途时视图已关：立即执行迟到的清理，不留孤儿。
            cleanup();
          }
        }),
      );
    } else {
      entry.status = 'mounted';
      entry.cleanup = typeof mountResult === 'function' ? mountResult : null;
    }
    return viewId;
  }

  /** 单实例语义：已有该应用视图则聚焦，不叠标签。 */
  function openOrFocusApp(appId: string): string {
    for (const entry of views.values()) {
      if (entry.appId === appId) {
        activeViewId = entry.viewId;
        return entry.viewId;
      }
    }
    return openApp(appId);
  }

  function setActiveView(viewId: string): void {
    if (!views.has(viewId)) throw new StarportShellError('view-missing', `视图不存在：${viewId}`);
    activeViewId = viewId;
  }

  function acquireOverlay(name: string): string {
    if ([...overlays.values()].includes(name)) {
      throw new StarportShellError('overlay-duplicate', `浮层已在打开态：${name}`);
    }
    overlaySeq += 1;
    const token = `overlay:${overlaySeq}`;
    overlays.set(token, name);
    return token;
  }

  function releaseOverlay(token: string): void {
    if (!overlays.has(token)) {
      throw new StarportShellError('overlay-missing', `浮层令牌未知或已释放：${token}`);
    }
    overlays.delete(token);
  }

  function getSnapshot(): StarportSnapshot {
    return {
      views: [...views.values()].map((entry) => ({
        viewId: entry.viewId,
        appId: entry.appId,
        title: entry.title,
        status: entry.status,
      })),
      activeViewId,
      openOverlays: [...overlays.values()],
    };
  }

  /** 挂载/清理队列清空后兑现（供测试断言安定态）。 */
  function whenSettled(): Promise<void> {
    if (pending.size === 0) return Promise.resolve();
    return new Promise<void>((resolve) => settleWaiters.add(resolve));
  }

  /** 视图全关（壳层卸载链路；每个清理恰好一次）。 */
  function dispose(): void {
    for (const viewId of [...views.keys()]) closeView(viewId);
    overlays.clear();
  }

  return {
    openApp,
    openOrFocusApp,
    closeView,
    setActiveView,
    acquireOverlay,
    releaseOverlay,
    getSnapshot,
    whenSettled,
    dispose,
  };
}

export type StarportShell = ReturnType<typeof createStarportShell>;
