<script lang="ts">
  // 治理卷宗 ② 授权管理 —— grants 列表 + revoke。
  //
  // 契约 §7：GET /v1/panel/grants（granted_at 缺省——canonical policy 不记
  // 时间戳，UI 如实说「未记录」）；POST /v1/panel/grants/revoke 是
  // **会话级热撤销**：即刻生效、作用于当前进程，进程重启后恢复默认策略
  // ——这条后端语义必须写进确认文案，不许暗示「永久撤销」。
  //
  // 不可逆操作纪律（gap-plan §4.4 组件 13）：revoke 用卡内红色边框确认块，
  // 不弹二次窗。
  import {onMount} from 'svelte';
  import {RotateCcw, ShieldAlert} from 'lucide-svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import ErrorState from '../../components/ErrorState.svelte';
  import LoadingState from '../../components/LoadingState.svelte';
  import StatusBadge from '../../components/StatusBadge.svelte';
  import GovUnsupported from './GovUnsupported.svelte';
  import type {ApeirethConfig, CapabilityManifest} from '../../types';
  import {
    capabilityAvailable,
    capabilityUnavailableReason,
    fetchGrants,
    friendlyErrorMessage,
    revokeGrant,
    type GrantView,
  } from '../../runtime';
  import {formatDateTimeMs} from '../../governance/format';

  let {
    config,
    capabilities = null,
  }: {
    config: ApeirethConfig;
    capabilities: CapabilityManifest | null;
  } = $props();

  const canRead = $derived(capabilityAvailable(capabilities, 'permissions.grants.read'));
  const canRevoke = $derived(capabilityAvailable(capabilities, 'permissions.revoke'));
  const readReason = $derived(capabilityUnavailableReason(capabilities, 'permissions.grants.read'));
  const revokeReason = $derived(capabilityUnavailableReason(capabilities, 'permissions.revoke'));

  let grants = $state<GrantView[]>([]);
  let loading = $state(false);
  let error = $state('');

  /** 卡内确认态：正在确认/正在撤销的 grant id（红边框卡内确认，不弹窗）。 */
  let confirmingId = $state<string | null>(null);
  let revokingId = $state<string | null>(null);
  let revokeError = $state('');

  async function load(): Promise<void> {
    if (!canRead) return;
    loading = true;
    error = '';
    try {
      grants = await fetchGrants(config);
    } catch (e) {
      error = friendlyErrorMessage(e, '/v1/panel/grants');
    } finally {
      loading = false;
    }
  }

  async function executeRevoke(grant: GrantView): Promise<void> {
    if (!canRevoke || revokingId) return;
    revokingId = grant.id;
    revokeError = '';
    const r = await revokeGrant(config, grant.id);
    revokingId = null;
    if (r.ok) {
      confirmingId = null;
      grants = grants.filter((g) => g.id !== grant.id);
    } else {
      revokeError = r.error || '撤销失败';
    }
  }

  onMount(() => {
    void load();
  });
</script>

