<script lang="ts">
  import {onMount} from 'svelte';
  import {Activity, Layers3, MessageCircleMore, MessagesSquare, Settings, Wrench} from 'lucide-svelte';
  import Sidebar from './app/Sidebar.svelte';
  import ChatView from './features/chat/ChatView.svelte';
  import ConversationsView from './features/conversations/ConversationsView.svelte';
  import ActivityCenterView from './features/activity/ActivityCenterView.svelte';
  import ToolRegistryView from './features/tools/ToolRegistryView.svelte';
  import MemoryView from './features/memory/MemoryView.svelte';
  import SettingsView from './features/settings/SettingsView.svelte';
  import type {ApeirethConfig, ApprovalRequest, ChatMessage, ChatMessageEvent, Conversation, HealthState, ViewId} from './lib/types';
  import {
    checkHealth,
    createAgentRuntime,
    fetchPendingApprovals,
    listModels,
    loadConfig,
    loadConversations,
    saveConfig,
    saveConversations,
  } from './lib/runtime';

  const nav = [
    {id: 'chat', label: '对话', icon: MessageCircleMore},
    {id: 'conversations', label: '会话', icon: MessagesSquare},
    {id: 'activity', label: '活动', icon: Activity},
    {id: 'tools', label: '工具', icon: Wrench},
    {id: 'memory', label: '记忆', icon: Layers3},
    {id: 'settings', label: '设置', icon: Settings},
  ] as const;

  let activeView = $state<ViewId>('chat');
  let config = $state<ApeirethConfig>(loadConfig());
  let conversations = $state<Conversation[]>(loadConversations());
  let activeId = $state<string | null>(null);
  let draft = $state('');
  let busy = $state(false);
  let error = $state('');
  let pendingApprovals = $state<ApprovalRequest[]>([]);
  // runtime 绑定当前配置; config 变更时在 saveSettings 重建
  let agentRuntime = $state(createAgentRuntime(loadConfig()));

  // Phase 5E: 真实 runtime health 状态 (connecting/ready/generating/error/offline)
  let healthState = $state<HealthState>('connecting');

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

  /** 按 id 原子更新某会话的一条消息 — 不依赖 activeConversation, 防 stale write. */
  function updateMessage(id: string, messageId: string, patch: Partial<ChatMessage>): void {
    conversations = conversations.map((item) => {
      if (item.id !== id) return item;
      return {
        ...item,
        updatedAt: Date.now(),
        messages: item.messages.map((m) => m.id === messageId ? {...m, ...patch} : m),
      };
    });
    persist();
  }

  /** 按 id 原子追加消息到指定会话. */
  function pushMessage(conversationId: string, message: ChatMessage): void {
    conversations = conversations.map((item) => {
      if (item.id !== conversationId) return item;
      return {...item, updatedAt: Date.now(), messages: [...item.messages, message]};
    });
    persist();
  }

  /** 按 id 原子拼接流式文本 delta. */
  function appendDelta(conversationId: string, messageId: string, delta: string): void {
    conversations = conversations.map((item) => {
      if (item.id !== conversationId) return item;
      return {
        ...item,
        updatedAt: Date.now(),
        messages: item.messages.map((m) => m.id === messageId ? {...m, text: m.text + delta} : m),
      };
    });
    persist();
  }

  /** 按 id 原子拼接推理思考 delta. */
  function appendReasoningDelta(conversationId: string, messageId: string, delta: string): void {
    conversations = conversations.map((item) => {
      if (item.id !== conversationId) return item;
      return {
        ...item,
        updatedAt: Date.now(),
        messages: item.messages.map((m) => m.id === messageId ? {...m, reasoning: (m.reasoning || '') + delta} : m),
      };
    });
    persist();
  }

  /** 按 id 原子追加或更新执行步骤事件. */
  function appendMessageEvent(conversationId: string, messageId: string, event: ChatMessageEvent): void {
    conversations = conversations.map((item) => {
      if (item.id !== conversationId) return item;
      return {
        ...item,
        updatedAt: Date.now(),
        messages: item.messages.map((m) => {
          if (m.id !== messageId) return m;
          const prevEvents = m.events || [];
          const existingIndex = prevEvents.findIndex((e) => e.id === event.id);
          if (existingIndex >= 0) {
            const updated = [...prevEvents];
            updated[existingIndex] = {...updated[existingIndex], ...event};
            return {...m, events: updated};
          }
          return {...m, events: [...prevEvents, event]};
        }),
      };
    });
    persist();
  }

  /** 真实 HTTP /health 检测与待审批请求轮询. */
  async function refreshConnection(): Promise<void> {
    const ok = await checkHealth(config.baseUrl);
    if (busy) {
      if (healthState === 'offline' || healthState === 'connecting') healthState = 'generating';
    } else {
      healthState = ok ? 'ready' : 'offline';
    }
    if (ok) {
      pendingApprovals = await fetchPendingApprovals(config);
    } else {
      pendingApprovals = [];
    }
  }

  async function send(customText?: string): Promise<void> {
    const text = (customText ?? draft).trim();
    if (!text || busy) return;
    const conversation = ensureConversation();
    const conversationId = conversation.id;
    const history = conversation.messages
      .filter((m) => m.role === 'user' || m.role === 'assistant')
      .map((m) => ({role: m.role, content: m.text}));

    if (!customText) draft = '';
    busy = true;
    healthState = 'generating';
    error = '';

    const userMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      text,
      time: new Date().toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'}),
    };
    const assistantMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'assistant',
      text: '',
      time: new Date().toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'}),
      streaming: true,
      reasoning: '',
      events: [],
    };

    pushMessage(conversationId, userMessage);
    pushMessage(conversationId, assistantMessage);

    // 自动起标题 (首条)
    if (conversation.messages.length <= 2) {
      updateConversation(conversationId, {title: text.slice(0, 24)});
    }

    const startTime = Date.now();
    let reasoningStartTime = startTime;
    let reasoningEndTime = 0;

    try {
      // UI 只面对 Agent Runtime Contract (§15) — 事件流驱动更新, 不裸碰 HTTP
      const full = await agentRuntime.run(
        {
          messages: [...history, {role: 'user', content: text}],
          model: {id: config.model, provider: 'apeireth'},
          sessionId: conversationId,
          context: {user: '主人'},
        },
        (event) => {
          if (event.type === 'text-delta') {
            if (!reasoningEndTime && reasoningStartTime) {
              reasoningEndTime = Date.now();
              const duration = reasoningEndTime - reasoningStartTime;
              updateMessage(conversationId, assistantMessage.id, {reasoningDurationMs: duration});
            }
            appendDelta(conversationId, assistantMessage.id, event.text);
          } else if (event.type === 'reasoning-delta') {
            appendReasoningDelta(conversationId, assistantMessage.id, event.text);
          } else if (event.type === 'tool-call') {
            appendMessageEvent(conversationId, assistantMessage.id, {
              id: `${event.requestId}-${event.tool}`,
              kind: 'tool',
              text: `调用工具 ${event.tool}`,
              status: 'running',
              action: typeof event.args === 'string' ? event.args : JSON.stringify(event.args),
              ts: Date.now(),
            });
          } else if (event.type === 'tool-result') {
            appendMessageEvent(conversationId, assistantMessage.id, {
              id: `${event.requestId}-${event.tool}`,
              kind: 'tool',
              text: `工具 ${event.tool} ${event.ok ? '执行成功' : '执行失败'}`,
              status: event.ok ? 'done' : 'failed',
              receipt: event.summary,
              ts: Date.now(),
            });
          }
        },
      );
      updateMessage(conversationId, assistantMessage.id, {text: full || '(空响应)', streaming: false});
    } catch (caught) {
      const isAborted = caught instanceof Error && caught.name === 'AbortError';
      const message = caught instanceof Error ? caught.message : String(caught);
      if (isAborted) {
        updateMessage(conversationId, assistantMessage.id, {streaming: false, aborted: true});
      } else {
        error = message;
        updateMessage(conversationId, assistantMessage.id, {streaming: false, error: message});
        healthState = 'error';
      }
    } finally {
      busy = false;
      // 生成结束: 恢复真实 health (backend 可能已离线)
      await refreshConnection();
    }
  }

  function stop(): void {
    agentRuntime.abort();
  }

  function retry(messageId: string): void {
    if (busy || !activeConversation) return;
    const msgs = activeConversation.messages;
    const idx = msgs.findIndex((m) => m.id === messageId);
    if (idx < 0) return;
    let userText = '';
    for (let i = idx - 1; i >= 0; i--) {
      if (msgs[i].role === 'user') {
        userText = msgs[i].text;
        break;
      }
    }
    const filtered = msgs.filter((m) => m.id !== messageId);
    updateConversation(activeConversation.id, {messages: filtered});
    if (userText) {
      void send(userText);
    }
  }

  /** health 状态 → 人类可读文案 (Phase 5E). */
  const healthLabel: Record<HealthState, string> = {
    connecting: '连接中…',
    ready: '后端已连接',
    generating: '生成中…',
    error: '出错了',
    offline: '后端离线',
  };

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
    // 重建 runtime 以使用新配置 (§15 contract 实例绑定 config)
    agentRuntime = createAgentRuntime(config);
    healthState = 'connecting';
    const ok = await checkHealth(config.baseUrl);
    healthState = ok ? 'ready' : 'offline';
    if (ok) {
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
    // 后台健康轮询与审批请求同步 (真实 HTTP /health + /v1/apeireth/approval-requests)
    const timer = window.setInterval(() => {
      void refreshConnection();
    }, 10000);
    return () => window.clearInterval(timer);
  });
