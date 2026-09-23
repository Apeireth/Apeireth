<script lang="ts">
  // 治理卷宗 ① 审批收件箱 —— 「账」：全局待批总账。
  //
  // 定位（00-PHILOSOPHY §3.2/§4）：对话内的金色待签文书卡是「当下」，
  // 在发生处完成签批；这里是「账」——所有会话当前待批的总账。
  // 诚实边界：后端没有已批/已拒历史端点，本 tab 只承载当前待签；
  // 已落笔的记录请去审计 tab。
  //
  // 数据：GET /v1/panel/sessions（会话清单）⋈ GET /v1/approvals?session=
  // （逐会话 inbox，契约 §6）；SSE approval_required/approval_resolved
  // 实时推入（契约 §8，required 帧只当脏标记重拉，不拿帧拼假卡）。
  import {onMount, onDestroy} from 'svelte';
  import {Inbox, RotateCcw, Radio, MessageCircleMore, ScrollText, Clock} from 'lucide-svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import GovManifestPending from './GovManifestPending.svelte';
  import ErrorState from '../../components/ErrorState.svelte';
  import LoadingState from '../../components/LoadingState.svelte';
  import StatusBadge from '../../components/StatusBadge.svelte';
  import GovUnsupported from './GovUnsupported.svelte';
  import type {ApeirethConfig, CapabilityManifest} from '../../types';
  import type {CanonicalPendingApproval} from '../../runtime';
  import {
    capabilityAvailable,
    capabilityUnavailableReason,
    fetchBackendSessions,
    fetchCanonicalApprovals,
    friendlyErrorMessage,
    resolveCanonicalApproval,
  } from '../../runtime';
  import {parseApprovalEventPayload} from '../../chat-shell/gateway-events';
  import {
    applyApprovalEventToLedger,
    approvalFilterCounts,
    filterApprovalLedger,
    isApprovalExpired,
    mergeSessionInbox,
    nextRetryDelay,
    nextSseState,
    SSE_INITIAL_RETRY_MS,
    SSE_STATE_LABEL,
    type ApprovalFilter,
    type ApprovalLedgerEntry,
    type SseConnectionState,
    type SseSignal,
  } from '../../governance/ledger';
  import {formatDateTimeMs, formatRelativeMs, shortId} from '../../governance/format';

  let {
    config,
    capabilities = null,
    onOpenChat,
    onGoAudit,
  }: {
    config: ApeirethConfig;
    capabilities: CapabilityManifest | null;
    /** 引导卡主卡：回到对话（对话内的金色待签文书是「当下」）。 */
    onOpenChat: () => void;
    /** 引导卡次卡：去看审计（已落笔的记录）。 */
    onGoAudit: () => void;
  } = $props();

  // ---- 能力门控（0 装：不支持就显式标注）----
  const canRead = $derived(capabilityAvailable(capabilities, 'permissions.approval.read'));
  const canResolve = $derived(capabilityAvailable(capabilities, 'permissions.approval.resolve'));
  const canListSessions = $derived(capabilityAvailable(capabilities, 'sessions.read'));
  const canSse = $derived(capabilityAvailable(capabilities, 'activity.sse'));
  const readReason = $derived(capabilityUnavailableReason(capabilities, 'permissions.approval.read'));
  const resolveReason = $derived(capabilityUnavailableReason(capabilities, 'permissions.approval.resolve'));

  let ledger = $state<ApprovalLedgerEntry[]>([]);
  let loading = $state(false);
  let error = $state('');
  let filter = $state<ApprovalFilter>('all');
  /** SSE 学到的、会话清单之外的会话 id（sessions.read 缺席时的诚实兜底）。 */
  let knownSessionIds = new Set<string>();
  let sessionTitles = new Map<string, string>();

  // ---- SSE 真实连接态（断连标断连，不装在线）----
  let sseState = $state<SseConnectionState>('connecting');
  let es: EventSource | null = null;
  let retryMs = SSE_INITIAL_RETRY_MS;
  let retryTimer: ReturnType<typeof setTimeout> | null = null;
  let destroyed = false;

  // ---- 签批操作态 ----
  let resolvingId = $state<string | null>(null);
  let resolveError = $state('');
  let resolveNote = $state('');
  let noteTimer: ReturnType<typeof setTimeout> | null = null;

  let nowMs = $state(Date.now());
  let clockTimer: ReturnType<typeof setInterval> | null = null;

  const counts = $derived(approvalFilterCounts(ledger, nowMs));
  const visible = $derived(filterApprovalLedger(ledger, filter, nowMs));

  function showNote(text: string): void {
    resolveNote = text;
    if (noteTimer) clearTimeout(noteTimer);
    noteTimer = setTimeout(() => (resolveNote = ''), 6000);
  }

  /** 逐会话 inbox 重拉（SSE required 帧的唯一诚实响应）。 */
  async function refreshSessionInbox(sessionId: string): Promise<void> {
    const inbox = await fetchCanonicalApprovals(config, sessionId).catch(() => null);
    if (inbox === null) return; // 网络失败不碰账本（旧账留着比清空诚实）
    ledger = mergeSessionInbox(ledger, sessionId, inbox, sessionTitles.get(sessionId));
  }

  async function loadLedger(): Promise<void> {
    if (!canRead) return;
    loading = true;
    error = '';
    try {
      if (canListSessions) {
        const sessions = await fetchBackendSessions(config).catch(() => []);
        sessionTitles = new Map(
          sessions.map((s) => [s.id, s.title ?? ''] as const).filter(([, t]) => t),
        );
        for (const s of sessions) knownSessionIds.add(s.id);
        knownSessionIds = new Set(knownSessionIds);
      }
      const ids = [...knownSessionIds];
      // 分批拉取（本机回环，批 6 足够温和）
      for (let i = 0; i < ids.length; i += 6) {
        const batch = ids.slice(i, i + 6);
        const results = await Promise.all(
          batch.map((id) => fetchCanonicalApprovals(config, id).catch(() => null)),
        );
        batch.forEach((id, j) => {
          const inbox = results[j];
          if (inbox !== null) {
            ledger = mergeSessionInbox(ledger, id, inbox, sessionTitles.get(id));
          }
        });
      }
    } catch (e) {
      error = friendlyErrorMessage(e, '/v1/approvals');
    } finally {
      loading = false;
    }
  }

  function handleApprovalEvent(kind: 'approval_required' | 'approval_resolved', msg: MessageEvent): void {
    let payload: unknown = null;
    try {
      payload = JSON.parse(typeof msg.data === 'string' ? msg.data : '');
    } catch {
      payload = null;
    }
    const info = parseApprovalEventPayload(payload);
    if (!info) {
      // 帧无法路由 → 保守全量重拉（0 装：不猜）
      void loadLedger();
      return;
    }
    const r = applyApprovalEventToLedger(ledger, info, kind);
    ledger = r.entries;
    for (const sid of r.dirtySessions) {
      if (!knownSessionIds.has(sid)) {
        knownSessionIds.add(sid);
        knownSessionIds = new Set(knownSessionIds);
      }
      void refreshSessionInbox(sid);
    }
  }

  function sseSignal(signal: SseSignal): void {
    sseState = nextSseState(sseState, signal);
  }

  function startSse(): void {
    if (destroyed) return;
    if (!canSse || typeof EventSource === 'undefined') {
      sseSignal('unsupported');
      return;
    }
    sseState = 'connecting';
    const base = config.baseUrl.replace(/\/+$/, '');
    const source = new EventSource(`${base}/v1/apeireth/events`);
    es = source;
    source.addEventListener('approval_required', (m) =>
      handleApprovalEvent('approval_required', m as MessageEvent),
    );
    source.addEventListener('approval_resolved', (m) =>
      handleApprovalEvent('approval_resolved', m as MessageEvent),
    );
    source.onopen = () => {
      if (es !== source) return;
      retryMs = SSE_INITIAL_RETRY_MS;
      sseSignal('open');
    };
    source.onerror = () => {
      if (es === source) es = null;
      source.close();
      if (destroyed) return;
      sseSignal('error');
      retryTimer = setTimeout(() => {
        retryTimer = null;
        startSse();
      }, retryMs);
      retryMs = nextRetryDelay(retryMs);
    };
  }

  async function resolveEntry(entry: ApprovalLedgerEntry, decision: 'approve' | 'reject'): Promise<void> {
    if (!canResolve || resolvingId) return;
    const id = entry.approval.approval_id;
    resolvingId = id;
    resolveError = '';
    try {
      const result = await resolveCanonicalApproval(config, entry.approval, decision);
      if (result.kind === 'completed') {
        ledger = ledger.filter((e) => e.approval.approval_id !== id);
        showNote(
          decision === 'approve'
            ? `已批准 ${entry.approval.tool_name} · 回合在对话中继续`
            : `已拒绝 ${entry.approval.tool_name} · 回合已在发生处收尾`,
        );
      } else {
        // 链式审批：签完一道，下一道入账（契约 §6：resolve 后继续原回合）
        ledger = mergeSessionInbox(ledger, entry.approval.session, [result.pending], entry.sessionTitle);
        showNote('文书已签 · 同一回合的下一道审批已入账');
      }
    } catch (e) {
      resolveError = friendlyErrorMessage(e, '/v1/approvals/resolve');
    } finally {
      resolvingId = null;
    }
  }

  function expiryLabel(entry: ApprovalLedgerEntry): string {
    if (entry.expiresAtMs === undefined) return '';
    if (isApprovalExpired(entry, nowMs)) return '已过期';
    const leftMin = Math.max(1, Math.round((entry.expiresAtMs - nowMs) / 60000));
    return `${leftMin} 分钟后过期`;
  }

  // 能力门控是异步到达的（manifest 在 App 启动节拍里拉取）：drawer 可能在
  // capabilities=null 时挂载，onMount 一次性早退会永久停在空态——首载改由
  // 能力翻转到位的 effect 触发（一次性闸，重复翻正不重复拉）。
  let loadStarted = false;
  $effect(() => {
    if (!canRead || loadStarted) return;
    loadStarted = true;
    void loadLedger();
  });

  let sseStarted = false;
  $effect(() => {
    if (capabilities === null || sseStarted || destroyed) return;
    sseStarted = true;
    startSse(); // 内部对 !canSse / 无 EventSource 置 unsupported
  });

  onMount(() => {
    clockTimer = setInterval(() => (nowMs = Date.now()), 30000);
  });

  onDestroy(() => {
    destroyed = true;
    if (retryTimer) clearTimeout(retryTimer);
    if (noteTimer) clearTimeout(noteTimer);
    if (clockTimer) clearInterval(clockTimer);
    es?.close();
    es = null;
  });
