<script lang="ts">
  import {AlertTriangle, X, ChevronDown, ChevronRight} from 'lucide-svelte';

  interface ErrorSolutionBannerProps {
    code?: string;
    message: string;
    /** 建议的解决方案提示文案。 */
    solution?: string;
    onRetry?: () => void;
    /** 可选关闭按钮。 */
    onClose?: () => void;
  }

  let {
    code,
    message,
    solution,
    onRetry,
    onClose,
  }: ErrorSolutionBannerProps = $props();

  let expanded = $state(false);
</script>

<div class="error-banner" role="alert">
  <div class="banner-main">
    <span class="warn-icon"><AlertTriangle size={16} /></span>
    <div class="banner-content">
      <p class="message">{message}</p>
      {#if solution}
        <p class="solution">
          <span class="solution-label">建议</span>
          {solution}
        </p>
      {/if}
      {#if code}
        <div class="code-detail">
          <button
            class="toggle-btn"
            onclick={() => (expanded = !expanded)}
            aria-expanded={expanded}
          >
            {#if expanded}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
            {expanded ? '收起详情' : '查看错误详情'}
          </button>
          {#if expanded}
            <pre class="code-block">{code}</pre>
          {/if}
        </div>
      {/if}
    </div>
    <div class="banner-actions">
      {#if onRetry}
        <button class="retry-btn" onclick={onRetry}>重试</button>
      {/if}
      {#if onClose}
        <button class="close-btn" onclick={onClose} aria-label="关闭">
          <X size={15} />
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .error-banner {
    border: 1px solid var(--amber-line);
    background: var(--amber-wash);
    border-radius: 8px;
    padding: 10px 12px;
    color: var(--text);
  }
  .banner-main {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .warn-icon {
    flex: none;
    display: grid;
    place-items: center;
    color: var(--amber);
    margin-top: 1px;
  }
  .banner-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .message {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text);
  }
  .solution {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--muted);
  }
  .solution-label {
    display: inline-block;
    margin-right: 6px;
    color: var(--amber);
    font-weight: 600;
  }
  .code-detail {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
  }
  .toggle-btn {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: 11px;
    padding: 0;
    cursor: pointer;
  }
  .toggle-btn:hover {
    color: var(--amber);
  }
  .code-block {
    margin: 0;
    padding: 8px 10px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 11px;
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 200px;
    overflow-y: auto;
  }
  .banner-actions {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .retry-btn {
    border: 1px solid var(--line-strong);
    background: transparent;
    color: var(--muted);
    border-radius: 6px;
    padding: 5px 12px;
    font-size: 12px;
    cursor: pointer;
  }
  .retry-btn:hover {
    color: var(--amber);
    border-color: var(--amber-line);
  }
  .close-btn {
    border: 0;
    background: transparent;
    color: var(--muted);
    padding: 4px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .close-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }
</style>
