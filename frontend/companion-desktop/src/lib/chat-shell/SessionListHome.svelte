<script lang="ts">
  // T0 聊天壳骨骼 — 会话列表主页（「谁找我了」，00-PHILOSOPHY §3.1）
  //
  // 打开应用的第一屏。数据源：本地 conversations ⋈ 后端账本
  // GET /v1/panel/sessions（gateway 契约 §4，canonical 已就绪），
  // 归并逻辑在 ./session-list.ts（纯函数，Node 可测）。
  //
  // 0 装纪律：
  // - 后端账本未声明 sessions.read / 拉取失败 → 顶部 mono 小字诚实标注，
  //   列表退化为本机会话，不伪造后端行。
  // - backend-only 会话（本机无消息副本）预览位写「内容在他的账本里」，
  //   不伪造消息正文。
  // - 无未读 badge（后端无此概念）；唯一的状态标记是「待签」（金，因为
  //   他停下了，§7 金色纪律），由 SSE approval_required 真实信号驱动。
  import {Plus} from 'lucide-svelte';
  import type {ApeirethConfig, CapabilityManifest, Conversation} from '../types';
  import {presenceStore, deriveEmberBreath} from '../presence';
  import {
    capabilityAvailable,
    capabilitySupported,
    fetchBackendSessions,
    friendlyErrorMessage,
  } from '../runtime';
  import {
    formatSessionTime,
    mergeSessionLedger,
    type BackendLedgerSession,
    type HomeSessionItem,
  } from './session-list';

  let {
    conversations,
    config,
    capabilities = null,
    pendingApprovalSessions,
    himName,
    himStatus,
    himAttention = false,
    reloadKey = 0,
    activeId = null,
    onOpen,
    onOpenHim,
    onNew,
  }: {
    conversations: Conversation[];
    config: ApeirethConfig;
    capabilities: CapabilityManifest | null;
    /** 有待签文书的会话 id 集（SSE 真实信号推导，来自 App）。 */
    pendingApprovalSessions: ReadonlySet<string>;
    /** 置顶行：他的名字（active persona 显示名）。 */
    himName: string;
    /** 置顶行：真实状态词（正在输出/思考中/在/离线…，由 App 从真实状态推导）。 */
    himStatus: string;
    /** 他停下了（有待签文书）→ 状态词用金（§7：金 = 他停下了）。 */
    himAttention?: boolean;
    /** App 节拍/SSE 驱动的重拉信号（变化即重拉账本）。 */
    reloadKey?: number;
    /** 三栏主从（2026-09-22 主人拍板）：当前选中会话 id，行内金线 active 态。 */
    activeId?: string | null;
    onOpen: (item: HomeSessionItem) => void;
    onOpenHim: () => void;
    onNew: () => void;
  } = $props();

  let backend = $state<BackendLedgerSession[] | null>(null);
  let ledgerNote = $state<string | null>(null);
  let ledgerLoading = $state(false);

  const items = $derived(
    mergeSessionLedger({local: conversations, backend, pendingApprovalSessions}),
  );

  // T0 余烬点呼吸参数：契约 §8a breath 通道（每帧更新，含 60s 心跳——
  // 显影分级：心跳只动余烬点，不动光环）。无帧/SIM 时回落契约基线 4s/0.65。
  const emberBreath = $derived(deriveEmberBreath($presenceStore.breath));

  async function loadLedger(): Promise<void> {
    // manifest 尚未到达 ≠ 不支持：先给中性探测态，等能力清单到了再判定（诚实时序）。
    if (capabilities === null) {
      backend = null;
      ledgerNote = null;
      return;
    }
    // capability gate（0 装）：不支持就显式标注，不请求不存在的面。
    if (!capabilitySupported(capabilities, 'sessions.read')) {
      backend = null;
      ledgerNote = '后端会话账本未开通（当前运行时未声明 sessions.read）——这里只列出本机记得的对话。';
      return;
    }
    if (!capabilityAvailable(capabilities, 'sessions.read')) {
      backend = null;
      ledgerNote = '后端会话账本当前不可用（sessions.read unavailable）——这里只列出本机记得的对话。';
      return;
    }
    ledgerLoading = true;
    try {
      backend = await fetchBackendSessions(config);
      ledgerNote = null;
    } catch (caught) {
      backend = null;
      ledgerNote = `后端会话账本暂时不可达（${friendlyErrorMessage(caught, '/v1/panel/sessions')}）——这里只列出本机记得的对话。`;
    } finally {
      ledgerLoading = false;
    }
  }

  // 挂载即拉取；baseUrl 变化（侧车重端口）与 reloadKey 节拍变化时重拉。
  $effect(() => {
    void config.baseUrl;
    void reloadKey;
    void capabilities;
    void loadLedger();
  });
