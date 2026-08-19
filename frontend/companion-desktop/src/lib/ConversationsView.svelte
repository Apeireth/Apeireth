<script lang="ts">
  import {Archive, MessageSquarePlus, Trash2} from 'lucide-svelte';
  import PageHeader from './PageHeader.svelte';
  import type {Conversation} from './types';

  let {conversations, activeId, onOpen, onCreate, onArchive, onDelete}: {
    conversations: Conversation[];
    activeId: string;
    onOpen: (id: string) => void;
    onCreate: () => void;
    onArchive: (id: string) => void;
    onDelete: (id: string) => void;
  } = $props();

  let showArchived = $state(false);
  const visible = $derived(conversations.filter((item) => showArchived ? item.archived : !item.archived));
  const preview = (item: Conversation) => item.messages.at(-1)?.text?.replace(/\s+/g, ' ').slice(0, 72) || '尚未开始';
</script>

<section class="view">
  <PageHeader eyebrow="工作区" title="对话管理" subtitle="全局与项目对话分开保存；归档不会删除本地记录。">
    <button class="quiet-button" onclick={() => showArchived = !showArchived}><Archive size={14}/>{showArchived ? '查看当前' : '查看归档'}</button>
    <button class="primary-button" onclick={onCreate}><MessageSquarePlus size={14}/>新对话</button>
  </PageHeader>
  <div class="conversation-list">
    {#each visible as conversation}
      <article class:active={conversation.id === activeId}>
        <button class="conversation-open" onclick={() => onOpen(conversation.id)}>
          <div>
            <strong>{conversation.title}</strong>
            <span class="badge" class:blue={conversation.scope !== 'project'} class:amber={conversation.scope === 'project'}>{conversation.scope === 'project' ? '项目' : '全局'}</span>
            <time>{new Date(conversation.updatedAt).toLocaleString('zh-CN', {month:'numeric', day:'numeric', hour:'2-digit', minute:'2-digit'})}</time>
          </div>
          <p>{preview(conversation)}</p>
          <small>{conversation.messages.length} 条消息</small>
        </button>
        <div class="conversation-actions">
          <button title={conversation.archived ? '恢复' : '归档'} aria-label={conversation.archived ? '恢复' : '归档'} onclick={() => onArchive(conversation.id)}><Archive size={14}/></button>
          <button title="删除" aria-label="删除" onclick={() => onDelete(conversation.id)}><Trash2 size={14}/></button>
        </div>
      </article>
    {:else}
      <div class="blank-state"><div class="blank-mark">⌁</div><h3>没有{showArchived ? '归档' : '当前'}对话</h3><p>新建一个对话，开始独立的上下文。</p></div>
    {/each}
  </div>
</section>
