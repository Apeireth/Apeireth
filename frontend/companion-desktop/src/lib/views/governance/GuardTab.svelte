<script lang="ts">
  // 治理卷宗 ③ 守卫 —— guard status / events / dry-run 试算。
  //
  // 契约：`crates/engine/guard/src/introspection.rs` DTO 逐字段核实；
  // 端点 /v1/safety/guard/status|events|evaluate，能力 id
  // safety.guard.status.read / safety.guard.events.read / safety.guard.evaluate。
  //
  // dry-run 是「试算」：只评估不执行——文案必须写明，不许让用户以为
  // 这里能真跑一次工具调用。守卫否决的实时通报当前不走 SSE 总线
  // （chat-shell/gateway-events.ts 头注的诚实断点），本 tab 是事后账，
  // 不轮询伪造节奏——刷新由用户手动触发。
  import {RotateCcw, FlaskConical, ChevronDown, ChevronRight} from 'lucide-svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import ErrorState from '../../components/ErrorState.svelte';
  import LoadingState from '../../components/LoadingState.svelte';
  import StatusBadge from '../../components/StatusBadge.svelte';
  import GovUnsupported from './GovUnsupported.svelte';
  import GovManifestPending from './GovManifestPending.svelte';
  import type {ApeirethConfig, CapabilityManifest, GuardEvent, GuardStatus} from '../../types';
  import {
    capabilityAvailable,
    capabilityUnavailableReason,
    evaluateGuard,
    fetchGuardEvents,
    fetchGuardStatus,
  } from '../../runtime';
  import {
    dryRunResultRow,
    guardEventRow,
    guardTally,
    parseDryRunArguments,
    validCapabilityId,
  } from '../../governance/guard';
  import {formatClockMs, shortId} from '../../governance/format';

  let {
    config,
    capabilities = null,
  }: {
    config: ApeirethConfig;
    capabilities: CapabilityManifest | null;
  } = $props();

  const canStatus = $derived(capabilityAvailable(capabilities, 'safety.guard.status.read'));
  const canEvents = $derived(capabilityAvailable(capabilities, 'safety.guard.events.read'));
  const canEvaluate = $derived(capabilityAvailable(capabilities, 'safety.guard.evaluate'));
  const statusReason = $derived(capabilityUnavailableReason(capabilities, 'safety.guard.status.read'));
  const eventsReason = $derived(capabilityUnavailableReason(capabilities, 'safety.guard.events.read'));
  const evaluateReason = $derived(capabilityUnavailableReason(capabilities, 'safety.guard.evaluate'));
  const anyGuard = $derived(canStatus || canEvents || canEvaluate);

  let status = $state<GuardStatus | null>(null);
  let statusError = $state('');
  let events = $state<GuardEvent[]>([]);
  let eventsError = $state('');
  let loading = $state(false);
  let expandedIds = $state<Record<string, boolean>>({});

  // ---- dry-run 试算 ----
  let dryCapability = $state('');
  let dryArguments = $state('{}');
  let dryScope = $state('');
  let dryBusy = $state(false);
  let dryError = $state('');
  let dryResult = $state<ReturnType<typeof dryRunResultRow> | null>(null);
  let dryRaw = $state<{reasons: string[]; evidence: string[]} | null>(null);

  const tally = $derived(status ? guardTally(status) : null);

  async function load(): Promise<void> {
    loading = true;
    statusError = '';
    eventsError = '';
    try {
      if (canStatus) {
        const r = await fetchGuardStatus(config);
        if ('error' in r) statusError = r.error;
        else status = r;
      }
      if (canEvents) {
        const r = await fetchGuardEvents(config, 50);
        if ('error' in (r as {error?: string})) eventsError = (r as {error: string}).error;
        else events = r as GuardEvent[];
      }
    } finally {
      loading = false;
    }
  }

  function toggleExpand(id: string): void {
    expandedIds = {...expandedIds, [id]: !expandedIds[id]};
  }

  async function runDry(): Promise<void> {
    dryError = '';
    dryResult = null;
    dryRaw = null;
    if (!validCapabilityId(dryCapability)) {
      dryError = '能力 id 不能为空、不含空白（形如 tool.repo / memory.read）。';
      return;
    }
    const parsed = parseDryRunArguments(dryArguments);
    if (!parsed.ok) {
      dryError = parsed.error;
      return;
    }
    dryBusy = true;
    const r = await evaluateGuard(config, {
      capability_id: dryCapability.trim(),
      arguments: parsed.value,
      ...(dryScope.trim() ? {declared_scope: dryScope.trim()} : {}),
    });
    dryBusy = false;
    if ('error' in r) {
      dryError = r.error;
      return;
    }
    dryResult = dryRunResultRow(r);
    dryRaw = {reasons: r.reasons, evidence: r.evidence};
  }

  // 能力异步到达：首载由任一守卫能力翻正触发（一次性闸）。
  let loadStarted = false;
  $effect(() => {
    if (!anyGuard || loadStarted) return;
    loadStarted = true;
    void load();
  });
