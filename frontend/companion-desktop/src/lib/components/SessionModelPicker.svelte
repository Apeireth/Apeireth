<script lang="ts">
  import {onDestroy, onMount} from 'svelte';
  import {ChevronDown, Search} from 'lucide-svelte';

  interface SessionModelPickerProps {
    models: {id: string; ownedBy?: string}[];
    value: string;
    onSelect: (id: string) => void;
    disabled?: boolean;
  }

  let {
    models = [],
    value = '',
    onSelect,
    disabled = false,
  }: SessionModelPickerProps = $props();

  let open = $state(false);
  let query = $state('');

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return models;
    return models.filter(
      (m) =>
        m.id.toLowerCase().includes(q) ||
        (m.ownedBy ?? '').toLowerCase().includes(q),
    );
  });

  function toggle(): void {
    if (disabled) return;
    open = !open;
    if (open) query = '';
  }

  function select(id: string): void {
    onSelect(id);
    open = false;
    query = '';
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      open = false;
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="model-picker" class:disabled>
  <button
    class="trigger"
    class:open
    onclick={toggle}
    disabled={disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    title={value || '选择模型'}
  >
    <span class="value">{value || '选择模型'}</span>
    <span class="chevron" class:rotated={open}><ChevronDown size={14} /></span>
  </button>

  {#if open}
    <div class="scrim" onclick={() => (open = false)} aria-hidden="true"></div>
    <div class="dropdown" role="listbox" aria-label="模型列表">
      <div class="search">
        <Search size={13} />
        <input bind:value={query} placeholder="过滤模型" />
      </div>

      {#if models.length === 0}
        <div class="empty" role="presentation">
          从 /v1/models 加载失败 — 检查密钥配置
        </div>
      {:else if filtered.length === 0}
        <div class="empty" role="presentation">无匹配</div>
      {:else}
        {#each filtered as model (model.id)}
          <button
            class="option"
            class:current={model.id === value}
            role="option"
            aria-selected={model.id === value}
            onclick={() => select(model.id)}
          >
            <span class="model-id">{model.id}</span>
            {#if model.ownedBy}<span class="owned-by">{model.ownedBy}</span>{/if}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .model-picker {
    position: relative;
    display: inline-block;
  }
  .model-picker.disabled {
    opacity: 0.55;
  }
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 7px 12px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    border-radius: 6px;
    color: var(--text);
    font-size: 13px;
    cursor: pointer;
    max-width: 320px;
  }
  .trigger:hover:not(:disabled) {
    border-color: var(--amber-line);
  }
  .trigger.open {
    border-color: var(--amber-line);
  }
  .trigger:disabled {
    cursor: default;
  }
  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
  }
  .trigger .chevron {
    display: inline-flex;
    flex: none;
    color: var(--muted);
    transition: transform 0.15s ease;
  }
  .trigger .chevron.rotated {
    transform: rotate(180deg);
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 210;
  }
  .dropdown {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 220;
    min-width: 280px;
    max-width: 420px;
    padding: 6px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--faint);
    margin-bottom: 2px;
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font-size: 12px;
  }
  .option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border: 0;
    background: transparent;
    border-radius: 6px;
    color: var(--text);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .option:hover {
    background: var(--surface-3);
  }
  .option.current {
    box-shadow: inset 2px 0 0 var(--amber);
  }
  .model-id {
    flex: none;
    font-family: var(--mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .owned-by {
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
    line-height: 1.5;
  }
</style>
