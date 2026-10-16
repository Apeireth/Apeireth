<!-- 桌宠常驻窗 — presence 第四个投影（00-PHILOSOPHY §5）
     #
     # 职责：消费 presence_state 真实帧与网关生命周期事件，把「他在」显影到
     # 桌面常驻面。形象层经 AvatarHost 插拔（默认光态哨兵，主题包可换）；
     # 主动涌现经 proactive 引擎（防唠叨纪律：通报限频、开口限量、抬头非语言）。
     #
     # 交互契约（00-PHILOSOPHY §5）：
     #   点击     = 快捷对话窗（toggle_quick_window，外部处决销毁纪律同源）
     #   右键     = 卷宗入口（记忆/日记/审批治理/状态日志 → open_main_target）
     #   拖拽     = 移动窗位（pointer 位移判别，微位移不算拖）
     #   互斥礼让 = 主窗前台时本窗由壳层隐藏（Rust 侧决策，见 lib.rs）
     #
     # 0 装：气泡文本只来自真实事件（待签文书/他开口的 initiative 契约帧），
     # 无事件不说话；断连 >30s 外显 SIM 小字，绝不编造情绪。-->
<script lang="ts">
  import {onMount} from 'svelte';
  import {loadConfig, subscribeCompanionEvents, fetchCanonicalApprovals, resolveCanonicalApproval} from '../../lib/runtime';
  import type {CompanionEvent} from '../../lib/runtime';
  import {
    presenceStore,
    subscribePresence,
    derivePresenceGlow,
    deriveEmberBreath,
    PRESENCE_GLOW_REST,
  } from '../../lib/presence';
  import {
    derivePetFrame,
    DEFAULT_AVATAR_LAYER_ID,
    FALLBACK_AVATAR_LAYER_ID,
    CHARACTER_AVATAR_LAYER_ID,
    SKELETAL_2D_AVATAR_LAYER_ID,
    type PetFrame,
  } from '../../lib/pet/pet-contract';
  import {
    createInitialProactiveState,
    reduceProactive,
    evaluateGlance,
    isGlanceActive,
    type ProactiveState,
    type ProactiveSignal,
    type ProactiveSignalKind,
  } from '../../lib/pet/proactive';
  import {
    createInitialAgentStatus,
    reduceAgentStatus,
    decayAgentStatus,
    isStatusDisplayable,
    type AgentStatusState,
  } from '../../lib/pet/agent-status';
  import {
    parseApprovalQuickRecord,
    selectPendingApproval,
    reduceApprovalQuick,
    type ApprovalQuickState,
    type ApprovalDecision,
  } from '../../lib/pet/approval-quick';
  import AvatarHost from './AvatarHost.svelte';
  import {
    validateUserAvatarFile,
    getUserAvatar,
    putUserAvatar,
    clearUserAvatar,
    normalizeVrmMetaLicense,
    USER_AVATAR_DISCLAIMER,
    EMPTY_USER_AVATAR_META,
    type UserAvatarMeta,
  } from '../../lib/pet/user-avatar';
  import {petRenderPause} from '../../lib/pet/pet-render-pause';

  const config = loadConfig();

  const LAYER_KEY = 'apeireth:pet:avatar-layer';
  const RELOAD_KEY = 'apeireth:pet:avatar-reload';
  let layerId = $state(DEFAULT_AVATAR_LAYER_ID);
  let clickThrough = $state(false);
  let menuOpen = $state(false);
  let menuPos = $state({x: 0, y: 0});
  let proactive = $state<ProactiveState>(createInitialProactiveState());
  let glanceActive = $state(false);
  // 伴星状态机（I 批 1 轨 B）：只吃真实信号，无产点预留态不亮灯（0 装）。
  let agentStatus = $state<AgentStatusState>(createInitialAgentStatus());
  // 面板代审批（I 批 1 轨 B）：帧真源 + 权威核对 + 先答为准。
  let approvalQuick = $state<ApprovalQuickState>({phase: 'none'});
  let panelNote = $state<string | null>(null);
  // 用户自带形象（BYO，avatar-pack-admission.md §1b）：格式校验 + 免责确认后装载。
  let userModelUrl = $state<string | null>(null);
  let userAvatarName = $state<string | null>(null);
  let avatarNotice = $state<string | null>(null);
  let disclaimerFile = $state<File | null>(null);
  let avatarMeta = $state<UserAvatarMeta>(EMPTY_USER_AVATAR_META);
  let fileInput = $state<HTMLInputElement | null>(null);
  let userObjectUrl: string | null = null;

  async function decideApproval(decision: ApprovalDecision): Promise<void> {
    if (approvalQuick.phase !== 'asking') return;
    const record = approvalQuick.record;
    approvalQuick = reduceApprovalQuick(approvalQuick, {type: 'begin-resolve'});
    panelNote = null;
    try {
      // 权威核对：以待签队列按 approval_id 精确匹配（他人/别处已处理不双批）。
      if (!record.session || !record.approvalId) {
        approvalQuick = reduceApprovalQuick(approvalQuick, {type: 'stale'});
        panelNote = '帧缺主键，无法核对——请到主窗审批收件箱处理。';
        return;
      }
      const queue = await fetchCanonicalApprovals(config, record.session);
      const pending = selectPendingApproval(queue, record);
      if (!pending) {
        approvalQuick = reduceApprovalQuick(approvalQuick, {type: 'stale'});
        panelNote = '文书已不在待签队列（可能别处已处理）。';
        return;
      }
      await resolveCanonicalApproval(config, pending, decision);
      approvalQuick = reduceApprovalQuick(approvalQuick, {type: 'resolved'});
      panelNote = `已${decision === 'approve' ? '允许' : decision === 'reject' ? '拒绝' : '取消'}：${record.toolName ?? '工具'}`;
    } catch (caught) {
      // 失败如实回 asking 并外显原因（不吞错、不假成功）。
      approvalQuick = {phase: 'asking', record};
      panelNote = `决策失败：${caught instanceof Error ? caught.message : String(caught)}`;
    }
  }

  const frame = $derived.by((): PetFrame => {
    const s = $presenceStore;
    return derivePetFrame(
      {
        stance: s.current?.stance ?? null,
        mode: s.current?.mode ?? 'quiet',
        breath: deriveEmberBreath(s.breath),
        glow: derivePresenceGlow(s.current),
        simulated: s.simulated,
        connected: s.connected,
      },
      {glance: glanceActive, restGlow: PRESENCE_GLOW_REST},
    );
  });

  // —— 真实信号入口 ——
  const KNOWN_SIGNAL_KINDS: ReadonlySet<string> = new Set<ProactiveSignalKind>([
    'approval_required',
    'approval_resolved',
    'backend_ready',
    'turn_completed',
    'initiative',
  ]);

  function onCompanionEvent(event: CompanionEvent): void {
    const kind = event.kind ?? '';
    if (!KNOWN_SIGNAL_KINDS.has(kind)) return;
    const payload = event.payload ?? {};
    const signal: ProactiveSignal = {
      kind: kind as ProactiveSignalKind,
      at: event.ts,
      toolName: typeof payload.tool_name === 'string' ? payload.tool_name : null,
      text: typeof payload.text === 'string' ? payload.text : null,
    };
    proactive = reduceProactive(proactive, {type: 'signal', signal});
    glanceActive = isGlanceActive(proactive, signal.at);
    // 伴星：真实信号归约（未知种类零操作，见 agent-status 契约 §2）。
    agentStatus = reduceAgentStatus(agentStatus, {
      kind,
      at: event.ts,
      toolName: signal.toolName,
    });
    // 面板代审批：approval_required 建面板，approval_resolved 清面板（先答为准）。
    if (kind === 'approval_required') {
      const record = parseApprovalQuickRecord(payload, event.ts);
      if (record) {
        approvalQuick = reduceApprovalQuick(approvalQuick, {type: 'frame', record});
        panelNote = null;
      }
    } else if (kind === 'approval_resolved') {
      approvalQuick = reduceApprovalQuick(approvalQuick, {type: 'resolved'});
    }
  }

  onMount(() => {
    // 每次挂载重读形象层选择（设置面/别的窗可能改过）
    layerId = localStorage.getItem(LAYER_KEY) ?? DEFAULT_AVATAR_LAYER_ID;

    // 自带形象恢复（BYO）：本机存档 → 对象 URL；没有存过 = 默认形象（诚实缺省）
    let alive = true;
    void (async () => {
      try {
        const stored = await getUserAvatar();
        if (alive && stored && !userModelUrl) applyUserAvatar(stored.blob, true);
      } catch {
        // 读取失败不编造：保持默认形象
      }
    })();

    const unsubscribePresence = subscribePresence(config.baseUrl);
    const unsubscribeEvents = subscribeCompanionEvents(config, onCompanionEvent);

    // 跨窗同步（设置 › 形象改配置）：storage 事件是同源窗口约定——形象层即时换；
    // 形象文件变更经 reload 标记 → 重读本机存档（极端情况重开桌宠窗生效）。
    const onStorage = (e: StorageEvent) => {
      if (e.key === LAYER_KEY && e.newValue) {
        layerId = e.newValue;
      } else if (e.key === RELOAD_KEY) {
        void (async () => {
          try {
            const stored = await getUserAvatar();
            if (stored) applyUserAvatar(stored.blob, true);
            else if (userModelUrl) await removeUserAvatar();
          } catch {
            // 重读失败不编造：保持现状
          }
        })();
      }
    };
    window.addEventListener('storage', onStorage);

    // 宿主显式停帧闩（真隐藏口径）：壳层礼让隐身/唤起时发 pet-host-visibility，
    // 形象层渲染循环见闩即停（document.hidden 不随 window.hide() 翻转，实测）。
    let unlistenHostVis: (() => void) | null = null;
    void (async () => {
      try {
        const {listen} = await import('@tauri-apps/api/event');
        unlistenHostVis = await listen<boolean>('pet-host-visibility', (e) => {
          petRenderPause.paused = e.payload === false;
        });
      } catch {
        // 非桌面环境：document.hidden 单闩照常
      }
    })();

    // 1s 记账拍（非动画源）：气泡到期 + 抬头评估。动画一律 CSS 合成器自跑。
    const tickTimer = window.setInterval(() => {
      const at = Date.now();
      proactive = evaluateGlance(reduceProactive(proactive, {type: 'tick', at}), at);
      glanceActive = isGlanceActive(proactive, at);
      agentStatus = decayAgentStatus(agentStatus, at);
    }, 1000);

    return () => {
      alive = false;
      unsubscribePresence();
      unsubscribeEvents();
      window.clearInterval(tickTimer);
      window.removeEventListener('storage', onStorage);
      unlistenHostVis?.();
      // 对象 URL 是借的，还的时候自己 revoke
      if (userObjectUrl) {
        URL.revokeObjectURL(userObjectUrl);
        userObjectUrl = null;
      }
    };
  });

  // —— 窗口交互（拖拽判别：微位移不算拖，算点击） ——
  let pointerDown = $state(false);
  let dragMoved = false;
  let downX = 0;
  let downY = 0;

  function onPointerDown(e: PointerEvent): void {
    if (e.button !== 0 || menuOpen) return;
    pointerDown = true;
    dragMoved = false;
    downX = e.clientX;
    downY = e.clientY;
  }

  function onPointerMove(e: PointerEvent): void {
    if (!pointerDown || dragMoved) return;
    if (Math.abs(e.clientX - downX) + Math.abs(e.clientY - downY) > 5) {
      dragMoved = true;
      void startWindowDrag();
    }
  }

  function onPointerUp(e: PointerEvent): void {
    if (!pointerDown) return;
    pointerDown = false;
    const wasDrag = dragMoved;
    dragMoved = false;
    if (wasDrag || e.button !== 0) return;
    if (menuOpen) {
      menuOpen = false;
      return;
    }
    void toggleQuickWindow();
  }

  async function startWindowDrag(): Promise<void> {
    try {
      const {getCurrentWebviewWindow} = await import('@tauri-apps/api/webviewWindow');
      getCurrentWebviewWindow().startDragging();
    } catch {
      // 非桌面环境（浏览器预览）：拖拽是壳层能力，此处无事可做
    }
  }

  async function tauriInvoke(command: string, args?: Record<string, unknown>): Promise<void> {
    const {invoke} = await import('@tauri-apps/api/core');
    await invoke(command, args);
  }

  async function toggleQuickWindow(): Promise<void> {
    try {
      await tauriInvoke('toggle_quick_window');
    } catch {
      // 非桌面环境
    }
  }

  // —— 右键卷宗入口 ——
  function onContextMenu(e: MouseEvent): void {
    e.preventDefault();
    menuPos = {x: e.clientX, y: e.clientY};
    menuOpen = true;
  }

  const DOSSIER_ENTRIES: ReadonlyArray<{target: string; label: string}> = [
    {target: 'memory', label: '记忆卷宗'},
    {target: 'diary', label: '他的日记'},
    {target: 'governance', label: '审批与治理'},
    {target: 'logs', label: '状态日志'},
  ];

  async function openMainTarget(target: string): Promise<void> {
    menuOpen = false;
    try {
      await tauriInvoke('open_main_target', {target});
    } catch {
      // 非桌面环境
    }
  }

  /** 形象层切换循环：光态哨兵（默认）→ 角色形象（华服）→ 2D 骨骼（华服二）→ 微光简模（T0） */
  const LAYER_CYCLE = [
    DEFAULT_AVATAR_LAYER_ID,
    CHARACTER_AVATAR_LAYER_ID,
    SKELETAL_2D_AVATAR_LAYER_ID,
    FALLBACK_AVATAR_LAYER_ID,
  ] as const;

  function toggleAvatarLayer(): void {
    menuOpen = false;
    const index = LAYER_CYCLE.indexOf(layerId as (typeof LAYER_CYCLE)[number]);
    layerId = LAYER_CYCLE[(index + 1) % LAYER_CYCLE.length];
    localStorage.setItem(LAYER_KEY, layerId);
  }

  async function toggleClickThrough(): Promise<void> {
    menuOpen = false;
    const next = !clickThrough;
    try {
      await tauriInvoke('set_pet_click_through', {enabled: next});
      clickThrough = next;
    } catch {
      // 非桌面环境：状态不谎报
    }
  }

  async function hidePet(): Promise<void> {
    menuOpen = false;
    try {
      await tauriInvoke('close_pet_window');
    } catch {
      // 非桌面环境
    }
  }

  // —— 用户自带形象（BYO）装载口：格式合法性校验 + 免责确认，许可责任用户侧 ——
  function pickUserAvatar(): void {
    menuOpen = false;
    fileInput?.click();
  }

  function showAvatarNotice(text: string): void {
    avatarNotice = text;
    window.setTimeout(() => {
      if (avatarNotice === text) avatarNotice = null;
    }, 8000);
  }

  async function onAvatarFileSelected(e: Event): Promise<void> {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0] ?? null;
    input.value = ''; // 同名重选要能再次触发 change
    if (!file) return;
    let head: Uint8Array | null = null;
    try {
      head = new Uint8Array(await file.slice(0, 8).arrayBuffer());
    } catch {
      head = null;
    }
    const verdict = validateUserAvatarFile(file.name, file.size, head);
    if (!verdict.ok) {
      showAvatarNotice(verdict.reason);
      return;
    }
    disclaimerFile = file; // 免责确认通过才真装载
  }

  async function acceptDisclaimer(): Promise<void> {
    const file = disclaimerFile;
    disclaimerFile = null;
    if (!file) return;
    try {
      await putUserAvatar(file, Date.now());
    } catch (caught) {
      showAvatarNotice(`形象持久化失败：${caught instanceof Error ? caught.message : String(caught)}`);
      return;
    }
    applyUserAvatar(file);
  }

  function applyUserAvatar(file: Blob, silent = false): void {
    if (userObjectUrl) URL.revokeObjectURL(userObjectUrl);
    userObjectUrl = URL.createObjectURL(file);
    userModelUrl = userObjectUrl;
    userAvatarName = (file as File).name ?? '自带形象';
    layerId = CHARACTER_AVATAR_LAYER_ID; // 换上华服：形象层自动切到角色形象
    localStorage.setItem(LAYER_KEY, layerId);
    if (!silent) showAvatarNotice('已换上你的形象');
  }

  async function removeUserAvatar(): Promise<void> {
    menuOpen = false;
    try {
      await clearUserAvatar();
    } catch {
      showAvatarNotice('本机存档清除失败（内存中的形象已移除）');
    }
    if (userObjectUrl) {
      URL.revokeObjectURL(userObjectUrl);
      userObjectUrl = null;
    }
    userModelUrl = null;
    userAvatarName = null;
    avatarMeta = EMPTY_USER_AVATAR_META;
    if (layerId === CHARACTER_AVATAR_LAYER_ID) {
      layerId = DEFAULT_AVATAR_LAYER_ID;
      localStorage.setItem(LAYER_KEY, layerId);
    }
  }

  function handleModelMeta(meta: UserAvatarMeta): void {
    avatarMeta = meta;
  }

  const LAYER_NAMES: Readonly<Record<string, string>> = {
    [DEFAULT_AVATAR_LAYER_ID]: '光态哨兵',
    [CHARACTER_AVATAR_LAYER_ID]: '角色形象',
    [SKELETAL_2D_AVATAR_LAYER_ID]: '2D 骨骼',
    [FALLBACK_AVATAR_LAYER_ID]: '微光简模',
  };
  const layerLabel = $derived(`形象层：${LAYER_NAMES[layerId] ?? layerId}（点击切换）`);
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="pet-root"
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  oncontextmenu={onContextMenu}
  role="application"
  aria-label="常驻桌宠"
