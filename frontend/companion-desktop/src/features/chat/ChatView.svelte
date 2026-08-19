<script lang="ts">
  import {ArrowUp, Loader2, Plus} from 'lucide-svelte';
  import MessageContent from './MessageContent.svelte';
  import type {ApeirethConfig, ChatMessage, Conversation} from '../../lib/types';

  let {
    config,
    conversation,
    messages,
    draft = $bindable(''),
    busy = false,
    error = '',
    onSend,
    onStop,
    onNewConversation,
  }: {
    config: ApeirethConfig;
    conversation: Conversation | null;
    messages: ChatMessage[];
    draft: string;
    busy: boolean;
    error: string;
    onSend: () => void;
    onStop: () => void;
    onNewConversation: () => void;
  } = $props();
</script>

<section class="chat-view">
  <header class="chat-header">
    <div>
      <h1>{conversation?.title || '新对话'}</h1>
      <small>{config.model} · {config.baseUrl}</small>
    </div>
    <div class="chat-header-actions">
      {#if busy}
        <button class="text-action" onclick={onStop}><Loader2 size={14}/>停止</button>
      {/if}
      <button class="text-action" onclick={onNewConversation}><Plus size={14}/>新对话</button>
    </div>
  </header>

  <div class="messages">
    {#if !messages.length}
      <div class="blank-state">
        <div class="blank-mark">⌁</div>
        <h3>开始对话</h3>
        <p>连接 Apeireth 后端后，在这里与伙伴对话。记忆、工具与宪法评审由后端负责。</p>
      </div>
    {:else}
      {#each messages as message}
        <article class="message-row" class:user={message.role === 'user'} class:assistant={message.role === 'assistant'}>
          <div class="message-avatar">{message.role === 'user' ? '主' : 'A'}</div>
          <div class="message-body">
            <MessageContent {message} />
          </div>
        </article>
      {/each}
    {/if}
    {#if error}
      <p class="error-banner" role="alert">{error}</p>
    {/if}
  </div>

  <footer class="composer">
    <textarea
      bind:value={draft}
      rows="3"
      placeholder="给阿佩瑞斯留言…… (Enter 发送, Shift+Enter 换行)"
      onkeydown={(event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
          event.preventDefault();
          onSend();
        }
      }}
    ></textarea>
    <button class="primary-button send-button" onclick={onSend} disabled={busy || !draft.trim()} aria-label="发送">
      {#if busy}<Loader2 size={18}/>{:else}<ArrowUp size={18}/>{/if}
    </button>
  </footer>
</section>
