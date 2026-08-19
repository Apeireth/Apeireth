<script lang="ts">
  import {
    AlertTriangle,
    CheckCircle2,
    ChevronDown,
    ChevronRight,
    CircleDot,
    Loader2,
    Sparkles,
    Terminal,
    Wrench,
    XCircle,
  } from 'lucide-svelte';
  import type {ChatMessageEvent} from './types';

  let {
    events = [],
    streaming = false,
  }: {
    events?: ChatMessageEvent[];
    /** When false, orphan running/pending non-task steps render as done (no forever spinner). */
    streaming?: boolean;
  } = $props();

  let open = $state(false);
  let expanded = $state<Record<string, boolean>>({});
  let userToggled = $state(false);

  function displayStatus(event: ChatMessageEvent): string | undefined {
    const status = event.status;
    if (!status) return streaming ? 'running' : 'done';
    if (status === 'awaiting_approval') return status;
    // Task-bound steps may keep updating after the chat stream ends.
    if (event.taskId && (status === 'running' || status === 'pending')) return status;
    if (!streaming && (status === 'running' || status === 'pending')) return 'done';
    return status;
  }

  const items = $derived(
    [...(events || [])]
      .filter((event) => ['tool', 'mcp', 'task', 'agent', 'error'].includes(event.kind))
      .sort((a, b) => (a.ts || 0) - (b.ts || 0)),
  );

  const resolved = $derived(
    items.map((event) => ({event, status: displayStatus(event)})),
  );

  const summary = $derived.by(() => {
    const total = resolved.length;
    if (!total) return '';
    const running = resolved.filter((item) => item.status === 'running' || item.status === 'pending').length;
    const failed = resolved.filter((item) => item.status === 'failed' || item.event.kind === 'error').length;
    const approval = resolved.filter((item) => item.status === 'awaiting_approval').length;
    if (running) return `正在处理 ${running} 项`;
    if (approval) return `${approval} 项待审批`;
    if (failed) return `${failed} 项失败 · 共 ${total} 项`;
    return `已完成 ${total} 项`;
  });

  $effect(() => {
    if (userToggled) return;
    const needsAttention = resolved.some(
      (item) => item.status === 'failed' || item.status === 'awaiting_approval' || item.event.kind === 'error',
    );
    // Keep open for live tool/task steps even after chat streaming ends (computer-use continues via task.updated).
    const active = resolved.some(
      (item) => item.status === 'running' || item.status === 'pending' || item.status === 'awaiting_approval',
    );
    if (needsAttention || active) open = true;
  });

  function kindLabel(kind: string) {
    const map: Record<string, string> = {
      tool: '工具',
      task: '任务',
      mcp: 'MCP',
      error: '错误',
      agent: 'Agent',
    };
    return map[kind] || kind;
  }

  function actionLabel(action?: string) {
    if (!action) return '';
    const map: Record<string, string> = {
      key: '按键',
      type: '输入',
      click: '点击',
      scroll: '滚动',
      launch: '启动',
      focus: '聚焦',
      screenshot: '截屏',
      foreground: '前台',
      uiaInvoke: 'UIA 调用',
      uiaSetValue: 'UIA 赋值',
      wait: '等待',
      done: '完成',
      fail: '失败',
      catalog: '工具目录',
    };
    return map[action] || action;
  }

  function statusLabel(status?: string) {
    const map: Record<string, string> = {
      pending: '等待',
      running: '进行中',
      done: '完成',
      failed: '失败',
      skipped: '已跳过',
      cancelled: '已取消',
      awaiting_approval: '待审批',
    };
    return status ? map[status] || status : '';
  }

  function iconFor(event: ChatMessageEvent, status?: string) {
    if (status === 'running' || status === 'pending') return Loader2;
    if (status === 'failed' || event.kind === 'error') return XCircle;
    if (status === 'awaiting_approval') return AlertTriangle;
    if (status === 'done' || status === 'skipped' || status === 'cancelled') return CheckCircle2;
    if (event.kind === 'mcp' || event.kind === 'tool') return Wrench;
    if (event.kind === 'agent') return Sparkles;
    if (event.kind === 'task') return Terminal;
    return CircleDot;
  }

  function timeText(ts?: number) {
    if (!ts) return '';
    return new Date(ts).toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit', second: '2-digit'});
  }

  function toggleOpen() {
    userToggled = true;
    open = !open;
  }

  function toggleReceipt(id: string) {
    expanded = {...expanded, [id]: !expanded[id]};
  }

  function isSpinning(status?: string) {
    return status === 'running' || status === 'pending';
  }
</script>

{#if items.length}
  <div class="exec-timeline" class:open aria-label="工作过程">
    <button type="button" class="exec-timeline-toggle" aria-expanded={open} onclick={toggleOpen}>
      {#if open}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
      <span class="exec-timeline-label">工作过程</span>
      <span class="exec-timeline-summary">{summary}</span>
    </button>
    {#if open}
      <ol class="exec-timeline-list">
        {#each resolved as {event, status} (event.id)}
          {@const Icon = iconFor(event, status)}
          <li class="exec-timeline-item" data-kind={event.kind} data-status={status || ''}>
            <div class="exec-timeline-rail" aria-hidden="true">
              <span class="exec-timeline-dot" class:spin={isSpinning(status)}>
                <Icon size={11} />
              </span>
            </div>
            <div class="exec-timeline-body">
              <div class="exec-timeline-meta">
                <span class="exec-kind">{kindLabel(event.kind)}</span>
                {#if status}
                  <span class="exec-status">{statusLabel(status)}</span>
                {/if}
                {#if typeof event.tier === 'number'}
                  <span class="exec-tier">T{event.tier}</span>
                {/if}
                {#if event.ts}
                  <time>{timeText(event.ts)}</time>
                {/if}
              </div>
              <p class="exec-timeline-text">
                {#if event.action}
                  <span class="exec-action">{actionLabel(event.action)}</span>
                  <span class="exec-sep">·</span>
                {/if}
                {event.text}
              </p>
              {#if event.receipt}
                <button type="button" class="text-action exec-receipt-toggle" onclick={() => toggleReceipt(event.id)}>
                  {expanded[event.id] ? '收起回执' : '查看回执'}
                </button>
                {#if expanded[event.id]}
                  <pre class="exec-timeline-receipt">{event.receipt}</pre>
                {/if}
              {/if}
            </div>
          </li>
        {/each}
      </ol>
    {/if}
  </div>
{/if}