</script>

<div class="gov-tab" style="--gov-accent: var(--ap-register-deepops-red)">
  <p class="gov-position">
    守卫的账：每一次评估、放行、否决、转人工都记在这里。守卫的实时通报不进本页——否决当下在对话内呈现。
  </p>

  {#if capabilities === null}
    <GovManifestPending message="正在读取运行时能力清单…" />
  {:else if !anyGuard}
    <GovUnsupported
      capabilityId="safety.guard.*"
      reason={statusReason ?? eventsReason ?? evaluateReason}
      hint="守卫内省面需要 canonical gateway 装配 BehaviorChainGuardHook。"
    />
  {:else}
    <!-- 状态条 -->
    {#if canStatus}
      {#if status}
        <div class="guard-status">
          <div class="gs-row">
            <StatusBadge variant={status.enabled ? 'green' : 'dim'} label={status.enabled ? '守卫启用' : '守卫停用'} />
            <StatusBadge variant={status.fast_guard_active ? 'blue' : 'dim'} label="快速守卫" size="small" />
            <StatusBadge variant={status.chain_guard_active ? 'blue' : 'dim'} label="链式守卫" size="small" />
            {#if status.intent_guard_active}
              <StatusBadge variant="blue" label="意图守卫" size="small" />
            {/if}
            {#if status.cross_turn_monitoring_active}
              <StatusBadge variant="blue" label="跨轮监测" size="small" />
            {/if}
            <StatusBadge
              variant={status.ml_classifier_available ? 'green' : 'dim'}
              label={status.ml_classifier_available ? `ML 分类器 ${status.ml_model_version ?? ''}`.trim() : 'ML 分类器不可用'}
              size="small"
            />
          </div>
          {#if tally}
            <div class="gs-tally">
              <div class="tally-cell">
                <span class="tally-num mono">{tally.evaluations}</span>
                <span class="tally-lbl">评估</span>
              </div>
              <div class="tally-cell">
                <span class="tally-num mono ok">{tally.allowed}</span>
                <span class="tally-lbl">放行</span>
              </div>
              <div class="tally-cell">
                <span class="tally-num mono" class:bad={tally.denied > 0}>{tally.denied}</span>
                <span class="tally-lbl">否决</span>
              </div>
              <div class="tally-cell">
                <span class="tally-num mono" class:warn={tally.approvalRequired > 0}>{tally.approvalRequired}</span>
                <span class="tally-lbl">转人工</span>
              </div>
            </div>
          {/if}
        </div>
      {:else if statusError}
        <p class="gov-note bad">守卫状态读取失败：{statusError}</p>
      {:else}
        <LoadingState message="正在读取守卫状态…" />
      {/if}
    {:else}
      <GovUnsupported capabilityId="safety.guard.status.read" reason={statusReason} />
    {/if}

    <!-- dry-run 试算 -->
    {#if canEvaluate}
      <section class="dry-card">
        <header class="dry-head">
          <FlaskConical size={13} />
          <span class="dry-title">试算一道调用</span>
          <span class="dry-tag mono">DRY-RUN · 只评估不执行</span>
        </header>
        <div class="dry-form">
          <label class="dry-field">
            <span class="dry-lbl">能力 id</span>
            <input class="dry-input mono" type="text" placeholder="tool.repo" bind:value={dryCapability} />
          </label>
          <label class="dry-field">
            <span class="dry-lbl">参数 JSON</span>
            <textarea class="dry-input mono" rows="3" bind:value={dryArguments}></textarea>
          </label>
          <label class="dry-field">
            <span class="dry-lbl">声明范围（可选）</span>
            <input class="dry-input mono" type="text" placeholder="repo:write" bind:value={dryScope} />
          </label>
          <div class="dry-actions">
            <button class="btn-eval" disabled={dryBusy} onclick={() => void runDry()}>
              {dryBusy ? '评估中…' : '试算'}
            </button>
          </div>
        </div>
        {#if dryError}
          <p class="gov-note bad">{dryError}</p>
        {/if}
        {#if dryResult}
          <div class="dry-result">
            <div class="dr-row">
              <StatusBadge variant={dryResult.badge.variant} label={dryResult.badge.label} />
              <span class="dr-stage">{dryResult.stageLabel}</span>
              <span class="dr-risk mono">风险 {dryResult.riskPct}%</span>
            </div>
            {#if dryRaw && dryRaw.reasons.length}
              <ul class="dr-list">
                {#each dryRaw.reasons as reason}
                  <li>{reason}</li>
                {/each}
              </ul>
            {/if}
            {#if dryRaw && dryRaw.evidence.length}
              <details class="dr-evidence">
                <summary>证据 {dryRaw.evidence.length} 条</summary>
                {#each dryRaw.evidence as ev}
                  <code class="dr-ev-item mono">{ev}</code>
                {/each}
              </details>
            {/if}
          </div>
        {/if}
      </section>
    {:else}
      <GovUnsupported capabilityId="safety.guard.evaluate" reason={evaluateReason} hint="试算面不可用。" />
    {/if}

    <!-- 事件账 -->
    <div class="gov-toolbar">
      <span class="gov-count mono">最近评估 {events.length} 条</span>
      <button class="gov-quiet" onclick={() => void load()} disabled={loading}>
        <RotateCcw size={13} class={loading ? 'gov-spin' : ''} />
        刷新
      </button>
    </div>

    {#if !canEvents}
      <GovUnsupported capabilityId="safety.guard.events.read" reason={eventsReason} />
    {:else if loading && !events.length && !status}
      <LoadingState message="正在读取守卫事件…" />
    {:else if eventsError && !events.length}
      <ErrorState title="拉取守卫事件失败" message={eventsError} onRetry={() => void load()} />
    {:else if !events.length}
      <EmptyState
        icon="⛨"
        title="守卫尚未拦下任何评估记录"
        description="当他在回合里调用能力时，每一次守卫评估（放行 / 否决 / 转人工）都会记在这里——能力、阶段、风险分与理由。"
      />
    {:else}
      <div class="gov-list">
        {#each events as ev (guardEventRow(ev).id)}
          {@const row = guardEventRow(ev)}
          <article class="gov-card">
            <header class="gov-card-head">
              <span class="gov-card-title mono">{ev.capability_id}</span>
              <span class="ev-badges">
                <StatusBadge variant={row.badge.variant} label={row.badge.label} size="small" />
                <span class="ev-stage">{row.stageLabel}</span>
              </span>
            </header>
            <div class="ev-risk-row">
              <span class="ev-risk-bar">
                <span
                  class="ev-risk-fill"
                  class:high={row.riskPct >= 70}
                  class:mid={row.riskPct >= 40 && row.riskPct < 70}
                  style="width: {row.riskPct}%"
                ></span>
              </span>
              <span class="mono ev-risk-num">{row.riskPct}%</span>
            </div>
            <footer class="gov-card-foot">
              <span class="gov-meta mono" title={ev.session_id}>会话 {shortId(ev.session_id)}</span>
              <span class="gov-meta mono">R{ev.round}</span>
              <span class="gov-meta mono">{formatClockMs(ev.timestamp_ms)}</span>
              {#if ev.reasons.length || ev.evidence.length}
                <button class="ev-expand" onclick={() => toggleExpand(row.id)}>
                  {#if expandedIds[row.id]}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
                  理由与证据
                </button>
              {/if}
            </footer>
            {#if expandedIds[row.id]}
              <div class="ev-detail">
                {#each ev.reasons as reason}
                  <p class="ev-reason">{reason}</p>
                {/each}
                {#each ev.evidence as evd}
                  <code class="ev-ev-item mono">{evd}</code>
                {/each}
              </div>
            {/if}
          </article>
        {/each}
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
  .gov-note.bad {
    margin: 0;
    font-size: 11px;
    color: var(--ap-semantic-danger);
  }
  .gov-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .gov-count {
    font-size: 11px;
    color: var(--ap-bone-42);
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

  /* 状态条 */
  .guard-status {
    border: 1px solid var(--ap-line);
    border-left: 2px solid var(--gov-accent);
    border-radius: 8px;
    background: rgba(5, 10, 15, 0.4);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .gs-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .gs-tally {
    display: flex;
    gap: 22px;
    flex-wrap: wrap;
  }
  .tally-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .tally-num {
    font-size: 18px;
    color: var(--ap-bone);
  }
  .tally-num.ok {
    color: var(--ap-semantic-success);
  }
  .tally-num.bad {
    color: var(--ap-semantic-danger);
  }
  .tally-num.warn {
    color: var(--ap-semantic-warning);
  }
  .tally-lbl {
    font-size: 10.5px;
    color: var(--ap-bone-30);
  }

  /* dry-run 试算卡 */
  .dry-card {
    border: 1px dashed color-mix(in srgb, var(--gov-accent) 45%, transparent);
    border-radius: 8px;
    background: rgba(5, 10, 15, 0.3);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .dry-head {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--gov-accent);
  }
  .dry-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ap-bone);
  }
  .dry-tag {
    margin-left: auto;
    font-size: 9px;
    letter-spacing: 0.2em;
    color: var(--ap-bone-30);
  }
  .dry-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .dry-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .dry-lbl {
    font-size: 10.5px;
    color: var(--ap-bone-42);
  }
  .dry-input {
    width: 100%;
    box-sizing: border-box;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--ap-line);
    border-radius: 6px;
    color: var(--ap-bone);
    font-size: 11.5px;
    padding: 7px 10px;
    resize: vertical;
  }
  .dry-input:focus {
    outline: none;
    border-color: color-mix(in srgb, var(--gov-accent) 55%, transparent);
  }
  .dry-actions {
    display: flex;
    justify-content: flex-end;
  }
  .btn-eval {
    padding: 6px 16px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    background: color-mix(in srgb, var(--gov-accent) 80%, black);
    border: 1px solid var(--gov-accent);
    color: #fff;
  }
  .btn-eval:hover:not(:disabled) {
    filter: brightness(1.12);
  }
  .btn-eval:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .dry-result {
    border-top: 1px solid var(--ap-line);
    padding-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .dr-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .dr-stage {
    font-size: 11px;
    color: var(--ap-bone-42);
  }
  .dr-risk {
    margin-left: auto;
    font-size: 11px;
    color: var(--ap-bone);
  }
  .dr-list {
    margin: 0;
    padding-left: 18px;
    font-size: 11.5px;
    line-height: 1.7;
    color: var(--ap-bone-68);
  }
  .dr-evidence summary {
    font-size: 11px;
    color: var(--ap-bone-42);
    cursor: pointer;
  }
  .dr-ev-item {
    display: block;
    margin-top: 6px;
    font-size: 10.5px;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 4px;
    padding: 5px 8px;
    color: var(--ap-bone-68);
    word-break: break-all;
  }

  /* 事件列表 */
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
  .gov-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .gov-card-title {
    font-size: 12px;
    color: var(--ap-bone);
  }
  .ev-badges {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .ev-stage {
    font-size: 10.5px;
    color: var(--ap-bone-30);
  }
  .ev-risk-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .ev-risk-bar {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: rgba(232, 224, 204, 0.08);
    overflow: hidden;
  }
  .ev-risk-fill {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--ap-semantic-success);
  }
  .ev-risk-fill.mid {
    background: var(--ap-semantic-warning);
  }
  .ev-risk-fill.high {
    background: var(--ap-semantic-danger);
  }
  .ev-risk-num {
    font-size: 10.5px;
    color: var(--ap-bone-42);
    min-width: 34px;
    text-align: right;
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
  }
  .ev-expand {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 0;
    background: transparent;
    color: var(--ap-bone-42);
    font-size: 11px;
    cursor: pointer;
    padding: 0;
  }
  .ev-expand:hover {
    color: var(--ap-bone);
  }
  .ev-detail {
    border-top: 1px solid var(--ap-line);
    padding-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .ev-reason {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.6;
    color: var(--ap-bone-68);
  }
  .ev-ev-item {
    font-size: 10.5px;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 4px;
    padding: 5px 8px;
    color: var(--ap-bone-42);
    word-break: break-all;
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