</script>

<section class="home-list" aria-label="往来">
  <header class="home-head">
    <p class="eyebrow">往来</p>
    <h1 class="home-title">谁找我了</h1>
    {#if ledgerNote}
      <p class="ledger-note" role="status">{ledgerNote}</p>
    {:else if capabilities === null || (ledgerLoading && !backend)}
      <p class="ledger-note" role="status">正在对齐他的账本…</p>
    {/if}
  </header>

  <!-- 置顶行：他。金色余烬 = 他在（§7 金色纪律；T0 余烬呼吸接 §8a breath，§8 性能表）。
       状态词全部来自前端真实状态（流式/审批/健康），无假数据。 -->
  <button class="him-row" onclick={onOpenHim} aria-label={`与 ${himName} 继续`}>
    <span
      class="ember-dot"
      aria-hidden="true"
      style:--ember-period={`${emberBreath.periodSecs}s`}
      style:--ember-amp={emberBreath.amplitude}
    ></span>
    <span class="him-name">{himName}</span>
    <span class="him-status" class:gold={himAttention}>{himStatus}</span>
  </button>

  {#if items.length}
    <ul class="session-list">
      {#each items as item (item.id)}
        <li>
          <button class="session-row" class:active={item.id === activeId} onclick={() => onOpen(item)}>
            <span class="session-main">
              <span class="session-title">
                {item.title}
                {#if item.pendingApproval}
                  <span class="pending-mark" title="他停下了，等你签字">待签</span>
                {/if}
              </span>
              {#if item.preview}
                <span class="session-preview">{item.preview}</span>
              {:else if item.origin === 'backend'}
                <span class="session-preview ledger">内容在他的账本里 · {item.messageCount} 条</span>
              {:else}
                <span class="session-preview faint">尚未开始交谈</span>
              {/if}
            </span>
            <span class="session-side">
              <span class="session-time">{formatSessionTime(item.lastActiveAt)}</span>
              {#if item.origin !== 'local' && item.messageCount > 0}
                <span class="session-count">{item.messageCount} 条</span>
              {/if}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {:else if !ledgerLoading && capabilities !== null}
    <!-- 空态即契约（00-PHILOSOPHY 原则 5）：不写"暂无数据"。 -->
    <div class="empty-contract">
      <p class="empty-line">这里还没有任何会话。</p>
      <p class="empty-promise">当你们开始第一段对话后，这里会出现它；他停下等你签字时，对应的一行会亮起金色「待签」。</p>
      <button class="quiet-btn" onclick={onNew}>
        <Plus size={13} />
        开始第一段对话
      </button>
    </div>
  {/if}
</section>

<style>
  .home-list {
    /* 三栏主从（2026-09-22 主人拍板）：本组件常驻 ~300px 列表栏，
       栏体面板承托由外层 .session-col 承担，此处只留内距。 */
    padding: 20px 14px 16px;
    pointer-events: auto;
  }
  .home-head {
    margin-bottom: 16px;
  }
  .eyebrow {
    margin: 0 0 6px;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.5em;
    color: var(--ap-bone-30);
  }
  .home-title {
    margin: 0;
    font-family: var(--ap-font-voice);
    font-weight: 400;
    font-size: clamp(19px, 1.8vw, 25px);
    letter-spacing: 0.14em;
    color: var(--ap-bone);
  }
  .ledger-note {
    margin: 10px 0 0;
    display: inline-block;
    padding: 6px 10px;
    border: 1px solid var(--ap-line);
    border-radius: 6px;
    background: var(--ap-shell-chip);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    line-height: 1.7;
    color: var(--ap-bone-42);
  }

  /* ---------- 置顶行：他 ---------- */
  .him-row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 13px 16px;
    margin-bottom: 14px;
    border: 1px solid var(--ap-line);
    border-radius: 10px;
    background: var(--ap-panel);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    cursor: pointer;
    text-align: left;
    transition: border-color 0.25s ease, box-shadow 0.25s ease;
  }
  .him-row:hover {
    border-color: rgba(255, 210, 122, 0.45);
    box-shadow: 0 0 22px -8px rgba(255, 210, 122, 0.35);
  }
  .ember-dot {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--ap-gold);
    box-shadow: 0 0 10px rgba(255, 210, 122, 0.6);
    /* T0 余烬呼吸：周期/振幅由 §8a breath 通道经 CSS 变量驱动（heuristic_v0 保守
       增益在 presence.ts deriveEmberBreath 内钳制）；无帧回落契约基线 4s/0.65
       （00-PHILOSOPHY §8 性能表 / §8a breath.period_secs=4.0）。 */
    animation: ap-ember-breathe var(--ember-period, 4s) ease-in-out infinite;
  }
  @keyframes ap-ember-breathe {
    0%,
    100% {
      /* amp 0.65 → opacity 0.35 / scale 1.13（≈旧静态基线 0.35/1.12） */
      opacity: calc(1 - var(--ember-amp, 0.65));
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(calc(1 + 0.2 * var(--ember-amp, 0.65)));
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .ember-dot {
      animation: none;
      opacity: 0.7;
    }
  }
  .him-name {
    font-family: var(--ap-font-voice);
    font-size: 15px;
    letter-spacing: 0.18em;
    color: var(--ap-bone);
  }
  .him-status {
    margin-left: auto;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.2em;
    color: var(--ap-bone-42);
  }
  .him-status.gold {
    color: var(--ap-gold);
  }

  /* ---------- 会话行（用户侧元素：无金，金只给待签标与 hover 细线） ----------
     页面层承托（01-DESIGN-SYSTEM §5.1）：逐行卡片 = ui.panel #0b0d12@82% +
     blur 20px，场景在行间透见而不是透视穿字；行照亮档下同样可读。 */
  .session-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .session-row {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    width: 100%;
    padding: 12px 14px;
    border: 1px solid var(--ap-line);
    border-radius: 10px;
    background: var(--ap-panel);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    cursor: pointer;
    text-align: left;
    transition: border-color 0.25s ease, box-shadow 0.25s ease;
  }
  .session-row:hover {
    border-color: rgba(255, 210, 122, 0.45);
    box-shadow: 0 0 22px -8px rgba(255, 210, 122, 0.3);
  }
  /* 三栏主从：选中行金线 active 态（已验收语言——hover 金细线的常驻版） */
  .session-row.active {
    border-color: rgba(255, 210, 122, 0.55);
    box-shadow:
      inset 2px 0 0 var(--ap-gold),
      0 0 18px -8px rgba(255, 210, 122, 0.28);
  }
  .session-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .session-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    letter-spacing: 0.04em;
    color: var(--ap-bone);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pending-mark {
    flex: none;
    font-family: var(--ap-font-mono);
    font-size: 9px;
    letter-spacing: 0.24em;
    padding: 2px 6px 2px 8px;
    border: 1px solid rgba(255, 210, 122, 0.5);
    border-radius: 2px;
    color: var(--ap-gold);
  }
  .session-preview {
    font-size: 11.5px;
    line-height: 1.6;
    color: var(--ap-bone-42);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .session-preview.ledger {
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.06em;
    color: var(--ap-bone-30);
  }
  .session-preview.faint {
    color: var(--ap-bone-30);
  }
  .session-side {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 3px;
  }
  .session-time {
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--ap-bone-42);
  }
  .session-count {
    font-family: var(--ap-font-mono);
    font-size: 9.5px;
    color: var(--ap-bone-30);
  }

  /* ---------- 空态契约（同在页面层：面板承托，不印在场景上） ---------- */
  .empty-contract {
    padding: 22px 24px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    border: 1px solid var(--ap-line);
    border-radius: 12px;
    background: var(--ap-panel);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
  }
  .empty-line {
    margin: 0;
    font-size: 13px;
    color: var(--ap-bone-68);
    letter-spacing: 0.04em;
  }
  .empty-promise {
    margin: 0 0 10px;
    font-size: 11.5px;
    line-height: 1.8;
    color: var(--ap-bone-42);
    max-width: 46ch;
  }
  .empty-contract .quiet-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