</script>

<div class="gov-tab" style="--gov-accent: var(--ap-register-deepops-blue)">
  <p class="gov-position">
    审批在对话内完成——对话里的金色待签文书是「当下」；这里是账：全局待批的总账。已落笔的记录在审计。
  </p>

  {#if capabilities === null}
    <!-- 清单未到达 ≠ 不支持：health→capabilities 串行拉取进行中；离线超时给诚实说明 -->
    <GovManifestPending message="正在读取运行时能力清单…" />
  {:else if !canRead}
    <GovUnsupported
      capabilityId="permissions.approval.read"
      reason={readReason}
      hint="审批收件箱需要 canonical gateway 的审批内省面。"
    />
  {:else}
    <div class="gov-toolbar">
      <div class="gov-chips" role="tablist" aria-label="审批过滤">
        <button class="gov-chip" class:on={filter === 'all'} onclick={() => (filter = 'all')}>
          全部 <span class="chip-n">{counts.all}</span>
        </button>
        <button class="gov-chip" class:on={filter === 'open'} onclick={() => (filter = 'open')}>
          待签 <span class="chip-n">{counts.open}</span>
        </button>
        <button class="gov-chip" class:on={filter === 'expired'} onclick={() => (filter = 'expired')}>
          已过期 <span class="chip-n">{counts.expired}</span>
        </button>
      </div>
      <div class="gov-toolbar-right">
        <span class="sse-pill" class:live={sseState === 'live'} class:down={sseState === 'reconnecting'}>
          <Radio size={11} />
          {SSE_STATE_LABEL[sseState]}
        </span>
        <button class="gov-quiet" onclick={() => void loadLedger()} disabled={loading} title="重新对齐后端账本">
          <RotateCcw size={13} class={loading ? 'gov-spin' : ''} />
          刷新
        </button>
      </div>
    </div>

    {#if !canListSessions}
      <p class="gov-note">
        会话清单能力 <code>sessions.read</code> 不可用——账本只覆盖本次运行中观测到审批事件的会话。
      </p>
    {/if}
    {#if resolveNote}
      <p class="gov-note ok">{resolveNote}</p>
    {/if}
    {#if resolveError}
      <p class="gov-note bad">{resolveError}</p>
    {/if}

    {#if loading && !ledger.length}
      <LoadingState message="正在对齐各会话的待批账本…" />
    {:else if error && !ledger.length}
      <ErrorState title="拉取审批账本失败" message={error} onRetry={() => void loadLedger()} />
    {:else if !visible.length}
      {#if ledger.length === 0 && filter === 'all'}
        <!-- 空态即契约（原则 5）+ 双引导卡（gap-plan §4.4 组件 10） -->
        <EmptyState
          icon="✒"
          title="账上无待签"
          description="当他在对话里停下、把一份文书递到你手边时，这里会同时记下一笔账——何时、哪个会话、哪件工具、为什么。"
        >
          <div class="gov-guides">
            <button class="guide-card primary" onclick={onOpenChat}>
              <span class="guide-ember" aria-hidden="true"></span>
              <span class="guide-title">去对话</span>
              <span class="guide-sub">让他在对话里跑一次需要批准的工具调用，金色待签文书会在发生处浮起</span>
            </button>
            <button class="guide-card" onclick={onGoAudit}>
              <ScrollText size={15} />
              <span class="guide-title">看审计</span>
              <span class="guide-sub">已落笔的批准与拒绝，在审计卷里留痕</span>
            </button>
          </div>
        </EmptyState>
      {:else}
        <EmptyState
          icon="✒"
          title="此过滤下无文书"
          description={filter === 'expired' ? '过期待签会记在这里——当前没有。' : '当前没有等待签字的文书。'}
        />
      {/if}
    {:else}
      <div class="gov-list">
        {#each visible as entry (entry.approval.approval_id)}
          {@const expired = isApprovalExpired(entry, nowMs)}
          {@const busy = resolvingId === entry.approval.approval_id}
          <article class="gov-card" class:faded={expired}>
            <header class="gov-card-head">
              <span class="gov-card-title">{entry.approval.tool_name}</span>
              <StatusBadge
                variant={expired ? 'dim' : 'amber'}
                label={expired ? '已过期' : '待签'}
                size="small"
              />
            </header>
            <p class="gov-reason">{entry.approval.governance_reason || '治理面要求主人批准'}</p>
            {#if entry.approval.command_text}
              <code class="gov-command">{entry.approval.command_text}</code>
            {/if}
            {#if entry.approval.arguments_summary && entry.approval.arguments_summary !== entry.approval.command_text}
              <p class="gov-args">参数摘要：{entry.approval.arguments_summary}</p>
            {/if}
            <footer class="gov-card-foot">
              <span class="gov-meta" title={entry.approval.session}>
                {entry.sessionTitle || `会话 ${shortId(entry.approval.session)}`}
              </span>
              {#if entry.createdAtMs !== undefined}
                <span class="gov-meta mono">{formatDateTimeMs(entry.createdAtMs)}</span>
              {/if}
              {#if entry.expiresAtMs !== undefined}
                <span class="gov-meta mono" class:warn={!expired}>
                  <Clock size={10} />
                  {expiryLabel(entry)}
                </span>
              {/if}
              <span class="gov-actions">
                <button
                  class="btn-reject"
                  disabled={!canResolve || expired || resolvingId !== null}
                  title={!canResolve ? '当前运行时不支持 permissions.approval.resolve' : expired ? '文书已过期，等后端清理即可' : ''}
                  onclick={() => void resolveEntry(entry, 'reject')}
                >
                  拒绝
                </button>
                <button
                  class="btn-approve"
                  disabled={!canResolve || expired || resolvingId !== null}
                  title={!canResolve ? '当前运行时不支持 permissions.approval.resolve' : expired ? '文书已过期，等后端清理即可' : ''}
                  onclick={() => void resolveEntry(entry, 'approve')}
                >
                  {busy ? '签批中…' : '批准'}
                </button>
              </span>
            </footer>
          </article>
        {/each}
      </div>
      {#if !canResolve}
        <GovUnsupported
          capabilityId="permissions.approval.resolve"
          reason={resolveReason}
          hint="账本可读，但签批动作不可用。"
        />
      {/if}
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
  .gov-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }
  .gov-chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
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
  .gov-chip:hover {
    color: var(--ap-bone);
  }
  /* 选中 chip 用模块识别色（金色纪律：卷宗内不出现金选中态） */
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
  .gov-toolbar-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .sse-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--ap-bone-30);
    padding: 3px 9px;
    border-radius: 999px;
    border: 1px solid var(--ap-line);
  }
  .sse-pill.live {
    color: var(--ap-semantic-success);
    border-color: rgba(127, 184, 148, 0.35);
  }
  .sse-pill.down {
    color: var(--ap-semantic-warning);
    border-color: rgba(217, 162, 74, 0.4);
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
  .gov-note {
    margin: 0;
    font-size: 11px;
    color: var(--ap-bone-42);
    line-height: 1.6;
  }
  .gov-note code {
    font-family: var(--ap-font-mono);
    font-size: 10px;
    background: rgba(0, 0, 0, 0.3);
    padding: 1px 5px;
    border-radius: 3px;
  }
  .gov-note.ok {
    color: var(--ap-semantic-success);
  }
  .gov-note.bad {
    color: var(--ap-semantic-danger);
  }
  .gov-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
    min-height: 0;
    padding-bottom: 8px;
  }
  .gov-card {
    border: 1px solid var(--ap-line);
    border-left: 2px solid var(--gov-accent);
    border-radius: 8px;
    background: rgba(5, 10, 15, 0.4);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .gov-card.faded {
    opacity: 0.55;
  }
  .gov-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .gov-card-title {
    font-family: var(--ap-font-mono);
    font-size: 12.5px;
    color: var(--ap-bone);
    letter-spacing: 0.04em;
  }
  .gov-reason {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.6;
    color: var(--ap-bone-68);
  }
  .gov-command {
    display: block;
    font-family: var(--ap-font-mono);
    font-size: 11px;
    line-height: 1.5;
    color: var(--ap-bone);
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--ap-line);
    border-radius: 6px;
    padding: 7px 10px;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .gov-args {
    margin: 0;
    font-size: 11px;
    color: var(--ap-bone-42);
    line-height: 1.6;
  }
  .gov-card-foot {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .gov-meta {
    font-size: 10.5px;
    color: var(--ap-bone-30);
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .gov-meta.mono {
    font-family: var(--ap-font-mono);
  }
  .gov-meta.warn {
    color: var(--ap-semantic-warning);
  }
  .gov-actions {
    margin-left: auto;
    display: inline-flex;
    gap: 8px;
  }
  .btn-reject,
  .btn-approve {
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn-reject {
    background: transparent;
    border: 1px solid var(--ap-line);
    color: var(--ap-bone-42);
  }
  .btn-reject:hover:not(:disabled) {
    color: var(--ap-bone);
    background: rgba(232, 224, 204, 0.06);
  }
  /* 批准 = 签字动作，用模块蓝（卷宗内不用金——金只在对话内文书上） */
  .btn-approve {
    background: color-mix(in srgb, var(--gov-accent) 88%, black);
    border: 1px solid var(--gov-accent);
    color: #f4f8fb;
    font-weight: 600;
  }
  .btn-approve:hover:not(:disabled) {
    filter: brightness(1.12);
  }
  .btn-reject:disabled,
  .btn-approve:disabled {
    opacity: 0.45;
    cursor: default;
  }

  /* 双引导卡（空态）：主卡一点金 = 指向对话内的金色文书（卷宗内唯一合法的金） */
  .gov-guides {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .guide-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    width: 240px;
    text-align: left;
    padding: 14px 16px;
    border-radius: 10px;
    border: 1px solid var(--ap-line);
    background: rgba(5, 10, 15, 0.5);
    color: var(--ap-bone-42);
    cursor: pointer;
    transition: border-color 0.2s ease;
  }
  .guide-card:hover {
    border-color: var(--ap-bone-30);
  }
  .guide-card.primary {
    border-color: rgba(255, 210, 122, 0.3);
  }
  .guide-card.primary:hover {
    border-color: rgba(255, 210, 122, 0.55);
  }
  .guide-ember {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ap-gold);
    box-shadow: 0 0 8px rgba(255, 210, 122, 0.55);
  }
  .guide-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--ap-bone);
  }
  .guide-sub {
    font-size: 11px;
    line-height: 1.6;
    color: var(--ap-bone-42);
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
</style>
