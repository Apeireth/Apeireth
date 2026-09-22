<script lang="ts">
  // 治理卷宗 ④ 审计 —— 执行轨迹 × 事件留痕，主从双栏。
  //
  // 契约 §8：GET /v1/panel/traces(/:id) + GET /v1/panel/audit。
  // 与左轨「日志」的分工（复用而非另起炉灶）：日志 = 实时流 + 本地调用记录；
  // 这里 = 后端持久账的主从检索——轨迹树与留痕详情。span 树逻辑与
  // ActivityView 共用 lib/governance/trace.ts 一份实现。
  //
  // 空态即契约（原则 5）：写「当 X 发生时这里会出现 Y」。
  import {onMount} from 'svelte';
  import {RotateCcw} from 'lucide-svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import ErrorState from '../../components/ErrorState.svelte';
  import LoadingState from '../../components/LoadingState.svelte';
  import StatusBadge from '../../components/StatusBadge.svelte';
  import GovUnsupported from './GovUnsupported.svelte';
  import type {ActivityItem, ApeirethConfig, CapabilityManifest} from '../../types';
  import {
    capabilityAvailable,
    capabilityUnavailableReason,
    fetchAuditLogs,
    fetchTraceDetail,
    fetchTraceList,
    friendlyErrorMessage,
    type TraceSpanItem,
  } from '../../runtime';
  import {
    auditGroupChips,
    filterAuditItems,
    filterTraceRows,
    spanDepth,
    spanDurationMs,
    spanStatusBadge,
    spanTreeSort,
    traceFilterCounts,
    traceRowFromSummary,
    type TraceFilter,
    type TraceRow,
  } from '../../governance/trace';
  import {formatDateTimeMs, formatDurationMs, shortId} from '../../governance/format';

  let {
    config,
    capabilities = null,
  }: {
    config: ApeirethConfig;
    capabilities: CapabilityManifest | null;
  } = $props();

  const canTrace = $derived(capabilityAvailable(capabilities, 'trace.read'));
  const canAudit = $derived(capabilityAvailable(capabilities, 'audit.read'));
  const traceReason = $derived(capabilityUnavailableReason(capabilities, 'trace.read'));
  const auditReason = $derived(capabilityUnavailableReason(capabilities, 'audit.read'));

  type AuditSource = 'traces' | 'events';
  let source = $state<AuditSource>('traces');

  // ---- 轨迹 ----
  let traceRows = $state<TraceRow[]>([]);
  let traceError = $state('');
  let traceFilter = $state<TraceFilter>('all');
  let selectedTraceId = $state<string | null>(null);
  let traceSpans = $state<TraceSpanItem[]>([]);
  let traceDetailLoading = $state(false);
  let traceDetailError = $state('');

  // ---- 事件留痕 ----
  let auditItems = $state<ActivityItem[]>([]);
  let auditError = $state('');
  let auditGroup = $state<string | 'all'>('all');
  let selectedAuditId = $state<string | null>(null);

  let loading = $state(false);

  const traceCounts = $derived(traceFilterCounts(traceRows));
  const visibleTraces = $derived(filterTraceRows(traceRows, traceFilter));
  const auditChips = $derived(auditGroupChips(auditItems));
  const visibleAudit = $derived(filterAuditItems(auditItems, auditGroup));
  const selectedAudit = $derived(auditItems.find((i) => i.id === selectedAuditId) ?? null);

  async function load(): Promise<void> {
    loading = true;
    traceError = '';
    auditError = '';
    try {
      if (canTrace) {
        const r = await fetchTraceList(config, 50);
        if (Array.isArray(r)) {
          traceRows = r.map(traceRowFromSummary);
        } else {
          traceError = r.error;
        }
      }
      if (canAudit) {
        try {
          auditItems = await fetchAuditLogs(config, 100);
        } catch (e) {
          auditError = friendlyErrorMessage(e, '/v1/panel/audit');
        }
      }
    } finally {
      loading = false;
    }
  }

  async function selectTrace(row: TraceRow): Promise<void> {
    selectedTraceId = row.traceId;
    traceSpans = [];
    traceDetailError = '';
    traceDetailLoading = true;
    const r = await fetchTraceDetail(config, row.traceId);
    traceDetailLoading = false;
    if (Array.isArray(r)) traceSpans = spanTreeSort(r);
    else traceDetailError = r.error;
  }

  onMount(() => {
    void load();
  });