</script>

<div class="shell">
  <Sidebar
    {nav}
    bind:activeView
    {healthState}
    {healthLabel}
    onNewConversation={newConversation}
  />

  <main class="main">
    {#if activeView === 'chat'}
      <ChatView
        {config}
        conversation={activeConversation}
        messages={activeMessages}
        approvalRequests={pendingApprovals}
        bind:draft
        {busy}
        {error}
        onSend={(text) => send(text)}
        onStop={stop}
        onRetry={retry}
        onApproved={refreshConnection}
        onNewConversation={newConversation}
      />
    {:else if activeView === 'conversations'}
      <ConversationsView
        {conversations}
        activeId={activeId || ''}
        onOpen={openConversation}
        onCreate={newConversation}
        onArchive={archiveConversation}
        onDelete={deleteConversation}
      />
    {:else if activeView === 'activity'}
      <ActivityCenterView {config} />
    {:else if activeView === 'tools'}
      <ToolRegistryView {config} />
    {:else if activeView === 'memory'}
      <MemoryView {config} />
    {:else}
      <SettingsView
        {config}
        {healthState}
        {healthLabel}
        bind:editBaseUrl
        bind:editApiKey
        bind:editModel
        {modelsList}
        {error}
        onSave={saveSettings}
        onRefreshModels={loadModels}
      />
    {/if}
  </main>
</div>
