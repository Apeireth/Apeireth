<script lang="ts">
  import {onMount} from 'svelte';
  import {MessageCircleMore, Settings, MessagesSquare, Layers3, Plus, ArrowUp, Trash2, Loader2, Plug} from 'lucide-svelte';
  import MessageContent from './lib/MessageContent.svelte';
  import ConversationsView from './lib/ConversationsView.svelte';
  import MemoryView from './lib/MemoryView.svelte';
  import PageHeader from './lib/PageHeader.svelte';
  import StatusDot from './lib/StatusDot.svelte';
  import type {ApeirethConfig, ChatMessage, Conversation, ViewId} from './lib/types';
  import {
    checkHealth,
    listModels,
    loadConfig,
    loadConversations,
    saveConfig,
    saveConversations,
    streamChat,
  } from './lib/runtime';

  const nav = [
    {id: 'chat', label: '对话', icon: MessageCircleMore},
    {id: 'conversations', label: '会话', icon: MessagesSquare},
    {id: 'memory', label: '记忆', icon: Layers3},
    {id: 'settings', label: '设置', icon: Settings},
  ] as const;

  let activeView = $state<ViewId>('chat');
  let config = $state<ApeirethConfig>(loadConfig());
  let conversations = $state<Conversation[]>(loadConversations());
  let activeId = $state<string | null>(null);
  let draft = $state('');
  let busy = $state(false);
  let connected = $state(false);
  let error = $state('');
  let abortController = $state<AbortController | null>(null);

  // 设置视图临时值 (初始从持久化配置读取, 编辑期间独立)
  let editBaseUrl = $state(loadConfig().baseUrl);
  let editApiKey = $state(loadConfig().apiKey);
  let editModel = $state(loadConfig().model);
  let modelsList = $state<string[]>([]);

  const activeConversation = $derived(
    conversations.find((item) => item.id === activeId) || null,
  );

  const activeMessages = $derived(activeConversation?.messages || []);

  function ensureConversation(): Conversation {
    if (activeConversation) return activeConversation;
    const now = Date.now();
    const conversation: Conversation = {
      id: crypto.randomUUID(),
      title: '新对话',
      createdAt: now,
      updatedAt: now,
      messages: [],
      scope: 'global',
    };
    conversations = [conversation, ...conversations];
    activeId = conversation.id;
    persist();
    return conversation;
  }

  function persist(): void {
    saveConversations(conversations);
  }

  function updateConversation(id: string, patch: Partial<Conversation>): void {
    conversations = conversations.map((item) => item.id === id ? {...item, ...patch, updatedAt: Date.now()} : item);
    persist();
  }

  function updateMessage(id: string, messageId: string, patch: Partial<ChatMessage>): void {
    if (!activeConversation) return;
    updateConversation(id, {
      messages: activeConversation.messages.map((m) => m.id === messageId ? {...m, ...patch} : m),
    });
  }

  function pushMessage(conversationId: string, message: ChatMessage): void {
    const conv = conversations.find((item) => item.id === conversationId);
    if (!conv) return;
    updateConversation(conversationId, {messages: [...conv.messages, message]});
  }

  async function refreshConnection(): Promise<void> {
    connected = await checkHealth(config.baseUrl);
  }

  async function send(): Promise<void> {
    const text = draft.trim();
    if (!text || busy) return;
    const conversation = ensureConversation();
    const conversationId = conversation.id;
    const history = conversation.messages
      .filter((m) => m.role === 'user' || m.role === 'assistant')
      .map((m) => ({role: m.role, content: m.text}));

    draft = '';
    busy = true;
    error = '';

    const userMessage: ChatMessage = {id: crypto.randomUUID(), role: 'user', text, time: new Date().toLocaleTimeString('zh-CN')};
    const assistantMessage: ChatMessage = {id: crypto.randomUUID(), role: 'assistant', text: '', time: new Date().toLocaleTimeString('zh-CN'), streaming: true};

    pushMessage(conversationId, userMessage);
    pushMessage(conversationId, assistantMessage);

    // 自动起标题 (首条)
    if (conversation.messages.length <= 2) {
      updateConversation(conversationId, {title: text.slice(0, 24)});
    }

    abortController = new AbortController();
    try {
      const full = await streamChat(
        config,
        [...history, {role: 'user', content: text}],
        (delta) => {
          const current = conversations.find((item) => item.id === conversationId);
          if (current) {
            updateConversation(conversationId, {
              messages: current.messages.map((m) => m.id === assistantMessage.id ? {...m, text: m.text + delta} : m),
            });
          }
        },
        abortController.signal,
      );
      updateMessage(conversationId, assistantMessage.id, {text: full || '(空响应)', streaming: false});
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      error = message;
      updateMessage(conversationId, assistantMessage.id, {text: '', streaming: false, error: message});
    } finally {
      busy = false;
      abortController = null;
    }
  }

  function stop(): void {
    abortController?.abort();
  }

  function newConversation(): void {
    const now = Date.now();
    const conversation: Conversation = {
      id: crypto.randomUUID(),
      title: '新对话',
      createdAt: now,
      updatedAt: now,
      messages: [],
      scope: 'global',
    };
    conversations = [conversation, ...conversations];
    activeId = conversation.id;
    activeView = 'chat';
    persist();
  }

  function openConversation(id: string): void {
    activeId = id;
    activeView = 'chat';
  }

  function archiveConversation(id: string): void {
    const conv = conversations.find((item) => item.id === id);
    if (conv) updateConversation(id, {archived: !conv.archived});
  }

  function deleteConversation(id: string): void {
    conversations = conversations.filter((item) => item.id !== id);
    if (activeId === id) activeId = null;
    persist();
  }

  async function saveSettings(): Promise<void> {
    config = {...config, baseUrl: editBaseUrl.trim(), apiKey: editApiKey.trim(), model: editModel.trim()};
    saveConfig(config);
    connected = await checkHealth(config.baseUrl);
    if (connected) {
      try {
        modelsList = await listModels(config.baseUrl, config.apiKey);
      } catch {
        modelsList = [];
      }
    }
  }

  async function loadModels(): Promise<void> {
    try {
      modelsList = await listModels(config.baseUrl, config.apiKey);
    } catch (caught) {
      modelsList = [];
      error = caught instanceof Error ? caught.message : String(caught);
    }
  }

  onMount(() => {
    if (!activeId && conversations.length) activeId = conversations[0].id;
    void refreshConnection();
  });