</script>

<div class="gov-tab" style="--gov-accent: var(--ap-register-deepops-yellow)">
  <p class="gov-position">
    审计是落笔之后的卷宗：每一轮执行的轨迹树、每一条系统留痕，都可在此检索复核。
  </p>

  {#if !canTrace && !canAudit}
    <GovUnsupported
      capabilityId="trace.read / audit.read"
      reason={traceReason ?? auditReason}
      hint="审计卷宗需要 canonical gateway 的 trace/audit 内省面。"
    />
  {:else}
    <!-- 数据源 chips（带计数） -->
    <div class="gov-toolbar">
      <div class="gov-chips" role="tablist" aria-label="审计数据源">
        <button class="gov-chip" class:on={source === 'traces'} onclick={() => (source = 'traces')}>
          执行轨迹 <span class="chip-n">{traceRows.length}</span>
        </button>
        <button class="gov-chip" class:on={source === 'events'} onclick={() => (source = 'events')}>
          事件留痕 <span class="chip-n">{auditItems.length}</span>
        </button>
      </div>
      <button class="gov-quiet" onclick={() => void load()} disabled={loading}>
        <RotateCcw size={13} class={loading ? 'gov-spin' : ''} />
        刷新
      </button>
    </div>

    {#if source === 'traces' && !canTrace}
      <GovUnsupported capabilityId="trace.read" reason={traceReason} />
    {:else if source === 'events' && !canAudit}
      <GovUnsupported capabilityId="audit.read" reason={auditReason} />
    {:else}
      <!-- 二级过滤 chips -->
      {#if source === 'traces'}
        <div class="gov-chips sub" role="tablist" aria-label="轨迹过滤">
          <button class="gov-chip sm" class:on={traceFilter === 'all'} onclick={() => (traceFilter = 'all')}>
            全部 <span class="chip-n">{traceCounts.all}</span>
          </button>
          <button class="gov-chip sm" class:on={traceFilter === 'error'} onclick={() => (traceFilter = 'error')}>
            有异常 <span class="chip-n">{traceCounts.error}</span>
          </button>
        </div>
      {:else}
        <div class="gov-chips sub" role="tablist" aria-label="留痕分组">
          <button class="gov-chip sm" class:on={auditGroup === 'all'} onclick={() => (auditGroup = 'all')}>
            全部 <span class="chip-n">{auditItems.length}</span>
          </button>
          {#each auditChips as chip (chip.group)}
            <button class="gov-chip sm" class:on={auditGroup === chip.group} onclick={() => (auditGroup = chip.group)}>
              {chip.group} <span class="chip-n">{chip.count}</span>
            </button>
          {/each}
        </div>
      {/if}

      <!-- 主从双栏 -->
      <div class="md-split">
        <div class="md-master">
          {#if loading && !traceRows.length && !auditItems.length}
            <LoadingState message="正在读取审计账…" />
          {:else if source === 'traces' && traceError && !traceRows.length}
            <ErrorState title="拉取轨迹失败" message={traceError} onRetry={() => void load()} />
          {:else if source === 'events' && auditError && !auditItems.length}
            <ErrorState title="拉取留痕失败" message={auditError} onRetry={() => void load()} />
          {:else if source === 'traces' && !visibleTraces.length}
            <EmptyState
              icon="⌘"
              title={traceFilter === 'error' ? '没有带异常的轨迹' : '暂无执行轨迹'}
              description="当一轮对话或一次工具调用完成时，它的完整执行轨迹（回合 → provider → 工具 → 治理）会记在这里。"
            />
          {:else if source === 'events' && !visibleAudit.length}
            <EmptyState
              icon="⌘"
              title={auditGroup === 'all' ? '暂无事件留痕' : `没有 ${auditGroup} 组留痕`}
              description="当回合完成、审批落笔、记忆变动时，运行时审计端口会把事件归档在这里。"
            />
          {:else if source === 'traces'}
            {#each visibleTraces as row (row.traceId)}
              <button
                class="md-row"
                class:on={selectedTraceId === row.traceId}
                onclick={() => void selectTrace(row)}
              >
                <span class="md-row-top">
                  <span class="md-row-title">{row.rootSummary || `${row.rootKind || 'trace'} · ${shortId(row.traceId)}`}</span>
                  <StatusBadge
                    variant={row.hasError ? 'danger' : 'green'}
                    label={row.hasError ? 'error' : row.rootStatus || 'ok'}
                    size="small"
                  />
                </span>
                <span class="md-row-sub mono">
                  {row.spanCount} spans · {formatDateTimeMs(row.startedAtMs)}
                </span>
              </button>
            {/each}
          {:else}
            {#each visibleAudit as item (item.id)}
              <button
                class="md-row"
                class:on={selectedAuditId === item.id}
                onclick={() => (selectedAuditId = item.id)}
              >
                <span class="md-row-top">
                  <span class="md-row-title mono">{item.title}</span>
                  <StatusBadge variant="blue" label={item.source} size="small" />
                </span>
                <span class="md-row-sub mono">{formatDateTimeMs(item.timestamp)}</span>
              </button>
            {/each}
          {/if}
        </div>

        <div class="md-detail">
          {#if source === 'traces'}
            {#if !selectedTraceId}
              <div class="md-placeholder">
                <p>选择左侧一条轨迹，这里会出现它的完整 span 树——每一步的参与者、状态、耗时。</p>
              </div>
            {:else if traceDetailLoading}
              <LoadingState message="正在展开轨迹…" />
            {:else if traceDetailError}
              <ErrorState title="轨迹详情加载失败" message={traceDetailError} onRetry={() => {
                const row = traceRows.find((r) => r.traceId === selectedTraceId);
                if (row) void selectTrace(row);
              }} />
            {:else if !traceSpans.length}
              <div class="md-placeholder">
                <p>该轨迹无 span 记录。</p>
              </div>
            {:else}
              <div class="span-head mono">
                trace {shortId(selectedTraceId, 12)} · {traceSpans.length} spans
              </div>
              <div class="span-tree">
                {#each traceSpans as span (span.span_id)}
                  {@const badge = spanStatusBadge(span.status)}
                  {@const dur = spanDurationMs(span)}
                  <div class="span-row" style="padding-left: {spanDepth(traceSpans, span) * 18}px">
                    <span class="span-kind mono">{span.kind}</span>
                    <span class="span-actor">{span.actor}</span>
                    <StatusBadge variant={badge.variant} label={badge.label} size="small" />
                    <span class="span-dur mono">{dur === null ? '进行中' : formatDurationMs(dur)}</span>
                  </div>
                  {#if span.summary}
                    <div class="span-summary" style="padding-left: {spanDepth(traceSpans, span) * 18}px">
                      {span.summary}
                    </div>
                  {/if}
                {/each}
              </div>
            {/if}
          {:else}
            {#if !selectedAudit}
              <div class="md-placeholder">
                <p>选择左侧一条留痕，这里会出现它的完整字段与原文。</p>
              </div>
            {:else}
              <div class="audit-detail">
                <div class="ad-row">
                  <span class="ad-lbl">事件</span>
                  <span class="ad-val mono">{selectedAudit.title}</span>
                </div>
                <div class="ad-row">
                  <span class="ad-lbl">时刻</span>
                  <span class="ad-val mono">{formatDateTimeMs(selectedAudit.timestamp)}</span>
                </div>
                {#if selectedAudit.summary && selectedAudit.summary !== selectedAudit.title}
                  <div class="ad-row">
                    <span class="ad-lbl">摘要</span>
                    <span class="ad-val">{selectedAudit.summary}</span>
                  </div>
                {/if}
                {#if selectedAudit.detail}
                  <div class="ad-row col">
                    <span class="ad-lbl">原始记录</span>
                    <pre class="ad-pre mono">{selectedAudit.detail}</pre>
                  </div>
                {/if}
              </div>
            {/if}
          {/if}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .gov-tab {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    flex: 1;
  }
  .gov-position {
    margin: 0;
    font-size: 11px;
    line-height: 1.7;
    color: var(--ap-bone-30);
    letter-spacing: 0.02em;
  }
  .mono {
    font-family: var(--ap-font-mono);
  }
  .gov-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .gov-chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .gov-chips.sub {
    margin-top: -4px;
  }
  .gov-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 11px;
    border-radius: 999px;
    border: 1px solid var(--ap-line);
    background: transparent;
    color: var(--ap-bone-42);
    font-size: 11.5px;
    cursor: pointer;
    transition: all 0.18s ease;
  }
  .gov-chip.sm {
    padding: 2px 9px;
    font-size: 10.5px;
  }
  .gov-chip:hover {
    color: var(--ap-bone);
  }
  .gov-chip.on {
    border-color: color-mix(in srgb, var(--gov-accent) 55%, transparent);
    background: color-mix(in srgb, var(--gov-accent) 12%, transparent);
    color: var(--ap-bone);
  }
  .chip-n {
    font-family: var(--ap-font-mono);
    font-size: 10px;
    color: var(--gov-accent);
  }
  .gov-quiet {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--ap-line);
    background: transparent;
    color: var(--ap-bone-42);
    font-size: 11.5px;
    cursor: pointer;
  }
  .gov-quiet:hover:not(:disabled) {
    color: var(--ap-bone);
    border-color: var(--ap-bone-30);
  }
  .gov-quiet:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* 主从双栏 */
  .md-split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(220px, 2fr) minmax(0, 3fr);
    gap: 12px;
  }
  .md-master {
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow-y: auto;
    min-height: 0;
    padding-bottom: 8px;
  }
  .md-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    text-align: left;
    padding: 9px 12px;
    border-radius: 8px;
    border: 1px solid var(--ap-line);
    border-left: 2px solid transparent;
    background: rgba(5, 10, 15, 0.35);
    cursor: pointer;
    transition: border-color 0.15s ease;
  }
  .md-row:hover {
    border-color: var(--ap-bone-30);
  }
  .md-row.on {
    border-left-color: var(--gov-accent);
    background: color-mix(in srgb, var(--gov-accent) 8%, rgba(5, 10, 15, 0.35));
  }
  .md-row-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .md-row-title {
    font-size: 12px;
    color: var(--ap-bone);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .md-row-sub {
    font-size: 10px;
    color: var(--ap-bone-30);
  }
  .md-detail {
    min-height: 0;
    overflow-y: auto;
    border: 1px solid var(--ap-line);
    border-radius: 8px;
    background: rgba(5, 10, 15, 0.25);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .md-placeholder {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--ap-bone-30);
    font-size: 11.5px;
    line-height: 1.8;
    text-align: center;
    padding: 24px;
  }
  .md-placeholder p {
    margin: 0;
    max-width: 34ch;
  }
  .span-head {
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--ap-bone-30);
    border-bottom: 1px solid var(--ap-line);
    padding-bottom: 8px;
  }
  .span-tree {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .span-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 4px;
    padding-bottom: 4px;
  }
  .span-kind {
    font-size: 10px;
    color: var(--gov-accent);
    min-width: 66px;
  }
  .span-actor {
    font-size: 11px;
    color: var(--ap-bone-68);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .span-dur {
    margin-left: auto;
    font-size: 10px;
    color: var(--ap-bone-30);
    flex: none;
  }
  .span-summary {
    font-size: 10.5px;
    color: var(--ap-bone-42);
    line-height: 1.5;
    padding-bottom: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .audit-detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .ad-row {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .ad-row.col {
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }
  .ad-lbl {
    flex: none;
    width: 44px;
    font-size: 10.5px;
    color: var(--ap-bone-30);
  }
  .ad-val {
    font-size: 12px;
    color: var(--ap-bone);
    word-break: break-all;
  }
  .ad-pre {
    margin: 0;
    font-size: 10.5px;
    line-height: 1.6;
    color: var(--ap-bone-68);
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--ap-line);
    border-radius: 6px;
    padding: 8px 10px;
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 320px;
    overflow-y: auto;
  }

  :global(.gov-spin) {
    animation: gov-rot 1s linear infinite;
  }
  @keyframes gov-rot {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.gov-spin) {
      animation: none;
    }
  }

  @media (max-width: 860px) {
    .md-split {
      grid-template-columns: 1fr;
    }
  }
</style>
