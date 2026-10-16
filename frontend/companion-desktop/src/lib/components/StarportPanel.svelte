<!-- 星港宿主面板（内置应用面 I 批 1 轨 A 收尾）
     #
     # 职责：壳层唯一天然宿主——持有贡献点注册表与多标签壳，把内置件
     # mount 进视图容器，标签栏驱动视图生命周期（关闭必走清理链）。
     # 入口遵循侧栏收纳宪法：不加第五栏位，走工作台卡片入口（与记忆/
     # 日记同模式，见 lib/shell-nav.ts）。
     #
     # 0 装：hello 契约件是活体契约证明（挂载回执=真实挂载事件），
     # 不是假仪表盘；后续内置件（星笺/铭刻台/星屑…）按批注册进来。-->
<script lang="ts">
  import {onMount} from 'svelte';
  import {Plus, X} from 'lucide-svelte';
  import {loadConfig} from '../runtime';
  import {ContributionRegistry, type AppContribution} from '../starport/contribution-registry';
  import {createStarportShell, type StarportSnapshot} from '../starport/shell';
  import {installStarportBuiltins, STARPORT_HELLO_APP_ID} from '../starport/builtins';
  import {installStarportFaces} from '../starport/faces-install';
  import {listFacesByStatus} from '../starport/faces';

  let hostEl: HTMLDivElement | null = null;
  let snapshot = $state<StarportSnapshot>({views: [], activeViewId: null, openOverlays: []});
  let apps = $state<AppContribution[]>([]);

  // 未组装面（faces.ts 单一口径）：诚实外显，不设假旋钮。
  const pendingFaces = listFacesByStatus('not_assembled')
    .map((face) => face.codename)
    .join('、');

  // 壳与注册表是命令式对象（非响应式），面板卸载即整体释放。
  const registry = new ContributionRegistry();
  let shell: ReturnType<typeof createStarportShell> | null = null;

  function refresh(): void {
    if (shell) snapshot = shell.getSnapshot();
  }

  onMount(() => {
    if (!hostEl) return;
    const hostRoot = hostEl;
    shell = createStarportShell({
      registry,
      host: {
        mountApp: (app, context) => {
          const viewRoot = document.createElement('div');
          viewRoot.className = 'starport-view';
          hostRoot.appendChild(viewRoot);
          const appCleanup = app.mount(viewRoot, context);
          return () => {
            if (typeof appCleanup === 'function') appCleanup();
            viewRoot.remove();
          };
        },
      },
    });
    installStarportBuiltins(registry, {
      onOpenHello: () => {
        shell?.openOrFocusApp(STARPORT_HELLO_APP_ID);
        refresh();
      },
    });
    // 批 2–5 工作切片面统一下挂（星屑/铭刻台/星笺/深望/星仪/泊位/信风/通译/船坞）。
    installStarportFaces({config: loadConfig(), registry});
    apps = registry.listApps();
    // 首个活体内置件随面板亮起（挂载回执即契约证明）。
    shell.openApp(STARPORT_HELLO_APP_ID);
    refresh();
    return () => {
      shell?.dispose();
      shell = null;
    };
  });

  function openHello(): void {
    shell?.openOrFocusApp(STARPORT_HELLO_APP_ID);
    refresh();
  }

  function openAppById(appId: string): void {
    shell?.openOrFocusApp(appId);
    refresh();
  }

  function focusView(viewId: string): void {
    shell?.setActiveView(viewId);
    refresh();
  }

  function closeView(viewId: string): void {
    shell?.closeView(viewId);
    refresh();
  }
</script>

<section class="starport-panel" aria-label="星港内置应用面">
  <header class="starport-bar">
    <span class="starport-title">星港 · 内置应用面</span>
    <div class="starport-faces">
      {#each apps as app (app.id)}
        <button
          type="button"
          class="starport-cmd"
          onclick={() => openAppById(app.id)}
          title={`挂载 ${app.title}`}
        >
          <Plus size={12} />
          <span>{app.title}</span>
        </button>
      {/each}
    </div>
  </header>

  <div class="starport-tabs" role="tablist" aria-label="星港标签">
    {#each snapshot.views as view (view.viewId)}
      <div class="starport-tab" class:active={snapshot.activeViewId === view.viewId}>
        <button
          type="button"
          class="starport-tab-label"
          role="tab"
          aria-selected={snapshot.activeViewId === view.viewId}
          onclick={() => focusView(view.viewId)}
        >{view.title}</button>
        <button
          type="button"
          class="starport-tab-close"
          aria-label={`关闭 ${view.title}`}
          onclick={() => closeView(view.viewId)}
        ><X size={11} /></button>
      </div>
    {/each}
  </div>

  <div class="starport-host" bind:this={hostEl}></div>

  <div class="starport-pending">
    未组装（诚实降级，不设假旋钮）：{pendingFaces}
  </div>
</section>

<style>
  .starport-panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 220px;
  }

  .starport-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .starport-title {
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 11px;
    letter-spacing: 0.18em;
    color: rgba(232, 224, 204, 0.8);
  }

  .starport-cmd {
    appearance: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    border-radius: 6px;
    border: 1px solid rgba(255, 210, 122, 0.4);
    background: transparent;
    color: #ffd27a;
    font-size: 11px;
    cursor: pointer;
  }

  .starport-tabs {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .starport-tab {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 2px 4px 2px 10px;
    border-radius: 6px;
    border: 1px solid rgba(232, 224, 204, 0.18);
  }

  .starport-tab.active {
    border-color: rgba(255, 210, 122, 0.5);
  }

  .starport-tab-label {
    appearance: none;
    border: none;
    background: transparent;
    color: rgba(232, 224, 204, 0.85);
    font-size: 11px;
    cursor: pointer;
  }

  .starport-tab-close {
    appearance: none;
    border: none;
    background: transparent;
    color: rgba(232, 224, 204, 0.55);
    cursor: pointer;
    display: inline-flex;
    padding: 2px;
  }

  .starport-host {
    flex: 1;
    min-height: 160px;
    border-radius: 8px;
    border: 1px dashed rgba(232, 224, 204, 0.16);
    padding: 8px;
    display: flex;
    flex-direction: column;
  }

  .starport-host :global(.starport-view) {
    flex: 1;
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 11px;
    color: rgba(232, 224, 204, 0.8);
  }
  .starport-faces {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .starport-pending {
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 9px;
    letter-spacing: 0.08em;
    color: rgba(232, 224, 204, 0.45);
    word-break: break-all;
  }
</style>
