<script module lang="ts">
  export type GovernanceTabId = 'approvals' | 'grants' | 'guard' | 'audit';
</script>

<script lang="ts">
  // 治理卷宗（Governance Archive）—— 一视图四 tab：审批 / 授权 / 守卫 / 审计。
  //
  // 定位（00-PHILOSOPHY §4「照看与调性正交」+ gap-plan §4.3 P0）：
  // 审批已在对话内完成（待签文书卡），治理中心降级为**事后卷宗**——
  // 活儿在哪儿干，账都在同一处记。
  //
  // 调性：Deep-Ops 数据深舱调（01-DESIGN-SYSTEM §5.6③）——深底
  // deepOps.base/bottom，模块识别色 ≤6 且避开存在金：
  //   审批=蓝 #2392fb / 授权=绿 #6fae5f / 守卫=红 #d95f55 / 审计=工业黄 #e8c33a。
  // 金色纪律：本卷宗内不出现金色选中态/金色强调；唯一的金在审批空态的
  // 引导卡余烬点上——它指向对话内的金色待签文书（「他」相关，合法）。
  import {Inbox, KeyRound, ShieldCheck, ScrollText} from 'lucide-svelte';
  import {untrack} from 'svelte';
  import type {ApeirethConfig, CapabilityManifest} from '../types';
  import ApprovalsTab from './governance/ApprovalsTab.svelte';
  import GrantsTab from './governance/GrantsTab.svelte';
  import GuardTab from './governance/GuardTab.svelte';
  import AuditTab from './governance/AuditTab.svelte';

  let {
    config,
    capabilities = null,
    initialTab = 'approvals',
    onOpenChat,
  }: {
    config: ApeirethConfig;
    capabilities: CapabilityManifest | null;
    initialTab?: GovernanceTabId;
    /** 回到对话主页（审批空态引导卡用）。 */
    onOpenChat: () => void;
  } = $props();

  // 初值快照语义：initialTab 只在挂载时取一次（与 a614f805 基线修复同一约定）
  let tab = $state<GovernanceTabId>(untrack(() => initialTab));

  const TABS: Array<{id: GovernanceTabId; label: string; icon: typeof Inbox; accent: string}> = [
    {id: 'approvals', label: '审批', icon: Inbox, accent: 'var(--ap-register-deepops-blue)'},
    {id: 'grants', label: '授权', icon: KeyRound, accent: 'var(--ap-register-deepops-green)'},
    {id: 'guard', label: '守卫', icon: ShieldCheck, accent: 'var(--ap-register-deepops-red)'},
    {id: 'audit', label: '审计', icon: ScrollText, accent: 'var(--ap-register-deepops-yellow)'},
  ];
</script>

<section class="governance-view">
  <!-- tab 条：模块识别色点（Deep-Ops ≤6 色纪律），选中态用该模块色不用金 -->
  <div class="gov-tabs" role="tablist" aria-label="治理卷宗分面">
    {#each TABS as t (t.id)}
      {@const Icon = t.icon}
      <button
        class="gov-tab-btn"
        class:on={tab === t.id}
        style="--tab-accent: {t.accent}"
        role="tab"
        aria-selected={tab === t.id}
        onclick={() => (tab = t.id)}
      >
        <span class="tab-dot" aria-hidden="true"></span>
        <Icon size={13} />
        <span>{t.label}</span>
      </button>
    {/each}
  </div>

  <div class="gov-body">
    {#if tab === 'approvals'}
      <ApprovalsTab {config} {capabilities} {onOpenChat} onGoAudit={() => (tab = 'audit')} />
    {:else if tab === 'grants'}
      <GrantsTab {config} {capabilities} />
    {:else if tab === 'guard'}
      <GuardTab {config} {capabilities} />
    {:else}
      <AuditTab {config} {capabilities} />
    {/if}
  </div>
</section>

<style>
  /* Deep-Ops 页面层承托：深底渐变（base → bottom），从轨道进入、场景透见 */
  .governance-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: linear-gradient(
      180deg,
      var(--ap-register-deepops-base) 0%,
      var(--ap-register-deepops-bottom) 100%
    );
  }
  .gov-tabs {
    flex: none;
    display: flex;
    gap: 4px;
    padding: 12px 20px 0;
    border-bottom: 1px solid var(--ap-line);
  }
  .gov-tab-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 14px 10px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--ap-bone-42);
    font-size: 12.5px;
    cursor: pointer;
    transition: color 0.18s ease, border-color 0.18s ease;
  }
  .tab-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--tab-accent);
    opacity: 0.45;
    transition: opacity 0.18s ease;
  }
  .gov-tab-btn:hover {
    color: var(--ap-bone-68);
  }
  .gov-tab-btn:hover .tab-dot {
    opacity: 0.8;
  }
  .gov-tab-btn.on {
    color: var(--ap-bone);
    border-bottom-color: var(--tab-accent);
  }
  .gov-tab-btn.on .tab-dot {
    opacity: 1;
    box-shadow: 0 0 6px color-mix(in srgb, var(--tab-accent) 60%, transparent);
  }
  .gov-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 14px 20px 20px;
    overflow: hidden;
  }
</style>
