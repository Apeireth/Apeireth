<script lang="ts">
  import {onMount} from 'svelte';
  import {Check, Upload, ImageOff} from 'lucide-svelte';
  import type {Accent, ApeirethConfig, Theme} from '../types';
  import {
    ACCENT_CATALOG,
    THEME_CATALOG,
    applyDocumentAccent,
    applyDocumentTheme,
    resolveAccent,
    resolveTheme,
  } from '../theme';
  import {clearCustomBg, getCustomBg, putCustomBg, validateCustomBgFile} from '../bg-store';

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

  /* ---------- UI 配色（规范 §8 增补⑤）：只染 UI chrome，可选方案无金色系 ---------- */
  const activeAccent = $derived(resolveAccent(config.accent));

  function pickAccent(accent: Accent): void {
    if (accent === activeAccent) return;
    applyDocumentAccent(accent);
    onSave({...config, accent});
  }

  /* ---------- 自定义背景（规范 §8 增补④）----------
     图本体存 IndexedDB（bg-store.ts），config 只记开关 customBg。
     真实边界（0 装）：web dev 按 localhost 原点持久化，换端口/换浏览器需重传；
     Tauri 壳走 WebView2 的 IndexedDB，desktop-bridge 无磁盘文件持久化能力。 */
  let bgBusy = $state(false);
  let bgError = $state('');
  let customPreview = $state<string | null>(null);
  let fileInput = $state<HTMLInputElement | undefined>();

  onMount(() => {
    if (config.customBg) {
      void getCustomBg()
        .then((blob) => {
          if (blob) customPreview = URL.createObjectURL(blob);
        })
        .catch(() => {
          /* 读不出就保持无预览，App 侧会诚实回落开关 */
        });
    }
    return () => {
      if (customPreview) URL.revokeObjectURL(customPreview);
    };
  });

  async function handleFile(e: Event): Promise<void> {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = ''; // 允许重选同一文件
    if (!file) return;
    const verdict = validateCustomBgFile(file.size, file.type);
    if (!verdict.ok) {
      bgError = verdict.reason;
      return;
    }
    bgBusy = true;
    bgError = '';
    try {
      await putCustomBg(file);
      if (customPreview) URL.revokeObjectURL(customPreview);
      customPreview = URL.createObjectURL(file);
      onSave({...config, customBg: true});
    } catch (err) {
      bgError = err instanceof Error ? err.message : '写入本机浏览器数据库失败';
    } finally {
      bgBusy = false;
    }
  }

  /** 恢复主题默认背景：上传的图保留在 IndexedDB，随时可重新启用。 */
  function useThemeDefault(): void {
    onSave({...config, customBg: false});
  }

  async function clearBg(): Promise<void> {
    bgBusy = true;
    try {
      await clearCustomBg();
      if (customPreview) {
        URL.revokeObjectURL(customPreview);
        customPreview = null;
      }
      onSave({...config, customBg: false});
    } finally {
      bgBusy = false;
    }
  }
</script>

