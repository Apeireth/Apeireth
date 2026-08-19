<script lang="ts">
  import {ChevronDown, ChevronRight, RotateCcw, Sparkles} from 'lucide-svelte';
  import {renderMarkdown} from '../../lib/markdown';
  import TaskCard from '../../components/TaskCard.svelte';
  import ExecutionTimeline from '../../components/ExecutionTimeline.svelte';
  import type {ChatMessage} from '../../lib/types';

  let {
    message,
    onOpenTask,
    onRetry,
  }: {
    message: ChatMessage;
    onOpenTask?: (taskId: string) => void;
    onRetry?: () => void;
  } = $props();

  let reasoningOpen = $state(false);

  // Track text/role explicitly so in-place streaming mutations always re-render markdown.
  const role = $derived(message.role);
  const text = $derived(message.text || '');
  const reasoning = $derived(message.reasoning || '');
  const streaming = $derived(!!message.streaming);
  const html = $derived(role === 'assistant' && text ? renderMarkdown(text) : '');
  const reasoningHtml = $derived(reasoning ? renderMarkdown(reasoning) : '');

  // Automatically keep reasoning open while streaming reasoning, collapse once text starts
  $effect(() => {
    if (streaming && reasoning && !text) {
      reasoningOpen = true;
    }
  });

  function formatDuration(ms?: number): string {
    if (!ms) return '';
    return `${(ms / 1000).toFixed(1)}s`;
  }
</script>

{#if message.events?.length}
  <ExecutionTimeline events={message.events} {streaming} />
{/if}

{#if role === 'assistant'}
  {#if reasoning}
    <div class="reasoning-box">
      <button
        type="button"
        class="reasoning-toggle"
        onclick={() => reasoningOpen = !reasoningOpen}
        aria-expanded={reasoningOpen}
      >
        <span class="reasoning-title">
          {#if reasoningOpen}
            <ChevronDown size={13} />
          {:else}
            <ChevronRight size={13} />
          {/if}
          <Sparkles size={13} class="exec-icon-accent" />
          <span>{streaming && !text ? '思考中…' : '已深度思考'}</span>
        </span>
        {#if message.reasoningDurationMs}
          <span class="reasoning-duration">{formatDuration(message.reasoningDurationMs)}</span>
        {/if}
      </button>
      {#if reasoningOpen}
        <div class="reasoning-content md-body">
          {@html reasoningHtml}
          {#if streaming && !text}<span class="md-caret" aria-hidden="true"></span>{/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if text}
    <div class="md-body" class:streaming>
      {@html html}
      {#if streaming}<span class="md-caret" aria-hidden="true"></span>{/if}
    </div>
  {:else if streaming && !reasoning && !message.error}
    <div class="typing" aria-label="正在生成"><i></i><i></i><i></i></div>
  {/if}

  {#if message.aborted}
    <span class="message-aborted">生成已中断</span>
  {/if}

  {#if message.taskCard}
    <TaskCard card={message.taskCard} onOpen={onOpenTask} />
  {/if}

  {#if message.error}
    <p class="message-error" role="alert">{message.error}</p>
    {#if onRetry}
      <div class="message-retry-row">
        <button type="button" class="message-retry-btn" onclick={onRetry}>
          <RotateCcw size={12} />重试
        </button>
      </div>
    {/if}
  {/if}
{:else}
  <p class="user-text">{text}</p>
{/if}
