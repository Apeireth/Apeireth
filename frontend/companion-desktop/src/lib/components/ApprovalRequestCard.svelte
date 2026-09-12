<script lang="ts">
  import {X, Copy, Check} from 'lucide-svelte';

  interface ApprovalRequestCardProps {
    item: {
      title?: string;
      commandText?: string;
      argumentsSummary?: string;
      reason?: string;
      createdAt?: number;
    };
    busy: boolean;
    onAllow: () => void;
    onReject: () => void;
    /** 右上角 X：仅关闭卡片，不做业务决策（区别于「拒绝」按钮）。 */
    onDismiss?: () => void;
  }

  let {
    item,
    busy = false,
    onAllow,
    onReject,
    onDismiss,
  }: ApprovalRequestCardProps = $props();

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  async function copyCommand(): Promise<void> {
    const text = item.commandText ?? '';
    if (!text) return;
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        return;
      }
    } catch {
      // 复制失败静默处理
      return;
    }
    copied = true;
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => {
      copied = false;
    }, 1500);
  }

  function formatTime(ts?: number): string {
    if (!ts && ts !== 0) return '';
    const d = new Date(ts);
    if (Number.isNaN(d.getTime())) return '';
    return d.toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'});
  }
</script>

<div class="approval-card" role="group" aria-label="待批准操作">
  <div class="card-header">
    <div class="title-wrap">
      <h3 class="title">{item.title || '需要批准的操作'}</h3>
      {#if item.createdAt !== undefined}
        <span class="time">{formatTime(item.createdAt)}</span>
      {/if}
    </div>
    {#if onDismiss}
      <button
        class="close-btn"
        onclick={onDismiss}
        disabled={busy}
        aria-label="关闭"
        title="关闭（不拒绝）"
      >
        <X size={16} />
      </button>
    {/if}
  </div>

  <div class="card-body">
    {#if item.commandText}
      <div class="command-row">
        <code class="command">{item.commandText}</code>
        <button
          class="copy-btn"
          class:copied
          onclick={() => void copyCommand()}
          disabled={busy}
          aria-label="复制命令"
          title="复制命令"
        >
          {#if copied}<Check size={14} />{:else}<Copy size={14} />{/if}
        </button>
      </div>
    {/if}

    {#if item.argumentsSummary}
      <div class="section">
        <span class="section-label">参数摘要</span>
        <p class="section-text">{item.argumentsSummary}</p>
      </div>
    {/if}

    {#if item.reason}
      <div class="section">
        <span class="section-label">审批原因</span>
        <p class="section-text reason">{item.reason}</p>
      </div>
    {/if}
  </div>

  <div class="card-footer">
    <button class="reject-btn" onclick={onReject} disabled={busy}>拒绝</button>
    <button class="allow-btn" onclick={onAllow} disabled={busy}>
      {busy ? '处理中…' : '批准'}
    </button>
  </div>
</div>

<style>
  .approval-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 12px;
    box-shadow: var(--shadow);
    max-width: 480px;
  }
  .card-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .title-wrap {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
  }
  .time {
    font-size: 11px;
    color: var(--faint);
    font-family: var(--mono);
  }
  .close-btn {
    flex: none;
    border: 0;
    background: transparent;
    color: var(--muted);
    padding: 4px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .close-btn:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }
  .card-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .command-row {
    display: flex;
    align-items: stretch;
    gap: 8px;
  }
  .command {
    flex: 1;
    min-width: 0;
    padding: 8px 10px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--text);
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
    word-break: break-all;
    line-height: 1.5;
  }
  .copy-btn {
    flex: none;
    display: grid;
    place-items: center;
    width: 32px;
    border: 1px solid var(--line-strong);
    background: transparent;
    color: var(--muted);
    border-radius: 6px;
    cursor: pointer;
  }
  .copy-btn:hover:not(:disabled) {
    color: var(--amber);
    border-color: var(--amber-line);
  }
  .copy-btn.copied {
    color: var(--green);
    border-color: var(--green);
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .section-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--faint);
  }
  .section-text {
    margin: 0;
    font-size: 13px;
    line-height: 1.6;
    color: var(--muted);
  }
  .section-text.reason {
    color: var(--text);
  }
  .card-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding-top: 4px;
  }
  .reject-btn,
  .allow-btn {
    padding: 7px 16px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
  }
  .reject-btn {
    background: transparent;
    border: 1px solid var(--line-strong);
    color: var(--muted);
  }
  .reject-btn:hover:not(:disabled) {
    color: var(--text);
    background: var(--surface-3);
  }
  .allow-btn {
    background: var(--amber);
    border: 1px solid var(--amber);
    color: #1a1408;
    font-weight: 600;
  }
  .allow-btn:hover:not(:disabled) {
    background: var(--amber-hi);
  }
  .reject-btn:disabled,
  .allow-btn:disabled,
  .close-btn:disabled,
  .copy-btn:disabled {
    opacity: 0.55;
    cursor: default;
  }
</style>