<div class="theme-panel">
  <p class="theme-lede">
    切换界面照明与背景。默认遗产星空为静态图；深空舰桥保留 WebGL 实时场景与金色存在纪律；Essence 使用星空山脉浅色雾面。
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

  <div class="bg-section">
    <h3 class="bg-title">自定义背景</h3>
    <p class="bg-lede">
      上传一张图片盖住主题默认背景（未选中会话时的右侧显影区与全局底）。
      图片只存在本机浏览器数据库里——开发模式换端口或换浏览器后需要重传。
    </p>
    <div class="bg-row">
      {#if config.customBg && customPreview}
        <div
          class="bg-thumb"
          style:background-image={`url(${customPreview})`}
          role="img"
          aria-label="当前自定义背景预览"
        ></div>
      {/if}
      <div class="bg-actions">
        <button class="quiet-btn" onclick={() => fileInput?.click()} disabled={bgBusy}>
          <Upload size={13} />
          {config.customBg ? '重新上传' : '上传图片'}
        </button>
        {#if config.customBg}
          <button class="quiet-btn" onclick={useThemeDefault} disabled={bgBusy}>恢复主题默认</button>
          <button class="quiet-btn" onclick={clearBg} disabled={bgBusy}>
            <ImageOff size={13} />
            清除已传图片
          </button>
        {/if}
      </div>
    </div>
    {#if bgError}
      <p class="bg-error" role="alert">{bgError}</p>
    {/if}
    <input
      bind:this={fileInput}
      type="file"
      accept="image/png,image/jpeg,image/webp,image/avif,image/gif"
      class="bg-file"
      onchange={handleFile}
    />
  </div>

  <div class="accent-section">
    <h3 class="bg-title">UI 配色</h3>
    <p class="bg-lede">
      配色只染界面高亮（导航激活、开关这类 UI 元素）。金色是他的存在色——可选方案里没有金色系。
    </p>
    <div class="accent-grid" role="listbox" aria-label="UI 配色方案">
      {#each ACCENT_CATALOG as item (item.id)}
        <button
          class="accent-card"
          class:selected={activeAccent === item.id}
          role="option"
          aria-selected={activeAccent === item.id}
          onclick={() => pickAccent(item.id)}
        >
          <!-- 真渲染预览：内联覆写 accent 变量，样品是真实 CSS 级联产物，不是示意图 -->
          <span
            class="accent-sample"
            style:--ap-accent-ui={item.accent}
            style:--ap-accent-ui-ink={item.ink}
            aria-hidden="true"
          >
            <span class="sample-tab">导航</span>
            <span class="sample-pill">开关</span>
          </span>
          <span class="accent-meta">
            <span class="accent-name">{item.label}</span>
            <span class="accent-desc">{item.desc}</span>
          </span>
          {#if activeAccent === item.id}
            <span class="accent-check" aria-hidden="true"><Check size={12} /></span>
          {/if}
        </button>
      {/each}
    </div>
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
    border-color: var(--ap-accent-ui);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--ap-accent-ui) 45%, transparent), 0 8px 28px -8px rgba(0, 0, 0, 0.2);
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
    color: var(--ap-accent-ui);
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
    background: var(--ap-accent-ui);
    color: var(--ap-accent-ui-ink);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  }

  /* ---------- 自定义背景（规范 §8 增补④） ---------- */
  .bg-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-top: 1px solid var(--line);
    padding-top: 16px;
  }
  .bg-title {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text);
  }
  .bg-lede {
    margin: 0;
    font-size: 12px;
    line-height: 1.7;
    color: var(--muted);
    max-width: 62ch;
  }
  .bg-row {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .bg-thumb {
    width: 128px;
    height: 72px;
    border-radius: 8px;
    border: 1px solid var(--line-strong);
    background: center / cover no-repeat;
  }
  .bg-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .bg-actions :global(.quiet-btn) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .bg-error {
    margin: 0;
    font-size: 12px;
    color: var(--danger);
  }
  .bg-file {
    display: none;
  }

  /* ---------- UI 配色（规范 §8 增补⑤） ---------- */
  .accent-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-top: 1px solid var(--line);
    padding-top: 16px;
  }
  .accent-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 10px;
  }
  .accent-card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px 12px;
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    background: var(--surface-2);
    cursor: pointer;
    text-align: left;
    transition: border-color 0.2s, box-shadow 0.2s;
  }
  .accent-card:hover {
    border-color: color-mix(in srgb, var(--ap-accent-ui) 45%, transparent);
  }
  .accent-card.selected {
    border-color: var(--ap-accent-ui);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--ap-accent-ui) 40%, transparent);
  }
  .accent-sample {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
  }
  .sample-tab {
    padding: 4px 12px;
    border-left: 2px solid var(--ap-accent-ui);
    border-radius: 0 6px 6px 0;
    background: color-mix(in srgb, var(--ap-accent-ui) 12%, transparent);
    color: var(--ap-accent-ui);
    font-size: 11px;
    letter-spacing: 0.06em;
  }
  .sample-pill {
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--ap-accent-ui);
    color: var(--ap-accent-ui-ink);
    font-size: 10px;
    letter-spacing: 0.06em;
  }
  .accent-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .accent-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
  }
  .accent-desc {
    font-size: 10.5px;
    color: var(--muted);
  }
  .accent-check {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--ap-accent-ui);
    color: var(--ap-accent-ui-ink);
  }
</style>
