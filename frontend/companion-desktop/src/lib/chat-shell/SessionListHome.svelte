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
  //
  // 2026-10-11 主人拍板批（Kimi Desktop 范式）：
  // - 分组：项目区（按会话级工作目录）+ 联系人区（按创建时的伙伴人设），
  //   组可下拉收起，收起态持久化 localStorage。
  // - 行操作：hover「···」菜单 = 置顶 / 重命名 / 归档（已归档组内为还原）/ 删除
  //   （两步确认）。backend-only 行无本地副本，不出菜单（归档/删除只动本地账）。
  import {ChevronDown, FolderOpen, MoreHorizontal, Pin, Plus, UserRound} from 'lucide-svelte';
  import type {ApeirethConfig, CapabilityManifest, Conversation} from '../types';
  import {presenceStore, deriveEmberBreath} from '../presence';
  import {
    capabilityAvailable,
    capabilitySupported,
    fetchBackendSessions,
    friendlyErrorMessage,
  } from '../runtime';
  import {
    archivedHomeItems,
    formatSessionTime,
    groupHomeSessions,
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
    defaultWorkspace = null,
    onOpen,
    onOpenHim,
    onNew,
    onRename,
    onTogglePin,
    onToggleArchive,
    onDelete,
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
    /** 工作区默认回退：会话未戳工作区（旧数据）时归入的当前项目组（App 传当前工作目录）。 */
    defaultWorkspace?: string | null;
    onOpen: (item: HomeSessionItem) => void;
    onOpenHim: () => void;
    onNew: () => void;
    onRename: (id: string, title: string) => void;
    onTogglePin: (id: string) => void;
    onToggleArchive: (id: string) => void;
    onDelete: (id: string) => void;
  } = $props();

  let backend = $state<BackendLedgerSession[] | null>(null);
  let ledgerNote = $state<string | null>(null);
  let ledgerLoading = $state(false);

  const items = $derived(
    mergeSessionLedger({local: conversations, backend, pendingApprovalSessions, defaultWorkspace}),
  );
  const sections = $derived(groupHomeSessions(items));
  const archived = $derived(archivedHomeItems(conversations, pendingApprovalSessions, defaultWorkspace));

  // ---- 分组收起态（持久化；键 = 区/组前缀 + 组键） ----
  const COLLAPSE_KEY = 'apeireth-home-collapsed';
  function loadCollapsed(): Record<string, boolean> {
    try {
      const raw = localStorage.getItem(COLLAPSE_KEY);
      const parsed = raw ? JSON.parse(raw) : {};
      return typeof parsed === 'object' && parsed !== null ? parsed : {};
    } catch {
      return {};
    }
  }
  let collapsed = $state<Record<string, boolean>>(loadCollapsed());
  function toggleCollapse(key: string): void {
    collapsed = {...collapsed, [key]: !collapsed[key]};
    try {
      localStorage.setItem(COLLAPSE_KEY, JSON.stringify(collapsed));
    } catch {
      /* 隐私模式等写入失败：收起态本会话内仍生效 */
    }
  }

  // ---- 行操作菜单（单开；rename / 删除两步确认） ----
  let menuFor = $state<string | null>(null);
  let renameFor = $state<string | null>(null);
  let renameDraft = $state('');
  let confirmDeleteFor = $state<string | null>(null);

  function openMenu(id: string, e: Event): void {
    e.stopPropagation();
    e.preventDefault();
    confirmDeleteFor = null;
    menuFor = menuFor === id ? null : id;
  }
  function startRename(item: HomeSessionItem, e: Event): void {
    e.stopPropagation();
    renameFor = item.id;
    renameDraft = item.title;
    menuFor = null;
  }
  function commitRename(): void {
    const title = renameDraft.trim();
    if (renameFor && title) onRename(renameFor, title);
    renameFor = null;
  }
  function askDelete(id: string, e: Event): void {
    e.stopPropagation();
    if (confirmDeleteFor === id) {
      menuFor = null;
      confirmDeleteFor = null;
      onDelete(id);
    } else {
      confirmDeleteFor = id;
    }
  }

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

  const isCollapsed = (key: string): boolean => collapsed[key] === true;
</script>

