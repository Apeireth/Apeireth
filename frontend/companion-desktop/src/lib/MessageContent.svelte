<script lang="ts">
  import {Copy, Check, Pencil, RotateCw, GitFork, Terminal, X, ArrowUp} from 'lucide-svelte';
  import {renderMarkdown} from './markdown';
  import TaskCard from '../components/TaskCard.svelte';
  import ExecutionTimeline from '../components/ExecutionTimeline.svelte';
  import ToolCallCard from './components/ToolCallCard.svelte';
  import type {ChatMessage} from './types';

  let {
    message,
    onOpenTask,
    onRetry,
    onEditSave,
    onEditAndRegenerate,
    onBranch,
  }: {
    message: ChatMessage;
    onOpenTask?: (taskId: string) => void;
    onRetry?: (messageId: string) => void;
    onEditSave?: (messageId: string, newText: string) => void;
    onEditAndRegenerate?: (messageId: string, newText: string) => void;
    onBranch?: (messageId: string) => void;
  } = $props();

  let copied = $state(false);
  let isEditing = $state(false);
  let editDraft = $state(message.text || '');

  const role = $derived(message.role);
  const text = $derived(message.text || '');
  const streaming = $derived(!!message.streaming);
  const html = $derived(text ? renderMarkdown(text) : '');

  async function copyText() {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    } catch {
      // ignore
    }
  }

  function handleCancelEdit() {
    isEditing = false;
    editDraft = text;
  }

  function handleSaveOnly() {
    const trimmed = editDraft.trim();
    if (!trimmed) return;
    onEditSave?.(message.id, trimmed);
    isEditing = false;
  }

  function handleSaveAndResend() {
    const trimmed = editDraft.trim();
    if (!trimmed) return;
    onEditAndRegenerate?.(message.id, trimmed);
    isEditing = false;
  }
</script>

