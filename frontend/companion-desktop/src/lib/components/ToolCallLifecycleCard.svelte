<script lang="ts">
  import {ChevronDown, ChevronRight} from 'lucide-svelte';

  interface ToolCallLifecycleCardProps {
    call: {
      id: string;
      name: string;
      status: 'pending' | 'running' | 'success' | 'error';
      latencyMs?: number;
      error?: string;
      arguments?: unknown;
      result?: unknown;
    };
  }

  let {call}: ToolCallLifecycleCardProps = $props();

  let expanded = $state(false);

  const STATUS_META: Record<
    ToolCallLifecycleCardProps['call']['status'],
    {label: string; dotClass: string}
  > = {
    pending: {label: '待执行', dotClass: 'pending'},
    running: {label: '执行中', dotClass: 'running'},
    success: {label: '成功', dotClass: 'success'},
    error: {label: '失败', dotClass: 'error'},
  };

  const statusMeta = $derived(STATUS_META[call.status] ?? STATUS_META.pending);

  function formatLatency(ms?: number): string {
    if (ms === undefined || Number.isNaN(ms)) return '';
    if (ms < 1000) return `${Math.round(ms)}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  }

  /** JSON.stringify 并截断超长内容，避免大对象撑爆布局。 */
  function stringify(value: unknown, limit = 4000): string {
    if (value === undefined) return '';
    let text: string;
    try {
      text = typeof value === 'string' ? value : JSON.stringify(value, null, 2);
    } catch {
      text = String(value);
    }
    if (text.length > limit) {
      return `${text.slice(0, limit)}…（已截断，共 ${text.length} 字符）`;
    }
    return text;
  }
</script>

<div class="tool-lifecycle-card" class:failed={call.status === 'error'}>
  <div
    class="tool-header"
    role="button"
    tabindex="0"
    onclick={() => (expanded = !expanded)}
    onkeydown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') expanded = !expanded;
    }}
    aria-expanded={expanded}
  >
    <div class="tool-title-group">
      <span class="status-dot {statusMeta.dotClass}" aria-hidden="true"></span>
      <strong class="tool-name">{call.name}</strong>
      <span class="status-label {statusMeta.dotClass}">{statusMeta.label}</span>
      {#if call.latencyMs !== undefined}
        <span class="latency">{formatLatency(call.latencyMs)}</span>
      {/if}
    </div>
    <button class="expand-btn" aria-label={expanded ? '收起详情' : '展开详情'}>
      {#if expanded}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
    </button>
  </div>

  {#if expanded}
    <div class="tool-body">
      {#if call.error}
        <div class="error-section">
          <span class="section-label">错误信息</span>
          <p class="error-text">{call.error}</p>
        </div>
      {/if}

      {#if call.arguments !== undefined}
        <div class="section">
          <span class="section-label">参数</span>
          <pre class="code-block">{stringify(call.arguments)}</pre>
        </div>
      {/if}

      {#if call.result !== undefined}
        <div class="section">
          <span class="section-label">结果</span>
          <pre class="code-block">{stringify(call.result)}</pre>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tool-lifecycle-card {
    margin: 8px 0;
    border: 1px solid var(--line);
    background: var(--surface-2);
    border-radius: 8px;
    overflow: hidden;
    transition: border-color 0.15s ease;
  }
  .tool-lifecycle-card:hover {
    border-color: var(--line-strong);
  }
  .tool-lifecycle-card.failed {
    border-color: rgba(224, 91, 80, 0.3);
  }
  .tool-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    cursor: pointer;
    user-select: none;
    background: var(--surface-2);
  }
  .tool-header:hover {
    background: var(--surface-3);
  }
  .tool-title-group {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    min-width: 0;
  }
  .tool-name {
    font-family: var(--mono);
    color: var(--text);
    font-weight: 600;
  }
  .status-dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .status-dot.pending {
    background: var(--faint);
  }
  .status-dot.running {
    background: var(--blue);
    animation: blink 1s ease-in-out infinite;
  }
  .status-dot.success {
    background: var(--green);
  }
  .status-dot.error {
    background: var(--danger);
  }
  .status-label {
    display: inline-flex;
    align-items: center;
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 11px;
  }
  .status-label.pending {
    background: var(--surface-3);
    color: var(--muted);
  }
  .status-label.running {
    background: var(--blue-wash);
    color: var(--blue);
  }
  .status-label.success {
    background: var(--green-wash);
    color: var(--green);
  }
  .status-label.error {
    background: rgba(224, 91, 80, 0.12);
    color: var(--danger);
  }
  .latency {
    color: var(--faint);
    font-family: var(--mono);
    font-size: 11px;
  }
  .expand-btn {
    border: 0;
    background: transparent;
    color: var(--muted);
    padding: 2px;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .tool-body {
    padding: 10px 14px 12px;
    border-top: 1px solid var(--line);
    background: var(--surface);
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-size: 12px;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .section-label {
    display: block;
    color: var(--faint);
    font-size: 11px;
    font-weight: 500;
  }
  .error-section {
    border-left: 2px solid var(--danger);
    padding-left: 8px;
  }
  .error-text {
    margin: 0;
    color: var(--danger);
    line-height: 1.5;
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
    max-height: 220px;
    overflow-y: auto;
  }
  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.35; }
  }
</style>
