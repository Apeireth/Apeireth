<script lang="ts">
  import {renderMarkdown} from './markdown';
  import TaskCard from './TaskCard.svelte';
  import ExecutionTimeline from './ExecutionTimeline.svelte';
  import type {ChatMessage} from './types';

  let {
    message,
    onOpenTask,
  }: {
    message: ChatMessage;
    onOpenTask?: (taskId: string) => void;
  } = $props();

  // Track text/role explicitly so in-place streaming mutations always re-render markdown.
  const role = $derived(message.role);
  const text = $derived(message.text || '');
  const streaming = $derived(!!message.streaming);
  const html = $derived(role === 'assistant' && text ? renderMarkdown(text) : '');
</script>

{#if message.events?.length}
  <ExecutionTimeline events={message.events} streaming={streaming} />
{/if}

{#if role === 'assistant'}
  {#if text}
    <div class="md-body" class:streaming>{@html html}{#if streaming}<span class="md-caret" aria-hidden="true"></span>{/if}</div>
  {:else if streaming && !message.error}
    <div class="typing" aria-label="正在生成"><i></i><i></i><i></i></div>
  {/if}
  {#if message.taskCard}
    <TaskCard card={message.taskCard} onOpen={onOpenTask} />
  {/if}
  {#if message.error}
    <p class="message-error" role="alert">{message.error}</p>
  {/if}
{:else}
  <p class="user-text">{text}</p>
{/if}
