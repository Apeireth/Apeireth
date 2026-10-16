<script lang="ts">
  // Ctrl+K 命令面板（00-PHILOSOPHY §6 原则 3「一个入口」；gap-plan §4.4 组件 2）。
  //
  // 视觉：Deep-Ops 深舱底（deepOps.base）居中浮层——它是「召唤出来的机器面」，
  // 不是场景的一部分；关闭即回到舰桥，用户从未离开（§5.1 层序）。
  // 金色纪律：选中行沿用壳层既有语言（ComposerMenu 同款 2px 金内缘），
  // 面板内不产生任何金色装饰元素。
  // 0 装：disabledReason 在册的命令置灰展示原因，不藏、不可执行、不计入最近榜。
  import {onDestroy, onMount, tick} from 'svelte';
  import {Search, Clock} from 'lucide-svelte';
  import type {CommandItem} from '../commands/registry';
  import {filterCommands, orderCommands} from '../commands/registry';

  let {
    open = false,
    commands = [],
    recentIds = [],
    onExecute,
    onClose,
  }: {
    open: boolean;
    commands: readonly CommandItem[];
    recentIds: readonly string[];
    /** 执行某命令（调用方负责执行后关闭面板与记入最近榜）。 */
    onExecute: (id: string) => void;
    onClose: () => void;
  } = $props();

  let query = $state('');
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  const visible = $derived(orderCommands(filterCommands(commands, query), recentIds));
  const recentSet = $derived(new Set(recentIds));

  // 每次召唤都是新会话：清空搜索词、回到第一项、焦点进输入框。
  $effect(() => {
    if (!open) return;
    query = '';
    selectedIndex = 0;
    void tick().then(() => inputEl?.focus());
  });

  // 过滤词变化时回到第一项，避免越界高亮（ComposerMenu 同款约定）。
  $effect(() => {
    void query;
    selectedIndex = 0;
  });

  function move(delta: number): void {
    if (visible.length === 0) return;
    selectedIndex = (selectedIndex + delta + visible.length) % visible.length;
  }

  function execute(item: CommandItem | undefined): void {
    if (!item || item.disabledReason) return; // 置灰命令：显示但不可执行（0 装）
    onExecute(item.id);
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (!open) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      move(1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      move(-1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      execute(visible[selectedIndex]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }

  onMount(() => window.addEventListener('keydown', handleKeydown));
  onDestroy(() => window.removeEventListener('keydown', handleKeydown));
</script>

{#if open}
  <div
    class="palette-backdrop"
    onclick={(e) => {
      // 点到面板外才关闭（target===currentTarget 即 backdrop 本体）
      if (e.target === e.currentTarget) onClose();
    }}
    role="presentation"
  >
    <div
      class="palette"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-label="命令面板"
    >
      <div class="palette-input-row">
        <Search size={14} class="palette-search-icon" />
        <input
          bind:this={inputEl}
          bind:value={query}
          class="palette-input"
          type="text"
          placeholder="召唤命令——标题、拼音或英文缩写均可"
          aria-label="搜索命令"
        />
        <span class="kbd">Esc</span>
      </div>

      <div class="palette-list" role="listbox" aria-label="命令列表">
        {#if visible.length === 0}
          <p class="palette-empty">没有匹配的命令——试试拼音或英文缩写。</p>
        {:else}
          {#each visible as item, index (item.id)}
            {@const disabled = !!item.disabledReason}
            <button
              class="palette-item"
              class:selected={index === selectedIndex}
              class:disabled
              role="option"
              aria-selected={index === selectedIndex}
              aria-disabled={disabled}
              onmousemove={() => (selectedIndex = index)}
              onclick={() => execute(item)}
            >
              <span class="item-title">{item.title}</span>
              {#if disabled}
                <span class="item-reason">{item.disabledReason}</span>
              {:else}
                {#if recentSet.has(item.id)}
                  <span class="item-recent" title="最近使用"><Clock size={10} />最近</span>
                {/if}
                {#if item.hint}<span class="item-hint">{item.hint}</span>{/if}
              {/if}
              <span class="item-group">{item.group}</span>
            </button>
          {/each}
        {/if}
      </div>

      <div class="palette-foot">
        <span class="kbd">↑</span><span class="kbd">↓</span><span>选择</span>
        <span class="kbd">↵</span><span>执行</span>
        <span class="kbd">Ctrl</span><span class="kbd">K</span><span>收起</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    z-index: 400;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 16vh;
    background: rgba(5, 10, 15, 0.55); /* deepOps.deep 系压暗，场景仍在场 */
    backdrop-filter: blur(2px);
  }
  .palette {
    width: min(560px, calc(100vw - 48px));
    max-height: 54vh;
    display: flex;
    flex-direction: column;
    background: var(--ap-register-deepops-base);
    border: 1px solid var(--ap-line);
    border-radius: 12px;
    box-shadow: 0 18px 60px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }
  .palette-input-row {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--ap-line);
  }
  .palette-input-row :global(.palette-search-icon) {
    flex: none;
    color: var(--ap-bone-42);
  }
  .palette-input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--ap-bone);
    font-family: var(--ap-font-ui);
    font-size: 14px;
    outline: none;
  }
  .palette-input::placeholder {
    color: var(--ap-bone-30);
  }
  .palette-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px;
  }
  .palette-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--ap-bone-68);
    font-family: var(--ap-font-ui);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .palette-item.selected {
    background: rgba(232, 224, 204, 0.06);
    color: var(--ap-bone);
    /* 壳层选中语言 = 金内缘（ComposerMenu 同款；金=存在，不是装饰） */
    box-shadow: inset 2px 0 0 var(--ap-gold);
  }
  .palette-item.disabled {
    cursor: default;
    opacity: 0.45;
  }
  .palette-item.disabled.selected {
    box-shadow: inset 2px 0 0 var(--ap-bone-30); /* 置灰行不挣金 */
  }
  .item-title {
    flex: none;
    max-width: 46%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item-reason {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ap-semantic-info); /* 原因是中性信息（§2.4），不告警 */
    font-size: 11px;
  }
  .item-recent {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--ap-bone-42);
    font-size: 10.5px;
  }
  .item-hint {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ap-bone-42);
    font-size: 11px;
  }
  .item-group {
    flex: none;
    margin-left: auto;
    color: var(--ap-bone-30);
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.06em;
  }
  .palette-empty {
    padding: 18px 12px;
    margin: 0;
    color: var(--ap-bone-42);
    font-size: 12px;
    text-align: center;
  }
  .palette-foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-top: 1px solid var(--ap-line);
    color: var(--ap-bone-42);
    font-size: 11px;
  }
  .kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    padding: 1px 5px;
    border: 1px solid var(--ap-line);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: rgba(232, 224, 204, 0.05);
    color: var(--ap-bone-68);
    font-family: var(--ap-font-mono);
    font-size: 10px;
  }
</style>
