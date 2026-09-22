<script lang="ts">
  // 底部状态条 —— 余光投影（00-PHILOSOPHY §2 第四个投影；gap-plan §4.3 P1）。
  //
  // 降密度纪律（§4.5）：只留 4 个指标，一行等宽小字（§6.2 状态行 11px/0.28em）。
  // 静是默认：常态全部骨白灰；只有异常挣色——黄=待完善（重连中/拉取失败/守卫
  // 新事件），红=危险（断连超 30s，附 SIM 纪律标注）。金档留给「他在场」，
  // 这四个指标无此语义，本条不用金。
  // 它是入口不是面板：每项点击直达对应动作/视图，不展开任何浮层。
  import type {StatusIndicator} from '../statusbar';

  let {
    sse,
    turn,
    guard,
    memory,
    onSseClick,
    onTurnClick,
    onGuardClick,
    onMemoryClick,
  }: {
    sse: StatusIndicator;
    turn: StatusIndicator;
    guard: StatusIndicator;
    memory: StatusIndicator;
    onSseClick: () => void;
    onTurnClick: () => void;
    onGuardClick: () => void;
    onMemoryClick: () => void;
  } = $props();

  const items = $derived([
    {key: 'sse', ind: sse, onclick: onSseClick},
    {key: 'turn', ind: turn, onclick: onTurnClick},
    {key: 'guard', ind: guard, onclick: onGuardClick},
    {key: 'memory', ind: memory, onclick: onMemoryClick},
  ]);
</script>

<footer class="status-bar" aria-label="状态条">
  {#each items as item (item.key)}
    {#if item.ind.clickable}
      <button
        class="sb-item tone-{item.ind.tone}"
        title={item.ind.title}
        onclick={item.onclick}
      >
        {item.ind.text}{#if item.ind.sim}<span class="sim-badge">SIM</span>{/if}
      </button>
    {:else}
      <span class="sb-item tone-{item.ind.tone}" title={item.ind.title}>
        {item.ind.text}{#if item.ind.sim}<span class="sim-badge">SIM</span>{/if}
      </span>
    {/if}
  {/each}
</footer>

<style>
  /* 余光 = 余光：一行通栏，不抢戏、不闪烁、无动画 */
  .status-bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 22px;
    height: 26px;
    padding: 0 18px;
    border-top: 1px solid var(--ap-line);
    /* 页面层承托：半透明面板底 + 虚化，所有主题下可读（§5.1 页面层语言） */
    background: var(--ap-panel);
    backdrop-filter: blur(10px);
    font-family: var(--ap-font-mono);
    font-size: 11px;
    letter-spacing: 0.28em;
    color: var(--ap-bone-42);
    user-select: none;
  }
  .sb-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 0;
    background: transparent;
    padding: 0;
    font: inherit;
    letter-spacing: inherit;
    color: var(--ap-bone-42);
    white-space: nowrap;
  }
  button.sb-item {
    cursor: pointer;
    transition: color 0.18s ease;
  }
  button.sb-item:hover {
    color: var(--ap-bone-68);
  }
  /* 异常才挣色（§2.4 语义色，禁止装饰性使用） */
  .sb-item.tone-warn {
    color: var(--ap-semantic-warning);
  }
  .sb-item.tone-danger {
    color: var(--ap-semantic-danger);
  }
  button.sb-item.tone-warn:hover {
    color: var(--ap-semantic-warning);
  }
  button.sb-item.tone-danger:hover {
    color: var(--ap-semantic-danger);
  }
</style>
