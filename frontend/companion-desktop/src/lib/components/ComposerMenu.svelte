<script lang="ts">
  import {onDestroy, onMount} from 'svelte';

  interface ComposerMenuProps {
    open: boolean;
    /** 当前输入的过滤词（斜杠/文件名片段）。 */
    query: string;
    slashCommands: {command: string; title: string; hint?: string}[];
    files: string[];
    onPick: (insert: string) => void;
    onClose: () => void;
  }

  let {
    open = false,
    query = '',
    slashCommands = [],
    files = [],
    onPick,
    onClose,
  }: ComposerMenuProps = $props();

  interface MenuItem {
    key: string;
    kind: 'slash' | 'file';
    label: string;
    insert: string;
    hint?: string;
  }

  const items = $derived.by<MenuItem[]>(() => {
    const q = query.trim().toLowerCase();
    const result: MenuItem[] = [];
    for (const cmd of slashCommands) {
      const matched =
        !q ||
        cmd.command.toLowerCase().includes(q) ||
        cmd.title.toLowerCase().includes(q);
      if (!matched) continue;
      const hint = cmd.hint ? `${cmd.title} — ${cmd.hint}` : cmd.title;
      result.push({
        key: `slash:${cmd.command}`,
        kind: 'slash',
        label: cmd.command,
        insert: cmd.command,
        hint,
      });
    }
    for (const file of files) {
      const matched = !q || file.toLowerCase().includes(q);
      if (!matched) continue;
      result.push({
        key: `file:${file}`,
        kind: 'file',
        label: file,
        insert: `@${file}`,
      });
    }
    return result;
  });

  let selectedIndex = $state(0);

  // 过滤结果变化时回到第一项，避免越界高亮。
  $effect(() => {
    const count = items.length;
    selectedIndex = 0;
  });

  function move(delta: number): void {
    if (items.length === 0) return;
    selectedIndex = (selectedIndex + delta + items.length) % items.length;
  }

  function pickCurrent(): void {
    const item = items[selectedIndex];
    if (item) onPick(item.insert);
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
      pickCurrent();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

{#if open}
  <div class="composer-menu" role="listbox" aria-label="命令与文件">
    {#if items.length === 0}
      <div class="empty" role="presentation">无匹配</div>
    {:else}
      {#each items as item, index (item.key)}
        <button
          class="menu-item"
          class:selected={index === selectedIndex}
          class:file={item.kind === 'file'}
          role="option"
          aria-selected={index === selectedIndex}
          onmousemove={() => (selectedIndex = index)}
          onclick={() => onPick(item.insert)}
        >
          <span class="kind-badge">{item.kind === 'slash' ? '/' : '@'}</span>
          <span class="label">{item.label}</span>
          {#if item.hint}<span class="hint">{item.hint}</span>{/if}
        </button>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .composer-menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 200;
    min-width: 280px;
    max-width: 420px;
    max-height: 320px;
    overflow-y: auto;
    padding: 6px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border: 0;
    background: transparent;
    border-radius: 6px;
    color: var(--text);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .menu-item:hover,
  .menu-item.selected {
    background: var(--surface-3);
  }
  .menu-item.selected {
    box-shadow: inset 2px 0 0 var(--amber);
  }
  .kind-badge {
    flex: none;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--amber);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 700;
  }
  .menu-item.file .kind-badge {
    color: var(--blue);
  }
  .label {
    flex: none;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
  }
  .hint {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--faint);
    font-size: 11px;
  }
  .empty {
    padding: 10px 12px;
    color: var(--faint);
    font-size: 12px;
    text-align: center;
  }
</style>
