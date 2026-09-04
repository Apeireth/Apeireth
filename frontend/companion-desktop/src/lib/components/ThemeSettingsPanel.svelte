<script lang="ts">
  import {Check} from 'lucide-svelte';
  import type {ApeirethConfig, Theme} from '../types';
  import {THEME_CATALOG, applyDocumentTheme, resolveTheme} from '../theme';

  let {
    config,
    onSave,
  }: {
    config: ApeirethConfig;
    onSave: (cfg: ApeirethConfig) => void;
  } = $props();

  const active = $derived(resolveTheme(config.theme));

  function pick(theme: Theme): void {
    if (theme === active) return;
    applyDocumentTheme(theme);
    onSave({...config, theme});
  }
</script>

<div class="theme-panel">
  <p class="theme-lede">
    切换界面照明与背景。Essence 使用星空山脉壁纸；深空舰桥保留 WebGL 场景与金色存在纪律。
  </p>
  <div class="theme-grid" role="listbox" aria-label="界面主题">
    {#each THEME_CATALOG as item (item.id)}
      <button
        class="theme-card"
        class:selected={active === item.id}
        role="option"
        aria-selected={active === item.id}
        onclick={() => pick(item.id)}
      >
        <div class="theme-swatch" style:background={item.swatch}></div>
        <div class="theme-meta">
          <span class="theme-name">{item.label}</span>
          <span class="theme-desc">{item.desc}</span>
        </div>
        {#if active === item.id}
          <span class="theme-check" aria-hidden="true"><Check size={14} /></span>
        {/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .theme-panel {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .theme-lede {
    margin: 0;
    font-size: 13px;
    line-height: 1.65;
    color: var(--muted);
    max-width: 62ch;
  }
  .theme-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
  }
  .theme-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0;
    padding: 0;
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    background: var(--surface-2);
    overflow: hidden;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.2s, box-shadow 0.2s, transform 0.15s;
  }
  .theme-card:hover {
    border-color: var(--amber-line);
    transform: translateY(-1px);
  }
  .theme-card.selected {
    border-color: var(--amber);
    box-shadow: 0 0 0 1px var(--amber-line), 0 8px 28px -8px rgba(0, 0, 0, 0.2);
  }
  .theme-swatch {
    height: 72px;
    border-bottom: 1px solid var(--line);
  }
  .theme-meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 12px 12px;
  }
  .theme-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .theme-card.selected .theme-name {
    color: var(--amber);
  }
  .theme-desc {
    font-size: 11px;
    color: var(--muted);
    line-height: 1.4;
  }
  .theme-check {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--amber);
    color: #1b1409;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  }
</style>