</script>

<div class="shell">
  <aside class="sidebar">
    <div class="sidebar-brand">
      <span class="logo-mark">A</span>
      <span class="brand-name">Apeireth 伙伴</span>
      <StatusDot size="small" off={!connected} />
    </div>
    <nav class="nav">
      {#each nav as item}
        <button class:active={activeView === item.id} onclick={() => activeView = item.id}>
          <item.icon size={17} />
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>
    <div class="sidebar-footer">
      <button class="quiet-button wide" onclick={newConversation}><Plus size={14}/>新对话</button>
      <div class="conn-hint">
        <Plug size={12} />
        <span>{connected ? '后端已连接' : '后端未连接'}</span>
      </div>
    </div>
  </aside>

  <main class="main">
    {#if activeView === 'chat'}
      <section class="chat-view">
        <header class="chat-header">
          <div>
            <h1>{activeConversation?.title || '新对话'}</h1>
            <small>{config.model} · {config.baseUrl}</small>
          </div>
          <div class="chat-header-actions">
            {#if busy}
              <button class="text-action" onclick={stop}><Loader2 size={14}/>停止</button>
            {/if}
            <button class="text-action" onclick={newConversation}><Plus size={14}/>新对话</button>
          </div>
        </header>

        <div class="messages">
          {#if !activeMessages.length}
            <div class="blank-state">
              <div class="blank-mark">⌁</div>
              <h3>开始对话</h3>
              <p>连接 Apeireth 后端后，在这里与伙伴对话。记忆、工具与宪法评审由后端负责。</p>
            </div>
          {:else}
            {#each activeMessages as message}
              <article class="message-row" class:user={message.role === 'user'} class:assistant={message.role === 'assistant'}>
                <div class="message-avatar">{message.role === 'user' ? '主' : 'A'}</div>
                <div class="message-body">
                  <MessageContent message={message} />
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
                void send();
              }
            }}
          ></textarea>
          <button class="primary-button send-button" onclick={send} disabled={busy || !draft.trim()} aria-label="发送">
            {#if busy}<Loader2 size={18}/>{:else}<ArrowUp size={18}/>{/if}
          </button>
        </footer>
      </section>
    {:else if activeView === 'conversations'}
      <ConversationsView
        conversations={conversations}
        activeId={activeId || ''}
        onOpen={openConversation}
        onCreate={newConversation}
        onArchive={archiveConversation}
        onDelete={deleteConversation}
      />
    {:else if activeView === 'memory'}
      <MemoryView {config} />
    {:else}
      <section class="view">
        <PageHeader eyebrow="配置" title="后端连接" subtitle="连接 Apeireth 的 OpenAI 兼容端点 (companion_serve :8090 或 apeireth-api :8080)。">
          <button class="primary-button" onclick={saveSettings}><Settings size={14}/>保存</button>
        </PageHeader>
        <div class="settings-form">
          <label>
            <span>端点地址</span>
            <input bind:value={editBaseUrl} placeholder="http://127.0.0.1:8090" />
          </label>
          <label>
            <span>API Key</span>
            <input bind:value={editApiKey} type="password" placeholder="后端持有的 key (任意非空串)" />
          </label>
          <label>
            <span>模型</span>
            <div class="model-row">
              <input bind:value={editModel} placeholder="MiniMax-M3" />
              <button class="quiet-button" onclick={loadModels} title="拉取模型列表">刷新</button>
            </div>
          </label>
          {#if modelsList.length}
            <div class="model-list">
              {#each modelsList as model}
                <button class="model-chip" onclick={() => editModel = model}>{model}</button>
              {/each}
            </div>
          {/if}
          <div class="conn-status">
            <StatusDot size="small" off={!connected} />
            <span>{connected ? '已连接' : '未连接'} · {config.baseUrl}</span>
          </div>
          {#if error}
            <p class="error-banner" role="alert">{error}</p>
          {/if}
        </div>
      </section>
    {/if}
  </main>
</div>