<div class="gov-tab" style="--gov-accent: var(--ap-register-deepops-green)">
  <p class="gov-position">
    授权是「他获得的能力」的账。撤销即刻生效、作用于当前进程；进程重启后按默认策略恢复。
  </p>

  {#if !canRead}
    <GovUnsupported
      capabilityId="permissions.grants.read"
      reason={readReason}
      hint="授权账本需要 canonical gateway 的 grants 内省面。"
    />
  {:else}
    <div class="gov-toolbar">
      <span class="gov-count mono">{grants.length} 项授权在账</span>
      <button class="gov-quiet" onclick={() => void load()} disabled={loading}>
        <RotateCcw size={13} class={loading ? 'gov-spin' : ''} />
        刷新
      </button>
    </div>

    {#if revokeError}
      <p class="gov-note bad">{revokeError}</p>
    {/if}

    {#if loading && !grants.length}
      <LoadingState message="正在读取授权账本…" />
    {:else if error && !grants.length}
      <ErrorState title="拉取授权失败" message={error} onRetry={() => void load()} />
    {:else if !grants.length}
      <EmptyState
        icon="⚿"
        title="账上无授权"
        description="当你在审批文书上批准一项能力、或会话策略放宽时，被授权的能力会记在这里——哪一项、属于哪类、何时入账（若后端记录）。"
      />
    {:else}
      <div class="gov-list">
        {#each grants as grant (grant.id)}
          {@const confirming = confirmingId === grant.id}
          {@const busy = revokingId === grant.id}
          <article class="gov-card" class:confirming>
            <header class="gov-card-head">
              <span class="gov-card-title">{grant.name}</span>
              <StatusBadge variant="green" label="生效中" size="small" />
            </header>
            <footer class="gov-card-foot">
              <span class="gov-meta mono">{grant.id}</span>
              <span class="gov-meta">
                {#if grant.activated_at_ms > 0}
                  入账 {formatDateTimeMs(grant.activated_at_ms)}
                {:else}
                  授予时刻未记录（canonical policy 不记时间戳）
                {/if}
              </span>
              {#if !confirming}
                <span class="gov-actions">
                  <button
                    class="btn-revoke"
                    disabled={!canRevoke || revokingId !== null}
                    title={!canRevoke
                      ? `当前运行时不支持 permissions.revoke${revokeReason ? `（${revokeReason}）` : ''}`
                      : '撤销此项授权'}
                    onclick={() => {
                      confirmingId = grant.id;
                      revokeError = '';
                    }}
                  >
                    撤销
                  </button>
                </span>
              {/if}
            </footer>

            {#if confirming}
              <!-- 卡内确认（§4.4 组件 13：不可逆操作红边框卡内确认，不弹二次窗） -->
              <div class="revoke-confirm" role="alert">
                <p class="rc-title">
                  <ShieldAlert size={13} />
                  确认撤销 {grant.name}？
                </p>
                <p class="rc-body">
                  撤销即刻生效，作用于当前进程的同一 policy 实例；进程重启后按默认策略恢复。
                  撤销后他再调用此能力会重新停下等审批。
                </p>
                <div class="rc-actions">
                  <button class="btn-cancel" disabled={busy} onclick={() => (confirmingId = null)}>
                    取消
                  </button>
                  <button class="btn-confirm-danger" disabled={busy} onclick={() => void executeRevoke(grant)}>
                    {busy ? '撤销中…' : '确认撤销'}
                  </button>
                </div>
              </div>
            {/if}
          </article>
        {/each}
      </div>
      {#if !canRevoke}
        <GovUnsupported
          capabilityId="permissions.revoke"
          reason={revokeReason}
          hint="授权账可读，撤销动作不可用。"
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
  }
  .gov-count {
    font-size: 11px;
    color: var(--ap-bone-42);
  }
  .mono {
    font-family: var(--ap-font-mono);
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
  .gov-note.bad {
    margin: 0;
    font-size: 11px;
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
    transition: border-color 0.2s ease;
  }
  /* 卡内确认态：整卡红边（危险语义），确认块不再另起弹窗 */
  .gov-card.confirming {
    border-color: rgba(192, 88, 78, 0.65);
    border-left-color: var(--ap-semantic-danger);
  }
  .gov-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .gov-card-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--ap-bone);
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
  .gov-actions {
    margin-left: auto;
  }
  .btn-revoke {
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 11.5px;
    cursor: pointer;
    background: transparent;
    border: 1px solid rgba(192, 88, 78, 0.4);
    color: var(--ap-semantic-danger);
  }
  .btn-revoke:hover:not(:disabled) {
    background: rgba(192, 88, 78, 0.12);
  }
  .btn-revoke:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .revoke-confirm {
    border: 1px solid rgba(192, 88, 78, 0.5);
    border-radius: 8px;
    background: rgba(192, 88, 78, 0.08);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .rc-title {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ap-semantic-danger);
  }
  .rc-body {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.7;
    color: var(--ap-bone-68);
  }
  .rc-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn-cancel {
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    background: transparent;
    border: 1px solid var(--ap-line);
    color: var(--ap-bone-42);
  }
  .btn-cancel:hover:not(:disabled) {
    color: var(--ap-bone);
  }
  .btn-confirm-danger {
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    background: var(--ap-semantic-danger);
    border: 1px solid var(--ap-semantic-danger);
    color: #fff;
  }
  .btn-confirm-danger:hover:not(:disabled) {
    filter: brightness(1.1);
  }
  .btn-confirm-danger:disabled,
  .btn-cancel:disabled {
    opacity: 0.5;
    cursor: default;
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