>
  {#if menuOpen}
    <button
      type="button"
      class="menu-backdrop"
      aria-label="关闭菜单"
      onclick={() => (menuOpen = false)}
    ></button>
    <div class="pet-menu" style:left={`${menuPos.x}px`} style:top={`${menuPos.y}px`} role="menu">
      {#each DOSSIER_ENTRIES as entry (entry.target)}
        <button type="button" class="menu-item" role="menuitem" onclick={() => openMainTarget(entry.target)}>
          {entry.label}
        </button>
      {/each}
      <div class="menu-sep" role="separator"></div>
      <button type="button" class="menu-item" role="menuitem" onclick={toggleAvatarLayer}>{layerLabel}</button>
      <button type="button" class="menu-item" role="menuitem" onclick={pickUserAvatar}>
        {userAvatarName ? '换一个形象文件…' : '接入形象文件…'}
      </button>
      {#if userAvatarName}
        <button type="button" class="menu-item" role="menuitem" onclick={removeUserAvatar}>移除形象文件</button>
        <div class="menu-meta" title="读取自形象文件内嵌声明；平台不解读不背书">
          {userAvatarName}｜许可声明：
          {avatarMeta.licenseUrl ?? avatarMeta.commercialUsage ?? avatarMeta.redistribution
            ? [avatarMeta.commercialUsage, avatarMeta.redistribution, avatarMeta.licenseUrl]
                .filter(Boolean)
                .join(' · ')
            : '未声明'}
        </div>
      {/if}
      <button type="button" class="menu-item" role="menuitem" onclick={toggleClickThrough}>
        {clickThrough ? '取消鼠标穿透' : '鼠标穿透'}
      </button>
      <button type="button" class="menu-item" role="menuitem" onclick={hidePet}>隐藏桌宠</button>
    </div>
  {/if}

  {#if frame.simulated}
    <span class="sim-mark" title="频道断连 >30s：本机中性回落，不编造情绪">SIM</span>
  {/if}

  {#if isStatusDisplayable(agentStatus.status)}
    <span
      class="status-mark"
      title={agentStatus.pendingTool
        ? `等待批准：${agentStatus.pendingTool}`
        : agentStatus.status === 'working'
          ? '他正在干活（90s 无信号回落）'
          : '回合收工（30s 后回落）'}
    >{agentStatus.status === 'awaiting_approval' ? '等签' : agentStatus.status === 'working' ? '在忙' : '收工'}</span>
  {/if}

  {#if approvalQuick.phase === 'asking' || approvalQuick.phase === 'resolving' || approvalQuick.phase === 'stale'}
    <div class="approval-quick" role="group" aria-label="待签文书快捷决策">
      <div class="approval-quick-title">
        等待批准：{approvalQuick.record.toolName ?? '工具'}
      </div>
      {#if approvalQuick.phase === 'stale'}
        <div class="approval-quick-note">{panelNote ?? '文书已不在待签队列。'}</div>
      {:else}
        <div class="approval-quick-actions">
          <button
            type="button"
            class="approval-btn approve"
            disabled={approvalQuick.phase === 'resolving'}
            onclick={() => decideApproval('approve')}
          >允许</button>
          <button
            type="button"
            class="approval-btn reject"
            disabled={approvalQuick.phase === 'resolving'}
            onclick={() => decideApproval('reject')}
          >拒绝</button>
          <button
            type="button"
            class="approval-btn cancel"
            disabled={approvalQuick.phase === 'resolving'}
            onclick={() => decideApproval('cancel')}
          >取消</button>
        </div>
        {#if panelNote}
          <div class="approval-quick-note">{panelNote}</div>
        {/if}
      {/if}
    </div>
  {/if}

  <div class="bubble-stack" aria-live="polite">
    {#if avatarNotice}
      <div class="bubble bubble-notice">{avatarNotice}</div>
    {/if}
    {#each proactive.bubbles as bubble (bubble.id)}
      <div class="bubble bubble-{bubble.kind}">{bubble.text}</div>
    {/each}
  </div>

  <div class="avatar-slot">
    <AvatarHost {frame} {layerId} modelUrl={userModelUrl} onModelMeta={handleModelMeta} />
  </div>

  <input
    bind:this={fileInput}
    type="file"
    accept=".vrm,.glb"
    style="display: none"
    onchange={onAvatarFileSelected}
  />

  {#if disclaimerFile}
    <div class="disclaimer-backdrop" role="presentation">
      <div class="disclaimer-card" role="dialog" aria-label="自带形象免责声明">
        <div class="disclaimer-text">{USER_AVATAR_DISCLAIMER}</div>
        <div class="disclaimer-file">文件：{disclaimerFile.name}</div>
        <div class="disclaimer-actions">
          <button type="button" class="approval-btn approve" onclick={acceptDisclaimer}>
            我确认有权使用，换上
          </button>
          <button type="button" class="approval-btn cancel" onclick={() => (disclaimerFile = null)}>
            取消
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  :global(html),
  :global(body),
  :global(#app) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    background: transparent;
    overflow: hidden;
    user-select: none;
  }

  .pet-root {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    cursor: grab;
  }
  .pet-root:active {
    cursor: grabbing;
  }

  .avatar-slot {
    width: 100%;
    height: 78%;
  }

  /* —— 气泡（他的话语与事实通报；金 = 他，通报走骨白） —— */
  .bubble-stack {
    position: absolute;
    top: 6px;
    left: 50%;
    transform: translateX(-50%);
    width: 92%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    pointer-events: none;
  }

  .bubble {
    max-width: 100%;
    padding: 6px 10px;
    border-radius: 10px;
    font-size: 12px;
    line-height: 1.5;
    font-family: system-ui, -apple-system, 'PingFang SC', 'Microsoft YaHei UI', sans-serif;
    background: rgba(11, 13, 18, 0.88);
    border: 1px solid rgba(232, 224, 204, 0.18);
    color: #e8e0cc;
    animation: bubble-in 0.42s cubic-bezier(0.22, 1, 0.36, 1);
    word-break: break-word;
  }

  .bubble-speech {
    /* 金色纪律：他开口用存在金 */
    color: #ffd27a;
    border-color: rgba(255, 210, 122, 0.4);
    font-family: 'Songti SC', 'Noto Serif CJK SC', 'STSong', 'SimSun', serif;
  }

  @keyframes bubble-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* —— SIM 标注（设计规范 §5.4：等宽小字，如实外显） —— */
  .sim-mark {
    position: absolute;
    top: 4px;
    right: 6px;
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 9px;
    letter-spacing: 0.28em;
    color: rgba(232, 224, 204, 0.5);
    pointer-events: none;
  }

  /* —— 伴星状态标（I 批 1 轨 B：等签/在忙/收工，只亮真实源态） —— */
  .status-mark {
    position: absolute;
    top: 4px;
    right: 38px;
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 9px;
    letter-spacing: 0.28em;
    color: rgba(255, 210, 122, 0.85);
    pointer-events: none;
  }

  /* —— 面板代审批（I 批 1 轨 B：等签文书快捷决策，先答为准） —— */
  .approval-quick {
    position: absolute;
    top: 22px;
    left: 50%;
    transform: translateX(-50%);
    min-width: 168px;
    padding: 6px 8px;
    border-radius: 8px;
    background: rgba(11, 13, 18, 0.96);
    border: 1px solid rgba(255, 210, 122, 0.35);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 8;
  }

  .approval-quick-title {
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 10px;
    letter-spacing: 0.08em;
    color: rgba(232, 224, 204, 0.9);
    word-break: break-all;
  }

  .approval-quick-actions {
    display: flex;
    gap: 6px;
  }

  .approval-btn {
    appearance: none;
    border-radius: 6px;
    padding: 3px 10px;
    font-size: 11px;
    cursor: pointer;
    background: transparent;
    border: 1px solid rgba(232, 224, 204, 0.3);
    color: rgba(232, 224, 204, 0.9);
  }

  .approval-btn.approve {
    border-color: rgba(255, 210, 122, 0.55);
    color: #ffd27a;
  }

  .approval-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .approval-quick-note {
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 9px;
    color: rgba(232, 224, 204, 0.6);
    word-break: break-all;
  }

  /* —— 右键卷宗菜单 —— */
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9;
    appearance: none;
    border: none;
    padding: 0;
    background: transparent;
    cursor: default;
  }

  .pet-menu {
    position: fixed;
    z-index: 10;
    min-width: 132px;
    padding: 4px;
    border-radius: 8px;
    background: rgba(11, 13, 18, 0.96);
    border: 1px solid rgba(232, 224, 204, 0.22);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
  }

  .menu-item {
    appearance: none;
    background: none;
    border: none;
    color: #e8e0cc;
    font-size: 12px;
    text-align: left;
    padding: 6px 10px;
    border-radius: 6px;
    cursor: pointer;
  }
  .menu-item:hover {
    background: rgba(255, 210, 122, 0.12);
    color: #ffd27a;
  }

  .menu-sep {
    height: 1px;
    margin: 4px 6px;
    background: rgba(232, 224, 204, 0.14);
  }

  /* —— 自带形象（BYO）：免责确认与菜单信息行 —— */
  .menu-meta {
    padding: 4px 10px 6px;
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 9px;
    line-height: 1.5;
    letter-spacing: 0.06em;
    color: rgba(232, 224, 204, 0.55);
    word-break: break-all;
  }

  .disclaimer-backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: rgba(7, 7, 12, 0.55);
    display: grid;
    place-items: center;
  }

  .disclaimer-card {
    width: 88%;
    padding: 12px;
    border-radius: 10px;
    background: rgba(11, 13, 18, 0.97);
    border: 1px solid rgba(255, 210, 122, 0.35);
  }

  .disclaimer-text {
    font-size: 12px;
    line-height: 1.7;
    color: #e8e0cc;
  }

  .disclaimer-file {
    margin-top: 6px;
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 10px;
    color: rgba(232, 224, 204, 0.6);
    word-break: break-all;
  }

  .disclaimer-actions {
    margin-top: 10px;
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  @media (prefers-reduced-motion: reduce) {
    .bubble {
      animation: none;
    }
  }
</style>
