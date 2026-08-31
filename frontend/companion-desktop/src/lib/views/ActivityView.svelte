<script lang="ts">
  import {onMount, onDestroy} from 'svelte';
  import {
    Activity,
    Search,
    RotateCcw,
    Radio,
    Terminal,
    Wrench,
    Layers3,
    Sparkles,
    CheckCircle2,
    AlertTriangle,
    XCircle,
    ChevronDown,
    ChevronRight,
    Clock,
    X,
    Filter,
    Play,
    Pause,
    FileJson,
    Download,
    Trash2,
    Copy,
    Check,
    Cpu,
    Zap,
  } from 'lucide-svelte';
  import PageHeader from '../../components/PageHeader.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import ErrorState from '../components/ErrorState.svelte';
  import LoadingState from '../components/LoadingState.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import type {ActivityItem, ApeirethConfig, CapabilityManifest} from '../types';
  import {fetchAuditLogs, fetchTraceDetail, capabilitySupported, friendlyErrorMessage} from '../runtime';
  import {splitPresenceLine, type PresenceFrame} from '../presence';
  import {
    getCallLogs,
    clearCallLogs,
    subscribeCallLogs,
    exportCallLogsJson,
    type CallLogEntry,
  } from '../call-logger';

  let {
    config,
    capabilities = null,
  }: {
    config: ApeirethConfig;
    capabilities: CapabilityManifest | null;
  } = $props();

  // Tab mode: 'calls' (模型调用日志) | 'audit' (系统事件审计)
  let activeTab = $state<'calls' | 'audit'>('calls');

  // Call Logs state
  let callLogs = $state<CallLogEntry[]>([]);
  let callStatusFilter = $state<'all' | 'success' | 'error' | 'aborted'>('all');
  let callSearchQuery = $state('');
  let expandedCallIds = $state<Record<string, boolean>>({});
  let copiedCallId = $state<string | null>(null);

  // Capability gating: trace 关联 (Phase 5).
  let canReadTrace = $derived(capabilitySupported(capabilities, 'trace.read'));

  // Trace detail modal (Phase 5): 点击带 traceId 的活动 → 打开 span 树.
  import type {TraceSpanItem} from '../runtime';
  let traceDetail = $state<{traceId: string; spans: TraceSpanItem[]; loading: boolean; error: string} | null>(null);

  async function openTrace(traceId: string): Promise<void> {
    if (!canReadTrace) return;

    if (!capabilitySupported(capabilities, 'trace.read')) {
      traceDetail = {traceId, spans: [], loading: false, error: '追踪详情不支持: 当前运行时未实现 trace.read (Apeireth 2.0 canonical gateway 无此内省 API)'};
      return;
    }

    traceDetail = {traceId, spans: [], loading: true, error: ''};
    const r = await fetchTraceDetail(config, traceId);
    if (Array.isArray(r)) {
      traceDetail = {traceId, spans: r, loading: false, error: ''};
    } else {
      traceDetail = {traceId, spans: [], loading: false, error: r.error};
    }
  }

  function spanTree(spans: TraceSpanItem[]): TraceSpanItem[] {
    return [...spans].sort((a, b) => a.started_at - b.started_at);
  }

  function spanIndent(spans: TraceSpanItem[], span: TraceSpanItem): number {
    let depth = 0;
    let cur = span.parent_span_id;
    const guard = new Set<string>();
    while (cur && !guard.has(cur)) {
      guard.add(cur);
      depth++;
      const parent = spans.find((s) => s.span_id === cur);
      cur = parent?.parent_span_id ?? null;
    }
    return Math.min(depth, 6);
  }

  type CategoryFilter = 'all' | 'tool' | 'agent' | 'memory' | 'workflow' | 'runtime' | 'error';
  type SeverityFilter = 'all' | 'info' | 'success' | 'warning' | 'error';

  let activities = $state<ActivityItem[]>([]);
  let loading = $state(false);
  let error = $state('');
  let searchQuery = $state('');
  let selectedCategory = $state<CategoryFilter>('all');
  let selectedSeverity = $state<SeverityFilter>('all');
  let isLive = $state(true);
  let expandedIds = $state<Record<string, boolean>>({});

  let sseEventSource: EventSource | null = null;

  function toggleCallExpand(id: string) {
    expandedCallIds = {...expandedCallIds, [id]: !expandedCallIds[id]};
  }

  async function copyCallJson(entry: CallLogEntry) {
    try {
      await navigator.clipboard.writeText(JSON.stringify(entry, null, 2));
      copiedCallId = entry.id;
      setTimeout(() => { copiedCallId = null; }, 2000);
    } catch {}
  }

  function handleExportLogs() {
    const json = exportCallLogsJson();
    const blob = new Blob([json], {type: 'application/json'});
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `apeireth_call_logs_${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function handleClearLogs() {
    if (confirm('确定要清空全部模型调用日志吗？')) {
      clearCallLogs();
    }
  }

  const filteredCallLogs = $derived.by(() => {
    let list = [...callLogs];
    if (callStatusFilter !== 'all') {
      list = list.filter((c) => c.status === callStatusFilter);
    }
    if (callSearchQuery.trim()) {
      const q = callSearchQuery.toLowerCase().trim();
      list = list.filter((c) =>
        c.model.toLowerCase().includes(q) ||
        c.endpoint.toLowerCase().includes(q) ||
        (c.responseContent && c.responseContent.toLowerCase().includes(q)) ||
        c.requestMessages.some((m) => m.content.toLowerCase().includes(q)) ||
        (c.errorMessage && c.errorMessage.toLowerCase().includes(q))
      );
    }
    return list;
  });

  const callStats = $derived.by(() => {
    const total = callLogs.length;
    if (!total) return {total: 0, avgLatency: 0, successRate: '100%', errors: 0};
    const totalLat = callLogs.reduce((acc, c) => acc + (c.latencyMs || 0), 0);
    const successCount = callLogs.filter((c) => c.status === 'success').length;
    const errors = callLogs.filter((c) => c.status === 'error').length;
    return {
      total,
      avgLatency: Math.round(totalLat / total),
      successRate: `${Math.round((successCount / total) * 100)}%`,
      errors,
    };
  });

  const categoryIcons = {
    conversation: Radio,
    agent: Sparkles,
    tool: Wrench,
    memory: Layers3,
    workflow: Terminal,
    runtime: Activity,
    error: XCircle,
  };

  const categoryLabels = {
    all: '全部类别',
    conversation: '对话',
    agent: 'Agent',
    tool: '工具执行',
    memory: '记忆读写',
    workflow: '工作流',
    runtime: '运行时',
    error: '错误异常',
  };

  function toggleExpand(id: string) {
    expandedIds = {...expandedIds, [id]: !expandedIds[id]};
  }

  /**
   * 去重合并算法：根据 id 或 (近同时间戳 + 相同标题/工具) 防止 SSE 与持久化 Audit 重复显示
   */
  function mergeActivities(existing: ActivityItem[], incoming: ActivityItem[]): ActivityItem[] {
    const map = new Map<string, ActivityItem>();

    // Put existing
    for (const item of existing) {
      map.set(item.id, item);
    }

    // Merge incoming with soft dedup
    for (const item of incoming) {
      if (map.has(item.id)) {
        map.set(item.id, {...map.get(item.id)!, ...item});
        continue;
      }
      // Check timestamp and title collision within 1500ms
      let foundDup = false;
      for (const [_, ex] of map) {
        if (
          Math.abs(ex.timestamp - item.timestamp) < 1500 &&
          ex.title === item.title &&
          ex.summary === item.summary
        ) {
          foundDup = true;
          break;
        }
      }
      if (!foundDup) {
        map.set(item.id, item);
      }
    }

    const merged = Array.from(map.values());
    merged.sort((a, b) => b.timestamp - a.timestamp);
    return merged.slice(0, 300); // keep up to 300 events in memory
  }

  async function loadPersistedAudit() {
    loading = true;
    error = '';
    try {
      // Capability gate: prevent calling unsupported /v1/panel/audit
      if (!capabilitySupported(capabilities, 'audit.read')) {
        error = '审计日志不支持: 当前运行时未实现 audit.read (Apeireth 2.0 canonical gateway 无此内省 API)';
        loading = false;
        return;
      }

      const logs = await fetchAuditLogs(config, 80);
      activities = mergeActivities(activities, logs);
    } catch (e) {
      error = friendlyErrorMessage(e, '/v1/panel/audit');
    } finally {
      loading = false;
    }
  }

  /**
   * presence 帧 → 活动条目（波次 2：契约 §8.1 分流纪律，修 G5 缺口）。
   * - initiative/held（欲言又止 = 他的内心）不进对话流，但在此可见；
   * - emotion 心跳（60s tick）与 legacy 测试行不进活动流——防刷屏，不是数据丢失；
   * - memory_recall 只带 found/keywords（redacted 恒 true，原文设计上不在 SSE）。
   */
  function presenceEventToActivity(ev: PresenceFrame): ActivityItem | null {
    const ts = 'at' in ev && typeof ev.at === 'string' ? Date.parse(ev.at) || Date.now() : Date.now();
    const id = `presence-${ev.type}-${ts}-${Math.random().toString(36).slice(2, 6)}`;
    if (ev.type === 'emotion') return null; // 心跳由场景层与状态行呈现
    if (ev.type === 'initiative') {
      if (ev.outcome === 'held') {
        return {
          id, timestamp: ts, category: 'agent', source: 'sse', severity: 'info',
          title: '他欲言又止',
          summary: `门控：${ev.gate_label || ev.gate || '未知'}`,
          detail: JSON.stringify(ev, null, 2), raw: ev,
        };
      }
      return {
        id, timestamp: ts, category: 'conversation', source: 'sse', severity: 'info',
        title: '他主动开口',
        summary: ev.action ? `动作：${ev.action}` : '完整话术见对话视图',
        detail: JSON.stringify(ev, null, 2), raw: ev,
      };
    }
    if (ev.type === 'dream') {
      return {
        id, timestamp: ts, category: 'memory', source: 'sse', severity: 'success',
        title: '做梦整合完成',
        summary: `合并 ${ev.merged_count} 条记忆${ev.summary_prefix ? ` · ${ev.summary_prefix}` : ''}`,
        detail: JSON.stringify(ev, null, 2), raw: ev,
      };
    }
    if (ev.type === 'memory_recall') {
      return {
        id, timestamp: ts, category: 'memory', source: 'sse', severity: 'info',
        title: `他想起了 ${ev.found} 段记忆`,
        summary: ev.keywords?.length ? `关键词：${ev.keywords.join(' · ')}` : '（脱敏事件，不含原文）',
        detail: JSON.stringify(ev, null, 2), raw: ev,
      };
    }
    // presence_error：序列化兜底帧，显式呈报而非静默
    return {
      id, timestamp: ts, category: 'runtime', source: 'sse', severity: 'error',
      title: 'presence 频道序列化异常',
      summary: ev.error,
      detail: JSON.stringify(ev, null, 2), raw: ev,
    };
  }

  function startSseListener() {
    if (sseEventSource) {
      sseEventSource.close();
      sseEventSource = null;
    }

    if (!isLive) return;
    if (!capabilitySupported(capabilities, 'activity.sse')) {
      return;
    }

    try {
      const base = config.baseUrl.replace(/\/+$/, '');
      sseEventSource = new EventSource(`${base}/v1/apeireth/events`);

      sseEventSource.onmessage = (event) => {
        const raw = typeof event.data === 'string' ? event.data : '';
        // 契约 §8.1 分流：行首 { → presence JSON；否则 legacy 文本（[他说]/测试事件）
        const split = splitPresenceLine(raw);
        if (split.kind === 'legacy') {
          const text = split.text ?? '';
          if (!text.startsWith('[他说]')) return; // 测试事件行 = 链路验证，不进活动流
          const said = text.slice('[他说]'.length).trim();
          activities = mergeActivities(activities, [{
            id: `legacy-say-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
            timestamp: Date.now(),
            category: 'conversation',
            title: '他主动开口',
            summary: said.length > 96 ? `${said.slice(0, 96)}…` : said,
            source: 'sse',
            severity: 'info',
          }]);
          return;
        }
        if (split.kind === 'presence' && split.event) {
          const item = presenceEventToActivity(split.event);
          if (item) activities = mergeActivities(activities, [item]);
          return;
        }
        // 其余 JSON（未来 span 帧等）：保留既有通用解析路径
        try {
          const parsed = JSON.parse(raw) as {
            id?: string;
            type?: string;
            action?: string;
            tool?: string;
            summary?: string;
            detail?: string;
            status?: string;
            ts?: number;
            trace_id?: string;
            span_id?: string;
            kind?: string;
          };

          const newEvent: ActivityItem = {
            id: parsed.id || `sse-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
            timestamp: parsed.ts || Date.now(),
            category: (parsed.tool ? 'tool' : parsed.type === 'memory' ? 'memory' : 'agent') as ActivityItem['category'],
            title: parsed.tool ? `调用工具: ${parsed.tool}` : (parsed.summary || parsed.action || 'Agent 活动'),
            summary: parsed.summary || parsed.detail || '实时事件',
            source: 'sse',
            severity: parsed.status === 'error' || parsed.status === 'failed' ? 'error' : 'info',
            detail: JSON.stringify(parsed, null, 2),
            raw: parsed,
            traceId: parsed.trace_id,
          };

          activities = mergeActivities(activities, [newEvent]);
        } catch {
          // ignore malformed SSE
        }
      };

      sseEventSource.onerror = () => {
        // SSE disconnected or endpoint offline
      };
    } catch {
      // ignore
    }
  }

  function toggleLive() {
    isLive = !isLive;
    if (isLive) {
      startSseListener();
    } else if (sseEventSource) {
      sseEventSource.close();
      sseEventSource = null;
    }
  }

  const filteredActivities = $derived.by(() => {
    let list = [...activities];

    if (selectedCategory !== 'all') {
      list = list.filter((a) => a.category === selectedCategory);
    }

    if (selectedSeverity !== 'all') {
      list = list.filter((a) => a.severity === selectedSeverity);
    }

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter((a) =>
        a.title.toLowerCase().includes(q) ||
        a.summary.toLowerCase().includes(q) ||
        (a.detail && a.detail.toLowerCase().includes(q)),
      );
    }

    return list;
  });

  function formatTime(ts: number): string {
    const d = new Date(ts);
    return d.toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit', second: '2-digit'});
  }

  function formatRelative(ts: number): string {
    const diffSec = Math.floor((Date.now() - ts) / 1000);
    if (diffSec < 5) return '刚刚';
    if (diffSec < 60) return `${diffSec}秒前`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}分钟前`;
    const diffHour = Math.floor(diffMin / 60);
    if (diffHour < 24) return `${diffHour}小时前`;
    return `${Math.floor(diffHour / 24)}天前`;
  }

  let unsubCallLogs: (() => void) | null = null;
  onMount(() => {
    unsubCallLogs = subscribeCallLogs((logs) => {
      callLogs = logs;
    });
    void loadPersistedAudit();
    startSseListener();
  });

  onDestroy(() => {
    unsubCallLogs?.();
    if (sseEventSource) {
      sseEventSource.close();
      sseEventSource = null;
    }
  });
</script>

<section class="activity-view">
  <PageHeader
    eyebrow="观察与审计"
    title="活动与调用日志"
    subtitle="记录每一轮模型交互的延迟、Token、Prompt 及 CoT 思考流，同步展示底层决策与系统事件。"
  />

  <!-- Mode Switch Tabs -->
  <div class="view-mode-header">
    <div class="view-mode-tabs">
      <button
        class="mode-tab-btn"
        class:active={activeTab === 'calls'}
        onclick={() => activeTab = 'calls'}
      >
        <Zap size={13} />
        <span>模型调用日志</span>
        <span class="tab-badge">{callLogs.length}</span>
      </button>
      <button
        class="mode-tab-btn"
        class:active={activeTab === 'audit'}
        onclick={() => activeTab = 'audit'}
      >
        <Activity size={13} />
        <span>系统事件审计</span>
        <span class="tab-badge">{activities.length}</span>
      </button>
    </div>

    {#if activeTab === 'calls'}
      <div class="tab-header-actions">
        <button class="quiet-button export-btn" onclick={handleExportLogs} disabled={!callLogs.length} title="导出全部调用日志为 JSON">
          <Download size={13} />
          <span>导出日志</span>
        </button>
        <button class="quiet-button clear-btn" onclick={handleClearLogs} disabled={!callLogs.length} title="清空调用日志">
          <Trash2 size={13} />
          <span>清空</span>
        </button>
      </div>
    {:else}
      <div class="tab-header-actions">
        <button
          class="live-toggle-btn"
          class:active={isLive}
          onclick={toggleLive}
          title={isLive ? '点击暂停实时监听' : '点击开启实时监听'}
        >
          {#if isLive}
            <Radio size={13} class="live-icon spin" />
            <span>实时流 (已连接)</span>
          {:else}
            <Pause size={13} />
            <span>已暂停</span>
          {/if}
        </button>
        <button class="quiet-button" onclick={loadPersistedAudit} disabled={loading}>
          <RotateCcw size={13} class={loading ? 'spin' : ''} />
          <span>刷新审计</span>
        </button>
      </div>
    {/if}
  </div>

  {#if activeTab === 'calls'}
    <!-- Call Stats Summary -->
    <div class="call-stats-strip">
      <div class="stat-pill">
        <span class="stat-num">{callStats.total}</span>
        <span class="stat-lbl">总调用轮次</span>
      </div>
      <div class="stat-pill">
        <span class="stat-num">{callStats.avgLatency} <small>ms</small></span>
        <span class="stat-lbl">平均耗时</span>
      </div>
      <div class="stat-pill">
        <span class="stat-num text-success">{callStats.successRate}</span>
        <span class="stat-lbl">成功率</span>
      </div>
      <div class="stat-pill">
        <span class="stat-num" class:text-danger={callStats.errors > 0}>{callStats.errors}</span>
        <span class="stat-lbl">异常错误</span>
      </div>
    </div>

    <!-- Call Logs Toolbar -->
    <div class="activity-toolbar">
      <div class="search-input-wrap">
        <Search size={14} class="search-icon" />
        <input
          type="text"
          placeholder="搜索模型、端点、Prompt 或输出内容…"
          bind:value={callSearchQuery}
        />
        {#if callSearchQuery}
          <button class="clear-search-btn" onclick={() => callSearchQuery = ''} aria-label="清除搜索">
            <X size={12} />
          </button>
        {/if}
      </div>

      <div class="filters-wrap">
        <div class="category-tabs">
          <button class="cat-btn" class:active={callStatusFilter === 'all'} onclick={() => callStatusFilter = 'all'}>全部</button>
          <button class="cat-btn" class:active={callStatusFilter === 'success'} onclick={() => callStatusFilter = 'success'}>成功</button>
          <button class="cat-btn" class:active={callStatusFilter === 'error'} onclick={() => callStatusFilter = 'error'}>错误</button>
          <button class="cat-btn" class:active={callStatusFilter === 'aborted'} onclick={() => callStatusFilter = 'aborted'}>已中止</button>
        </div>
      </div>
    </div>

    <!-- Call Logs List -->
    <div class="call-logs-container">
      {#if !filteredCallLogs.length}
        <EmptyState
          icon="⚡"
          title={callSearchQuery ? '没有找到匹配的调用日志' : '暂无模型调用记录'}
          description="在对话窗口发送消息后，每一轮调用的端点、延迟、Payload 与 CoT 思考流将在此实时记录。"
        />
      {:else}
        <div class="call-logs-stream">
          {#each filteredCallLogs as log (log.id)}
            <article class="call-log-card" class:expanded={expandedCallIds[log.id]} class:error={log.status === 'error'}>
              <div
                class="call-card-head"
                role="button"
                tabindex="0"
                onclick={() => toggleCallExpand(log.id)}
                onkeydown={(e) => e.key === 'Enter' && toggleCallExpand(log.id)}
              >
                <div class="call-head-left">
                  <span class="status-pill status-{log.status}">
                    {log.status === 'success' ? 'OK 200' : log.status === 'aborted' ? 'ABORT' : 'ERR'}
                  </span>
                  <span class="proto-tag proto-{log.protocol}">{log.protocol.toUpperCase()}</span>
                  <strong class="model-name">{log.model}</strong>
                  <span class="endpoint-url" title={log.endpoint}>{log.endpoint}</span>
                </div>

                <div class="call-head-right">
                  <span class="latency-pill">{log.latencyMs} ms</span>
                  <time class="call-time">{log.timeFormatted}</time>
                  <button class="expand-arrow-btn" aria-label={expandedCallIds[log.id] ? '收起' : '展开'}>
                    {#if expandedCallIds[log.id]}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
                  </button>
                </div>
              </div>

              <!-- Compact Snippet -->
              {#if !expandedCallIds[log.id]}
                <div class="call-snippet">
                  <span class="snip-prompt">提问: {log.requestMessages[log.requestMessages.length - 1]?.content.slice(0, 85) || '(无提示词)'}</span>
                  {#if log.responseContent}
                    <span class="snip-resp">↳ 回复: {log.responseContent.slice(0, 110)}</span>
                  {/if}
                  {#if log.errorMessage}
                    <span class="snip-err">↳ 报错: {log.errorMessage}</span>
                  {/if}
                </div>
              {/if}

              <!-- Expanded Details -->
              {#if expandedCallIds[log.id]}
                <div class="call-details-expanded">
                  <!-- Prompt / Messages -->
                  <div class="detail-section">
                    <div class="section-title">
                      <span>请求消息上下文 ({log.requestMessages.length} 条)</span>
                    </div>
                    <div class="messages-list-inspect">
                      {#each log.requestMessages as msg}
                        <div class="inspect-msg-row role-{msg.role}">
                          <span class="role-badge">{msg.role}</span>
                          <pre class="msg-content-pre">{msg.content}</pre>
                        </div>
                      {/each}
                    </div>
                  </div>

                  <!-- Reasoning CoT if available -->
                  {#if log.reasoningContent}
                    <div class="detail-section reasoning-section">
                      <div class="section-title">
                        <Sparkles size={12} class="sparkle-gold" />
                        <span>思考过程 (Reasoning / CoT)</span>
                      </div>
                      <pre class="reasoning-pre">{log.reasoningContent}</pre>
                    </div>
                  {/if}

                  <!-- Response Content -->
                  {#if log.responseContent}
                    <div class="detail-section">
                      <div class="section-title">
                        <span>模型生成结果</span>
                      </div>
                      <pre class="response-pre">{log.responseContent}</pre>
                    </div>
                  {/if}

                  <!-- Error message -->
                  {#if log.errorMessage}
                    <div class="detail-section error-section">
                      <div class="section-title">
                        <AlertTriangle size={12} />
                        <span>异常详情</span>
                      </div>
                      <p class="error-text">{log.errorMessage}</p>
                    </div>
                  {/if}

                  <!-- Bottom actions -->
                  <div class="detail-bottom-bar">
                    <button class="call-copy-json-btn" onclick={() => copyCallJson(log)}>
                      {#if copiedCallId === log.id}
                        <Check size={12} class="green" />
                        <span>已复制完整 JSON 记录</span>
                      {:else}
                        <Copy size={12} />
                        <span>复制调用 JSON 详情</span>
                      {/if}
                    </button>
                  </div>
                </div>
              {/if}
            </article>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Toolbar & Filters for Audit -->
    <div class="activity-toolbar">
      <div class="search-input-wrap">
        <Search size={14} class="search-icon" />
        <input
          type="text"
          placeholder="搜索事件标题、摘要或参数…"
          bind:value={searchQuery}
        />
        {#if searchQuery}
          <button class="clear-search-btn" onclick={() => searchQuery = ''} aria-label="清除搜索">
            <X size={12} />
          </button>
        {/if}
      </div>

      <div class="filters-wrap">
        <!-- Category Tabs -->
        <div class="category-tabs">
          <button
            class="cat-btn"
            class:active={selectedCategory === 'all'}
            onclick={() => selectedCategory = 'all'}
          >全部</button>
          <button
            class="cat-btn"
            class:active={selectedCategory === 'tool'}
            onclick={() => selectedCategory = 'tool'}
          >工具</button>
          <button
            class="cat-btn"
            class:active={selectedCategory === 'agent'}
            onclick={() => selectedCategory = 'agent'}
          >Agent</button>
          <button
            class="cat-btn"
            class:active={selectedCategory === 'memory'}
            onclick={() => selectedCategory = 'memory'}
          >记忆</button>
          <button
            class="cat-btn"
            class:active={selectedCategory === 'runtime'}
            onclick={() => selectedCategory = 'runtime'}
          >运行时</button>
        </div>

        <!-- Severity Filter -->
        <select class="severity-select" bind:value={selectedSeverity} aria-label="筛选级别">
          <option value="all">全部状态</option>
          <option value="info">正常 / 信息</option>
          <option value="success">成功</option>
          <option value="warning">警告</option>
          <option value="error">异常 / 错误</option>
        </select>
      </div>
    </div>

    <!-- Timeline Body for Audit -->
    <div class="timeline-container">
      {#if loading && !activities.length}
        <LoadingState message="正在连接并加载活动时间线…" />
      {:else if error && !activities.length}
        <ErrorState title="拉取活动记录失败" message={error} onRetry={loadPersistedAudit} />
      {:else if !filteredActivities.length}
        <EmptyState
          icon="⚡"
          title={searchQuery ? '没有匹配的活动事件' : '暂无活动记录'}
          description="当与伙伴对话、调用工具或系统反思时，事件流会实时更新。"
        />
      {:else}
        <div class="timeline-stream">
          {#each filteredActivities as item (item.id)}
            {@const CategoryIcon = categoryIcons[item.category] || Activity}
            <article class="timeline-item" class:error={item.severity === 'error'} class:expanded={expandedIds[item.id]}>
              <!-- Left Axis Dot -->
              <div class="timeline-axis">
                <div class="axis-icon-dot {item.category} {item.severity}">
                  <CategoryIcon size={12} />
                </div>
                <div class="axis-line"></div>
              </div>

              <!-- Content Card -->
              <div class="timeline-card">
                <div
                  class="timeline-card-head"
                  role="button"
                  tabindex="0"
                  onclick={() => toggleExpand(item.id)}
                  onkeydown={(e) => e.key === 'Enter' && toggleExpand(item.id)}
                >
                  <div class="head-left">
                    <span class="source-tag {item.source}">{item.source === 'sse' ? '实时流' : '审计'}</span>
                    <strong class="item-title">{item.title}</strong>
                    <StatusBadge
                      label={categoryLabels[item.category] || item.category}
                      variant={item.category === 'tool' ? 'amber' : item.category === 'error' ? 'danger' : 'neutral'}
                      size="small"
                    />
                  </div>

                  <div class="head-right">
                    <span class="rel-time">{formatRelative(item.timestamp)}</span>
                    <time class="abs-time">{formatTime(item.timestamp)}</time>
                    <button class="expand-arrow-btn" aria-label={expandedIds[item.id] ? '收起详情' : '展开详情'}>
                      {#if expandedIds[item.id]}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}
                    </button>
                  </div>
                </div>

                <p class="item-summary">{item.summary}</p>

                {#if item.traceId && canReadTrace}
                  <button
                    class="trace-link-btn"
                    onclick={() => openTrace(item.traceId as string)}
                    title="查看执行轨迹 (trace 树)"
                  >
                    轨迹 →
                  </button>
                {/if}

                {#if expandedIds[item.id] && item.detail}
                  <div class="item-detail-wrap">
                    <span class="detail-label">技术详情 / 原始参数</span>
                    <pre class="detail-pre">{item.detail}</pre>
                  </div>
                {/if}
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</section>

{#if traceDetail}
  <div class="modal-backdrop" onclick={() => (traceDetail = null)} role="presentation">
    <div
      class="modal-dialog"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
      aria-modal="true"
    >
      <div class="modal-header">
        <h3>执行轨迹：{traceDetail.traceId.slice(0, 12)}…</h3>
        <button class="modal-close-btn" onclick={() => (traceDetail = null)} aria-label="关闭">×</button>
      </div>
      <div class="modal-body">
        {#if traceDetail.loading}
          <p class="trace-loading">加载轨迹中…</p>
        {:else if traceDetail.error}
          <p class="trace-error">轨迹加载失败：{traceDetail.error}</p>
        {:else if traceDetail.spans.length === 0}
          <p class="trace-empty">该轨迹无 span 记录。</p>
        {:else}
          <div class="trace-tree">
            {#each spanTree(traceDetail.spans) as span}
              <div class="trace-span" style="margin-left: {spanIndent(traceDetail.spans, span) * 16}px">
                <span class="span-kind">{span.kind}</span>
                <span class="span-actor">{span.actor}</span>
                <span class="span-status status-{span.status}">{span.status}</span>
                <span class="span-summary">{span.summary || ''}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .activity-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  /* Mode Header & Tabs */
  .view-mode-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 0 32px 12px;
    border-bottom: 1px solid var(--line);
    flex-wrap: wrap;
  }
  .view-mode-tabs {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--surface-2);
    padding: 3px;
    border-radius: 8px;
    border: 1px solid var(--line);
  }
  .mode-tab-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: 6px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .mode-tab-btn:hover {
    color: var(--text);
  }
  .mode-tab-btn.active {
    background: var(--surface-3);
    color: var(--ap-gold-ui, #ffd27a);
    font-weight: 500;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
  }
  .tab-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 9999px;
    background: rgba(255, 210, 122, 0.15);
    color: var(--ap-gold);
    font-family: var(--mono);
  }
  .tab-header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .export-btn, .clear-btn {
    font-size: 11.5px;
  }

  /* Call Stats Strip */
  .call-stats-strip {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 32px;
    background: rgba(0, 0, 0, 0.15);
    border-bottom: 1px solid var(--line);
    flex-wrap: wrap;
  }
  .stat-pill {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 4px 12px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 6px;
  }
  .stat-num {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    font-family: var(--mono);
  }
  .stat-num small {
    font-size: 10px;
    color: var(--faint);
  }
  .stat-lbl {
    font-size: 11px;
    color: var(--faint);
  }
  .text-success {
    color: #4ade80 !important;
  }
  .text-danger {
    color: #f87171 !important;
  }

  /* Call Logs Stream & Cards */
  .call-logs-container {
    flex: 1;
    overflow-y: auto;
    padding: 16px 36px 60px;
    width: 100%;
    min-height: 0;
  }
  .call-logs-stream {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    max-width: 100%;
    margin: 0;
  }
  .call-log-card {
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 12px 18px;
    transition: all 0.15s ease;
    width: 100%;
  }
  .call-log-card:hover {
    border-color: var(--line-strong);
    background: var(--surface-3);
  }
  .call-log-card.error {
    border-color: rgba(239, 68, 68, 0.35);
  }
  .call-log-card.expanded {
    border-color: var(--ap-gold-ui, #ffd27a);
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.28);
  }

  .call-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
    user-select: none;
    gap: 16px;
    width: 100%;
  }
  .call-head-left {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    min-width: 0;
    flex: 1;
  }
  .status-pill {
    font-size: 10.5px;
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 600;
    font-family: var(--mono);
  }
  .status-pill.status-success {
    background: rgba(74, 222, 128, 0.15);
    color: #4ade80;
    border: 1px solid rgba(74, 222, 128, 0.3);
  }
  .status-pill.status-error {
    background: rgba(248, 113, 113, 0.15);
    color: #f87171;
    border: 1px solid rgba(248, 113, 113, 0.3);
  }
  .status-pill.status-aborted {
    background: rgba(251, 191, 36, 0.15);
    color: #fbbf24;
    border: 1px solid rgba(251, 191, 36, 0.3);
  }
  .proto-tag {
    font-size: 10px;
    padding: 2px 7px;
    border-radius: 4px;
    font-family: var(--mono);
    background: var(--surface-3);
    color: var(--muted);
    border: 1px solid var(--line);
  }
  .proto-tag.proto-openai {
    color: #38bdf8;
    border-color: rgba(56, 189, 248, 0.3);
  }
  .proto-tag.proto-anthropic {
    color: #f472b6;
    border-color: rgba(244, 114, 182, 0.3);
  }
  .proto-tag.proto-gateway {
    color: var(--ap-gold);
    border-color: rgba(255, 210, 122, 0.3);
  }
  .model-name {
    font-size: 13.5px;
    color: var(--text);
    font-family: var(--mono);
  }
  .endpoint-url {
    font-size: 11.5px;
    color: var(--faint);
    max-width: 480px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .call-head-right {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }
  .latency-pill {
    font-size: 11.5px;
    color: var(--ap-gold-ui, #ffd27a);
    font-family: var(--mono);
    background: rgba(255, 210, 122, 0.08);
    padding: 3px 8px;
    border-radius: 4px;
    border: 1px solid rgba(255, 210, 122, 0.18);
  }
  .call-time {
    font-size: 11px;
    color: var(--faint);
    font-family: var(--mono);
  }

  .call-snippet {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 8px;
    padding-top: 6px;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
    font-size: 12px;
    color: var(--muted);
    line-height: 1.5;
  }
  .snip-prompt {
    color: var(--text);
    word-break: break-word;
  }
  .snip-resp {
    color: rgba(232, 224, 204, 0.7);
    word-break: break-word;
  }
  .snip-err {
    color: #f87171;
  }

  /* Expanded Inspect */
  .call-details-expanded {
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .detail-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .section-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.6px;
  }
  .sparkle-gold {
    color: var(--ap-gold);
  }
  .messages-list-inspect {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .inspect-msg-row {
    display: flex;
    flex-direction: column;
    gap: 5px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 8px 12px;
  }
  .role-badge {
    align-self: flex-start;
    font-size: 10.5px;
    padding: 2px 7px;
    border-radius: 3px;
    font-family: var(--mono);
    text-transform: uppercase;
    background: var(--surface-3);
    color: var(--muted);
  }
  .inspect-msg-row.role-user .role-badge {
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
  }
  .inspect-msg-row.role-assistant .role-badge {
    background: rgba(255, 210, 122, 0.15);
    color: var(--ap-gold);
  }
  .inspect-msg-row.role-system .role-badge {
    background: rgba(148, 163, 184, 0.15);
    color: #94a3b8;
  }
  .msg-content-pre, .response-pre, .reasoning-pre {
    margin: 0;
    font-family: var(--ap-font-body, system-ui);
    font-size: 12.5px;
    line-height: 1.65;
    color: var(--text);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 700px;
    overflow-y: auto;
  }
  .reasoning-section {
    background: rgba(255, 210, 122, 0.05);
    border: 1px solid rgba(255, 210, 122, 0.2);
    border-radius: 6px;
    padding: 10px 14px;
  }
  .reasoning-pre {
    color: #d8cca9;
    font-style: italic;
    font-size: 12px;
  }
  .response-pre {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 10px 14px;
  }
  .error-section {
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 6px;
    padding: 10px 14px;
  }
  .error-text {
    margin: 0;
    color: #f87171;
    font-size: 12px;
    font-family: var(--mono);
  }
  .detail-bottom-bar {
    display: flex;
    justify-content: flex-end;
    margin-top: 6px;
  }
  .call-copy-json-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 4px;
    border: 1px solid var(--line-strong);
    background: var(--surface-3);
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .call-copy-json-btn:hover {
    color: var(--text);
    border-color: var(--ap-gold-ui, #ffd27a);
  }

  .live-toggle-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 6px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .live-toggle-btn.active {
    border-color: var(--amber-line);
    color: var(--amber);
    background: var(--amber-wash);
  }
  :global(.live-icon) {
    color: var(--amber);
  }

  .activity-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 32px 14px;
    border-bottom: 1px solid var(--line);
    flex-wrap: wrap;
  }
  .search-input-wrap {
    flex: 1;
    min-width: 240px;
    max-width: 380px;
    position: relative;
    display: flex;
    align-items: center;
  }
  :global(.search-icon) {
    position: absolute;
    left: 10px;
    color: var(--faint);
  }
  .search-input-wrap input {
    width: 100%;
    padding: 7px 28px 7px 30px;
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    color: var(--text);
    font-size: 12px;
    outline: 0;
  }
  .search-input-wrap input:focus {
    border-color: var(--amber-line);
  }
  .clear-search-btn {
    position: absolute;
    right: 8px;
    border: 0;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
    display: grid;
    place-items: center;
  }

  .filters-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .category-tabs {
    display: flex;
    gap: 4px;
  }
  .cat-btn {
    border: 1px solid var(--line);
    background: var(--surface-2);
    color: var(--muted);
    font-size: 11px;
    padding: 5px 10px;
    border-radius: 6px;
    cursor: pointer;
  }
  .cat-btn:hover {
    color: var(--text);
    border-color: var(--line-strong);
  }
  .cat-btn.active {
    background: var(--amber-wash);
    border-color: var(--amber-line);
    color: var(--amber);
  }
  .severity-select {
    padding: 5px 8px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--muted);
    font-size: 11px;
    outline: 0;
  }

  .timeline-container {
    flex: 1;
    overflow-y: auto;
    padding: 20px 36px 60px;
    width: 100%;
    min-height: 0;
  }
  .timeline-stream {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    max-width: 100%;
    margin: 0;
  }
  .timeline-item {
    display: flex;
    gap: 16px;
  }
  .timeline-axis {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 28px;
    flex: none;
  }
  .axis-icon-dot {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--surface-3);
    border: 1px solid var(--line-strong);
    color: var(--muted);
    display: grid;
    place-items: center;
    z-index: 1;
  }
  .axis-icon-dot.tool {
    background: var(--amber-wash);
    color: var(--amber);
    border-color: var(--amber-line);
  }
  .axis-icon-dot.error {
    background: rgba(224, 91, 80, 0.15);
    color: var(--danger);
    border-color: rgba(224, 91, 80, 0.35);
  }
  .axis-line {
    flex: 1;
    width: 1px;
    background: var(--line);
    margin-top: 4px;
  }
  .timeline-item:last-child .axis-line {
    display: none;
  }

  .timeline-card {
    flex: 1;
    margin-bottom: 12px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 9px;
    padding: 10px 14px;
    transition: all 0.15s ease;
  }
  .timeline-card:hover {
    border-color: var(--line-strong);
    background: var(--surface-3);
  }
  .timeline-item.error .timeline-card {
    border-color: rgba(224, 91, 80, 0.3);
  }

  .timeline-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
    user-select: none;
  }
  .head-left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .source-tag {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    font-family: var(--mono);
  }
  .source-tag.sse {
    background: var(--amber-wash);
    color: var(--amber);
  }
  .source-tag.audit {
    background: var(--blue-wash);
    color: var(--blue);
  }
  .item-title {
    font-size: 13px;
    color: var(--text);
  }
  .head-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .rel-time {
    font-size: 11px;
    color: var(--faint);
  }
  .abs-time {
    font-size: 10px;
    font-family: var(--mono);
    color: var(--faint);
  }
  .expand-arrow-btn {
    border: 0;
    background: transparent;
    color: var(--muted);
    padding: 2px;
    display: grid;
    place-items: center;
  }

  .item-summary {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.5;
  }

  .item-detail-wrap {
    margin-top: 10px;
    padding-top: 8px;
    border-top: 1px solid var(--line);
  }
  .detail-label {
    display: block;
    font-size: 10px;
    color: var(--faint);
    margin-bottom: 4px;
    text-transform: uppercase;
  }
  .detail-pre {
    margin: 0;
    padding: 8px 10px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 11px;
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 240px;
    overflow-y: auto;
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  .trace-link-btn {
    display: inline-block;
    margin-top: 4px;
    padding: 2px 8px;
    font-size: 11px;
    border-radius: 4px;
    background: rgba(245, 166, 35, 0.12);
    color: var(--accent, #f5a623);
    border: 1px solid rgba(245, 166, 35, 0.2);
    cursor: pointer;
  }
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: grid;
    place-items: center;
    z-index: 1000;
    padding: 20px;
  }
  .modal-dialog {
    background: var(--bg-card, #1a1a1a);
    border: 1px solid var(--border, rgba(255,255,255,0.1));
    border-radius: 12px;
    width: 90%;
    max-width: 640px;
    max-height: 80vh;
    overflow: auto;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 20px;
    border-bottom: 1px solid var(--border, rgba(255,255,255,0.08));
  }
  .modal-header h3 {
    font-size: 14px;
    margin: 0;
  }
  .modal-close-btn {
    background: none;
    border: none;
    color: var(--text-dim, #888);
    font-size: 20px;
    cursor: pointer;
  }
  .modal-body {
    padding: 14px 20px;
    font-family: monospace;
    font-size: 12px;
  }
  .trace-tree {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .trace-span {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 4px 8px;
    border-left: 2px solid var(--border, rgba(255,255,255,0.1));
  }
  .span-kind {
    color: var(--accent, #f5a623);
    min-width: 70px;
  }
  .span-actor {
    color: var(--text-dim, #aaa);
    min-width: 90px;
  }
  .span-status {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 3px;
  }
  .status-succeeded {
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
  }
  .status-failed {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
  }
  .status-running {
    background: rgba(245, 166, 35, 0.15);
    color: #f5a623;
  }
  .span-summary {
    color: var(--text, #ccc);
  }
  .trace-loading, .trace-error, .trace-empty {
    color: var(--text-dim, #888);
    text-align: center;
    padding: 20px;
  }
  .trace-error {
    color: #f87171;
  }
</style>