{#if menuFor}
  <!-- 菜单外点收幕 -->
  <button class="menu-scrim" onclick={() => (menuFor = null)} aria-label="收起菜单" tabindex="-1"></button>
{/if}

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

  <!-- ======== 项目区（按工作目录收纳，Kimi 范式） ======== -->
  {#if sections.projects.length}
    <div class="sec">
      <button class="sec-head" onclick={() => toggleCollapse('sec:projects')} aria-expanded={!isCollapsed('sec:projects')}>
        项目
        <span class="sec-count">{items.length}</span>
        <ChevronDown size={12} class={isCollapsed('sec:projects') ? 'caret closed' : 'caret'} />
      </button>
      {#if !isCollapsed('sec:projects')}
        {#each sections.projects as group (group.key)}
          <div class="grp">
            <button class="grp-head" onclick={() => toggleCollapse(`p:${group.key}`)} aria-expanded={!isCollapsed(`p:${group.key}`)}>
              <FolderOpen size={12} class="grp-ico" />
              <span class="grp-name">{group.label}</span>
              <span class="grp-count">{group.items.length}</span>
              <ChevronDown size={11} class={isCollapsed(`p:${group.key}`) ? 'caret closed' : 'caret'} />
            </button>
            {#if !isCollapsed(`p:${group.key}`)}
              <ul class="session-list">
                {#each group.items as item (item.id)}
                  {@render sessionRow(item, false, 'p')}
                {/each}
              </ul>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  <!-- ======== 联系人区（按伙伴人设备纳） ======== -->
  {#if sections.contacts.length}
    <div class="sec">
      <button class="sec-head" onclick={() => toggleCollapse('sec:contacts')} aria-expanded={!isCollapsed('sec:contacts')}>
        联系人
        <span class="sec-count">{items.length}</span>
        <ChevronDown size={12} class={isCollapsed('sec:contacts') ? 'caret closed' : 'caret'} />
      </button>
      {#if !isCollapsed('sec:contacts')}
        {#each sections.contacts as group (group.key)}
          <div class="grp">
            <button class="grp-head" onclick={() => toggleCollapse(`c:${group.key}`)} aria-expanded={!isCollapsed(`c:${group.key}`)}>
              <UserRound size={12} class="grp-ico" />
              <span class="grp-name">{group.label}</span>
              <span class="grp-count">{group.items.length}</span>
              <ChevronDown size={11} class={isCollapsed(`c:${group.key}`) ? 'caret closed' : 'caret'} />
            </button>
            {#if !isCollapsed(`c:${group.key}`)}
              <ul class="session-list">
                {#each group.items as item (item.id)}
                  {@render sessionRow(item, false, 'p')}
                {/each}
              </ul>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  <!-- ======== 已归档（管理动作，收在底部折叠组） ======== -->
  {#if archived.length}
    <div class="sec">
      <button class="sec-head" onclick={() => toggleCollapse('sec:archived')} aria-expanded={!isCollapsed('sec:archived')}>
        已归档
        <span class="sec-count">{archived.length}</span>
        <ChevronDown size={12} class={isCollapsed('sec:archived') ? 'caret closed' : 'caret'} />
      </button>
      {#if !isCollapsed('sec:archived')}
        <ul class="session-list archived">
          {#each archived as item (item.id)}
            {@render sessionRow(item, true, 'a')}
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  {#if !items.length && !archived.length && !ledgerLoading}
    <!-- 空态即契约（00-PHILOSOPHY 原则 5）：不写"暂无数据"。
         走查修复：原门槛要求 capabilities !== null——网关离线时清单恒 null，
         全新用户看到整栏空白、无任何入口；CTA 本身本地可用（发送时离线
         会如实弹错误横幅），不应被后端状态吞掉。 -->
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

{#snippet sessionRow(item: HomeSessionItem, inArchived: boolean, sec: 'p' | 'c' | 'a')}
  {@const menuKey = `${sec}:${item.id}`}
  <li class="row-wrap">
    {#if renameFor === item.id}
      <!-- 行内重命名 -->
      <div class="session-row renaming">
        <input
          class="rename-input"
          bind:value={renameDraft}
          onkeydown={(e) => {
            if (e.key === 'Enter') commitRename();
            if (e.key === 'Escape') renameFor = null;
          }}
          onblur={commitRename}
          aria-label="重命名会话"
        />
      </div>
    {:else}
      <button class="session-row" class:active={item.id === activeId} onclick={() => onOpen(item)}>
        <span class="session-main">
          <span class="session-title">
            {#if item.pinned}
              <Pin size={10} class="pin-ico" aria-label="已置顶" />
            {/if}
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
    {/if}
    {#if item.origin !== 'backend'}
      <button
        class="row-more"
        class:show={menuFor === menuKey}
        onclick={(e) => openMenu(menuKey, e)}
        aria-label="会话操作"
        aria-expanded={menuFor === menuKey}
      >···</button>
      {#if menuFor === menuKey}
        <div class="row-menu" role="menu">
          <button role="menuitem" onclick={(e) => { e.stopPropagation(); onTogglePin(item.id); menuFor = null; }}>
            {item.pinned ? '取消置顶' : '置顶'}
          </button>
          <button role="menuitem" onclick={(e) => startRename(item, e)}>重命名</button>
          <button role="menuitem" onclick={(e) => { e.stopPropagation(); onToggleArchive(item.id); menuFor = null; }}>
            {inArchived ? '还原' : '归档'}
          </button>
          <button
            role="menuitem"
            class="danger"
            class:confirming={confirmDeleteFor === item.id}
            onclick={(e) => askDelete(item.id, e)}
          >
            {confirmDeleteFor === item.id ? '确认删除？' : '删除'}
          </button>
        </div>
      {/if}
    {/if}
  </li>
{/snippet}

<style>
  .home-list {
    /* 三栏主从（2026-09-22 主人拍板）：本组件常驻 ~300px 列表栏，
       栏体面板承托由外层 .session-col 承担，此处只留内距。 */
    padding: 20px 14px 16px;
    pointer-events: auto;
  }
  .home-head {
    /* 主人反馈批③：眉题区常驻栏顶——sticky 实底带（同 .chat-head 语言），
       会话组往下滚动时「谁找我了」留在框里，栏顶与窗框融为一体。
       负边距抵消 .home-list 内距，实底触到栏体四边（栏体 overflow 裁剪，
       不出血到右侧聊天区，那里保留壁纸显影）。 */
    position: sticky;
    top: 0;
    z-index: 5;
    margin: -20px -14px 16px;
    padding: 20px 14px 12px;
    background: var(--ap-panel-solid);
  }
  .eyebrow {
    margin: 0 0 6px;
    font-family: var(--ap-font-mono);
    /* 走查校准：同 drawer-head .eyebrow——0.5em 字距把两字眉题拉成噪点。 */
    font-size: 10px;
    letter-spacing: 0.26em;
    color: var(--ap-bone-42);
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

  /* ---------- 分区（项目/联系人/已归档）与组（Kimi 范式） ---------- */
  .sec {
    margin-top: 14px;
  }
  .sec-head {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 4px 2px 8px;
    border: 0;
    background: transparent;
    cursor: pointer;
    font-family: var(--ap-font-mono);
    font-size: 10.5px;
    letter-spacing: 0.24em;
    color: var(--ap-bone-42);
    text-align: left;
  }
  .sec-head:hover {
    color: var(--ap-bone-68);
  }
  .sec-count {
    margin-left: auto;
    letter-spacing: 0.1em;
    color: var(--ap-bone-30);
  }
  .grp {
    margin-bottom: 6px;
  }
  .grp-head {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    padding: 5px 6px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    cursor: pointer;
    font-size: 12px;
    letter-spacing: 0.05em;
    color: var(--ap-bone-68);
    text-align: left;
  }
  .grp-head:hover {
    background: var(--ap-shell-chip);
    color: var(--ap-bone);
  }
  :global(.grp-ico) {
    flex: none;
    color: var(--ap-bone-42);
  }
  .grp-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .grp-count {
    flex: none;
    font-family: var(--ap-font-mono);
    font-size: 9.5px;
    color: var(--ap-bone-30);
  }
  :global(.caret) {
    flex: none;
    transition: transform 0.2s ease;
  }
  :global(.caret.closed) {
    transform: rotate(-90deg);
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
  .session-list.archived {
    opacity: 0.72;
  }
  .row-wrap {
    position: relative;
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
  .session-row.renaming {
    cursor: default;
  }
  .rename-input {
    width: 100%;
    padding: 6px 8px;
    border: 1px solid rgba(255, 210, 122, 0.5);
    border-radius: 7px;
    background: var(--ap-panel-solid);
    color: var(--ap-bone);
    font-size: 13px;
    letter-spacing: 0.04em;
    outline: none;
  }
  :global(.pin-ico) {
    flex: none;
    color: var(--ap-gold);
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
    transition: opacity 0.15s ease;
  }
  /* hover 时右侧时间/条数让位给「···」操作钮（Kimi 行内菜单范式） */
  .row-wrap:hover .session-side {
    opacity: 0;
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

  /* ---------- 行「···」菜单 ---------- */
  .row-more {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 3;
    padding: 2px 7px;
    border: 1px solid var(--ap-line);
    border-radius: 6px;
    background: var(--ap-panel-solid);
    color: var(--ap-bone-42);
    font-size: 11px;
    line-height: 1.4;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s ease, color 0.15s ease;
  }
  .row-wrap:hover .row-more,
  .row-more.show,
  .row-more:focus-visible {
    opacity: 1;
  }
  .row-more:hover {
    color: var(--ap-bone);
    border-color: rgba(255, 210, 122, 0.45);
  }
  .row-menu {
    position: absolute;
    top: 30px;
    right: 8px;
    z-index: 30;
    min-width: 128px;
    padding: 5px;
    border: 1px solid var(--ap-line);
    border-radius: 9px;
    background: var(--ap-panel-solid);
    box-shadow: 0 10px 30px -10px rgba(0, 0, 0, 0.7);
    display: flex;
    flex-direction: column;
  }
  .row-menu button {
    padding: 7px 10px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--ap-bone-68);
    font-size: 12px;
    letter-spacing: 0.04em;
    text-align: left;
    cursor: pointer;
  }
  .row-menu button:hover {
    background: var(--ap-shell-chip);
    color: var(--ap-bone);
  }
  .row-menu button.danger {
    color: var(--ap-semantic-danger);
  }
  .row-menu button.danger:hover {
    background: rgba(192, 88, 78, 0.12);
    color: var(--ap-semantic-danger);
  }
  .row-menu button.danger.confirming {
    background: rgba(192, 88, 78, 0.18);
  }
  .menu-scrim {
    position: fixed;
    inset: 0;
    z-index: 20;
    border: 0;
    background: transparent;
    cursor: default;
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