<div class="message-wrapper" class:user={role === 'user'} class:assistant={role === 'assistant'} class:system={role === 'system'}>
  {#if role === 'system'}
    <div class="system-message">
      <span class="system-icon"><Terminal size={12} /></span>
      <span class="system-text">{text}</span>
    </div>
  {:else}
    {#if message.events?.length}
      <ExecutionTimeline events={message.events} streaming={streaming} />
    {/if}

    {#if message.toolCalls?.length}
      <div class="tool-calls-container">
        {#each message.toolCalls as toolCall (toolCall.id)}
          <ToolCallCard {toolCall} />
        {/each}
      </div>
    {/if}

    {#if role === 'assistant'}
      {#if text}
        <!-- 他的声音（规范 §5.3/§6.1）：衬线引语级排版 .ap-voice；流式光标 .md-caret -->
        <div class="md-body ap-voice" class:streaming>
          {@html html}
          {#if streaming}
            <span class="md-caret" aria-hidden="true"></span>
          {/if}
        </div>
      {:else if streaming && !message.error && !message.toolCalls?.length}
        <!-- 正在输入 = 呼吸的金色小光环（规范 §5.3，motion.breathe ✅ 2.8s），禁止三个点 -->
        <div class="presence-halo" role="status" aria-label="他正在组织语言"><i></i></div>
      {/if}

      {#if message.taskCard}
        <TaskCard card={message.taskCard} onOpen={onOpenTask} />
      {/if}

      {#if message.error}
        <p class="message-error" role="alert">{message.error}</p>
      {/if}

      {#if !streaming && (text || message.error)}
        <div class="message-toolbar">
          <button class="tool-icon-btn" onclick={copyText} title="复制内容" aria-label="复制">
            {#if copied}<Check size={12} class="green" />{:else}<Copy size={12} />{/if}
            <span class="btn-text">{copied ? '已复制' : '复制'}</span>
          </button>

          {#if onRetry}
            <button class="tool-icon-btn" onclick={() => onRetry?.(message.id)} title="重新生成回复" aria-label="重试">
              <RotateCw size={12} />
              <span class="btn-text">重新生成</span>
            </button>
          {/if}

          {#if onBranch}
            <button class="tool-icon-btn" onclick={() => onBranch?.(message.id)} title="从此条消息创建新分支会话" aria-label="分支">
              <GitFork size={12} />
              <span class="btn-text">创建分支</span>
            </button>
          {/if}

          {#if message.modelInfo?.id}
            <span class="model-tag">{message.modelInfo.id}</span>
          {/if}
        </div>
      {/if}
    {:else}
      <div class="user-bubble" class:editing={isEditing}>
        {#if isEditing}
          <div class="user-edit-box">
            <textarea
              bind:value={editDraft}
              rows="3"
              class="user-edit-textarea"
              placeholder="修改消息内容..."
              onkeydown={(e) => {
                if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  handleSaveAndResend();
                } else if (e.key === 'Escape') {
                  handleCancelEdit();
                }
              }}
            ></textarea>
            <div class="user-edit-actions">
              <span class="edit-hint">Ctrl+Enter 重新发送 · Esc 取消</span>
              <div class="edit-btn-group">
                <button class="edit-btn cancel" onclick={handleCancelEdit}>取消</button>
                <button class="edit-btn save" onclick={handleSaveOnly}>仅保存</button>
                <button class="edit-btn resend" onclick={handleSaveAndResend}>
                  <ArrowUp size={12} />
                  <span>保存并重新生成</span>
                </button>
              </div>
            </div>
          </div>
        {:else}
          <div class="user-text md-body user-md">
            {@html html}
          </div>
          <div class="user-actions-bar">
            <button class="user-action-btn" onclick={() => { isEditing = true; editDraft = text; }} title="编辑消息" aria-label="编辑">
              <Pencil size={11} />
            </button>
            <button class="user-action-btn" onclick={copyText} title="复制" aria-label="复制">
              {#if copied}<Check size={11} class="green" />{:else}<Copy size={11} />{/if}
            </button>
            {#if onBranch}
              <button class="user-action-btn" onclick={() => onBranch?.(message.id)} title="从此处分支出新会话" aria-label="分支">
                <GitFork size={11} />
              </button>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .message-wrapper {
    display: flex;
    flex-direction: column;
    width: 100%;
  }
  .system-message {
    align-self: center;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    color: var(--faint);
    font-size: 11px;
    font-family: var(--mono);
    margin: 6px 0;
  }
  .user-bubble {
    position: relative;
    display: block;
    width: 100%;
  }
  .user-bubble.editing {
    width: 100%;
  }

  .user-edit-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 14px;
    background: rgba(255, 255, 255, 0.96);
    border: 1px solid var(--ap-gold);
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
  }
  .user-edit-textarea {
    width: 100%;
    border: 1px solid rgba(0, 0, 0, 0.15);
    border-radius: 6px;
    padding: 8px 10px;
    font-family: var(--ap-font-ui);
    font-size: 13.5px;
    line-height: 1.6;
    color: #1a1a1e;
    background: #fff;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }
  .user-edit-textarea:focus {
    border-color: var(--ap-gold-ui);
    box-shadow: 0 0 0 2px rgba(255, 210, 122, 0.25);
  }
  .user-edit-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
  }
  .edit-hint {
    font-size: 11px;
    color: rgba(38, 38, 42, 0.5);
  }
  .edit-btn-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .edit-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11.5px;
    cursor: pointer;
    font-family: var(--ap-font-ui);
    border: 1px solid transparent;
    transition: all 0.15s ease;
  }
  .edit-btn.cancel {
    background: transparent;
    border-color: rgba(0, 0, 0, 0.15);
    color: #555;
  }
  .edit-btn.cancel:hover {
    background: rgba(0, 0, 0, 0.05);
    color: #222;
  }
  .edit-btn.save {
    background: rgba(0, 0, 0, 0.06);
    border-color: rgba(0, 0, 0, 0.18);
    color: #222;
  }
  .edit-btn.save:hover {
    background: rgba(0, 0, 0, 0.12);
  }
  .edit-btn.resend {
    background: #242428;
    color: #f5f5f7;
    border-color: #111;
  }
  .edit-btn.resend:hover {
    background: #000;
    color: #ffd27a;
  }

  .user-text {
    padding: 10px 14px;
    margin: 0;
    line-height: 1.7;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .user-actions-bar {
    position: absolute;
    bottom: -22px;
    right: 4px;
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.15s ease;
    background: rgba(11, 13, 18, 0.85);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--ap-line);
    backdrop-filter: blur(8px);
    z-index: 5;
  }
  .user-bubble:hover .user-actions-bar {
    opacity: 1;
  }
  .user-action-btn {
    border: 0;
    background: transparent;
    color: rgba(232, 224, 204, 0.6);
    padding: 2px 4px;
    cursor: pointer;
    border-radius: 3px;
    display: inline-flex;
    align-items: center;
    transition: all 0.15s ease;
  }
  .user-action-btn:hover {
    color: var(--ap-gold);
    background: rgba(255, 210, 122, 0.12);
  }
  .message-toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 6px;
    padding-top: 4px;
  }
  .tool-icon-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 0;
    background: transparent;
    color: var(--faint);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    transition: color 0.15s ease;
  }
  .tool-icon-btn:hover {
    color: var(--muted);
    background: var(--surface-2);
  }
  .model-tag {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
  }
  :global(.green) {
    color: var(--green);
  }

  /* ---------- 他的声音排版语境（规范 §6.1/.ap-voice 承载于 md-body） ----------
     引语级参数（衬线/2.1 行高/0.13em 字距/31ch 行宽）由 tokens.css 的 .ap-voice 给出；
     此处只做语境化回落：代码与表格按 §6.3 纪律回到等宽/UI 栈、字距归零，
     列表与引用收紧行高以保持对话密度。 */
  .md-body.ap-voice :global(.md-list),
  .md-body.ap-voice :global(.md-quote) {
    line-height: 1.9;
  }
  .md-body.ap-voice :global(.md-table) {
    font-family: var(--ap-font-ui);
    letter-spacing: 0;
    line-height: 1.6;
    font-size: 13px;
  }
  .md-body.ap-voice :global(.md-code) {
    letter-spacing: 0;
    max-width: 100%;
  }
  .md-body.ap-voice :global(.md-inline) {
    letter-spacing: 0.01em;
  }
  .md-body.ap-voice :global(.md-h) {
    letter-spacing: 0.08em;
  }

  /* ---------- 正在输入的呼吸光环（规范 §5.3；数值 = motion.breathe ✅ index.html:32-36） ----------
     与 SceneLayer 的 pulse 元素同一语言：铂白径向微光 + 金环，2.8s 呼吸。 */
  .presence-halo {
    display: flex;
    align-items: center;
    height: 42px;
  }
  .presence-halo i {
    display: block;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid rgba(255, 210, 122, 0.85);
    background: radial-gradient(closest-side, rgba(255, 243, 214, 0.35), rgba(255, 243, 214, 0) 72%);
    box-shadow: 0 0 14px rgba(255, 210, 122, 0.5), inset 0 0 5px rgba(255, 243, 214, 0.45);
    animation: ap-halo-breathe 2.8s ease-in-out infinite;
  }
  @keyframes ap-halo-breathe {
    0%,
    100% {
      opacity: 0.22;
      transform: scale(1);
    }
    50% {
      opacity: 0.65;
      transform: scale(1.09);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .presence-halo i {
      animation: none;
      opacity: 0.45;
    }
  }

  /* 错误行（随本波对话重构补齐，此前全库零定义） */
  .message-error {
    margin: 6px 0 0;
    font-size: 12px;
    line-height: 1.6;
    color: var(--ap-semantic-danger);
  }
</style>
