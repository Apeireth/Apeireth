<script lang="ts">
  import {onMount, tick} from 'svelte';
  import {
    Plus,
    ArrowUp,
    Square,
    ChevronDown,
    Sparkles,
    PhoneCall,
    Sofa,
    Gauge,
    Eclipse,
    MessageCircleMore,
    History,
    Layers3,
    BookOpen,
    Wrench,
    Landmark,
    Activity,
    ScrollText,
    Settings,
    PanelRight,
    X,
    Search,
    Info,
    ShieldCheck,
  } from 'lucide-svelte';
  import MessageContent from './lib/MessageContent.svelte';
  import RuntimeModal from './lib/components/RuntimeModal.svelte';
  import FirstRunWizard from './lib/components/FirstRunWizard.svelte';
  import VoiceCallModal from './components/VoiceCallModal.svelte';
  import { voiceCallManager } from './lib/voice';

  import ErrorSolutionBanner from './lib/components/ErrorSolutionBanner.svelte';
  import ToolCallLifecycleCard from './lib/components/ToolCallLifecycleCard.svelte';
  import ComposerMenu from './lib/components/ComposerMenu.svelte';
  import CommandPalette from './lib/components/CommandPalette.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import {
    sseSourceOf,
    sseIndicator,
    turnIndicator,
    guardIndicator,
    memoryIndicator,
  } from './lib/statusbar';
  import {
    pushRecentId,
    loadRecentIds,
    saveRecentIds,
    type CommandItem,
  } from './lib/commands/registry';
  import SessionModelPicker from './lib/components/SessionModelPicker.svelte';
  import WorkspacePickerModal from './lib/components/WorkspacePickerModal.svelte';
  import {
    getWorkspaceDir,
    setWorkspaceDir,
    listWorkspaceSuggestions,
  } from './lib/tauri-bridge';
  import SceneLayer from './lib/scene/SceneLayer.svelte';
  import PlanetLayer from './lib/scene/PlanetLayer.svelte';
  import BridgeLayer from './lib/bridge/BridgeLayer.svelte';
  import DeepCabinLayer from './lib/cabin/DeepCabinLayer.svelte';
  import IntroLayer from './lib/intro/IntroLayer.svelte';
  import {localClockHour} from './lib/scene/timeline';
  import ConversationsView from './lib/ConversationsView.svelte';
  import SessionListHome from './lib/chat-shell/SessionListHome.svelte';
  import PendingDocumentDock from './lib/chat-shell/PendingDocumentDock.svelte';
  import GuardNoticeCard from './lib/chat-shell/GuardNoticeCard.svelte';
  import type {HomeSessionItem} from './lib/chat-shell/session-list';
  import {
    applyApprovalEventToPending,
    classifyGovernanceNotice,
    parseApprovalEventPayload,
  } from './lib/chat-shell/gateway-events';
  import ActivityView from './lib/views/ActivityView.svelte';
  import ToolsView from './lib/views/ToolsView.svelte';
  import GovernanceView, {type GovernanceTabId} from './lib/views/GovernanceView.svelte';
  import MemoryView from './lib/MemoryView.svelte';
  import DiaryView from './lib/views/DiaryView.svelte';
  import SettingsView from './lib/views/SettingsView.svelte';
  import Workbench from './lib/components/Workbench.svelte';
  import {applyDocumentAccent, applyDocumentTheme, isStaticBgTheme, resolveAccent, resolveTheme, themeLabel, THEME_CATALOG} from './lib/theme';
  import {getCustomBg} from './lib/bg-store';
  import type {Theme} from './lib/types';

  import type {
    ApeirethConfig,
    ApprovalRequestItem,
    CapabilityManifest,
    ChatMessage,
    Conversation,
    HealthState,
    ModelInfo,
    RuntimeHealthReport,
    SessionSettings,
    ToolCallDetails,
    WorkbenchTurn,
    GuardStatus,
  } from './lib/types';
  import {
    checkHealthDetailed,
    createAgentRuntime,
    fetchCanonicalApprovals,
    resolveCanonicalApproval,
    applyCanonicalEvents,
    fetchWorkbenchTurn,
    ApprovalRequiredError,
    loadConfig,
    loadConversations,
    saveConfig,
    saveConversations,
    listModels,
    describeError,
    getSessionSettings,
    patchSessionSettings,
    fetchCapabilities,
    fetchGuardStatus,
    fetchGuardEvents,
    fetchMemoryEpisodes,
    subscribeCompanionEvents,
    capabilityAvailable,
    capabilitySupported,
    capabilityUnavailableReason,
    activePersonaOf,
    DEFAULT_PERSONAS,
    type CompanionPresentationState,
    type CanonicalPendingApproval,
  } from './lib/runtime';
  import {presenceStore, subscribePresence, derivePresenceGlow} from './lib/presence';
  import {
    applyBackendConfig,
    backendProviderEnvFromConfig,
    capabilityEnvFromConfig,
    isDesktop,
    resolveBackendEndpoint,
  } from './lib/desktop-bridge';

  type DrawerId = 'history' | 'memory' | 'diary' | 'tools' | 'governance' | 'status' | 'logs' | 'settings';
  const DRAWER_META: Record<DrawerId, {eyebrow: string; title: string; sub: string; action: string}> = {
    history: {
      eyebrow: '管理',
      title: '历史',
      sub: '本地对话上下文与后端持久账本；删除需确认，归档不丢记录。',
      action: '新对话',
    },
    memory: {
      eyebrow: '认知 · 纸面档案',
      title: '记忆卷宗',
      sub: '持久化情节记忆的主从卷宗——检索、出处、图谱关联与保护/遗忘治理。',
      action: '',
    },
    diary: {
      eyebrow: '档案 · 纸面',
      title: '他的日记',
      sub: '他写下的日子——纸面档案调（§5.6②）；后端尚无日记端点，当前为空态契约页。',
      action: '',
    },
    tools: {
      eyebrow: '能力',
      title: '工具管理与权限',
      sub: '注册工具、参数规范及待主人批准的高危调用。',
      action: '',
    },
    governance: {
      eyebrow: '照看 · 事后卷宗',
      title: '治理卷宗',
      sub: '审批的账、授权的账、守卫的账、执行的账——对话内完成的判断，在这里成卷。',
      action: '',
    },
    status: {
      eyebrow: '微内核',
      title: '系统状态',
      sub: '网关、模型服务、账本与记忆流的实时探测。',
      action: '深度诊断',
    },
    logs: {
      eyebrow: '观察与审计',
      title: '活动与调用日志',
      sub: '每一轮交互的延迟、Token、执行事件与工具调用轨迹。',
      action: '',
    },
    settings: {
      eyebrow: '首选项',
      title: '设置',
      sub: '模型提供商、人设、记忆策略、权限与数据。',
      action: '',
    },
  };

  // ---------- 波次 4：三模式骨架 ----------
  // companion=陪伴（舰桥+对话，默认）｜engineering=工程（深舱+页面层）｜focus=专注（临渊机位+chrome 淡出）
  type ModeId = 'companion' | 'engineering' | 'focus';
  // 开发覆写 ?mode=focus|engineering（与 ?hour= 同纪律：初始模式按参数设定，供无头截图验证）
  const modeQuery =
    typeof window !== 'undefined' ? new URLSearchParams(window.location.search).get('mode') : null;
  const initialMode: ModeId =
    modeQuery === 'engineering' || modeQuery === 'focus' ? modeQuery : 'companion';
  let mode = $state<ModeId>(initialMode);

  const modes = [
    {id: 'companion' as const, label: '陪伴 · 舰桥', icon: Sofa},
    {id: 'engineering' as const, label: '工程 · 深舱', icon: Gauge},
    {id: 'focus' as const, label: '专注 · 临渊', icon: Eclipse},
  ];

  const themeQuery =
    typeof window !== 'undefined' ? new URLSearchParams(window.location.search).get('theme') : null;
  let activeTheme = $state<Theme>(resolveTheme(loadConfig().theme, themeQuery));
  const isEssenceTheme = $derived(activeTheme === 'essence');
  // 静态背景主题（规范 §8 增补）：场景层隐藏 + 渲染循环暂停；自定义上传背景
  // 开启时同样盖掉场景（画面被静态图接管），暂停同样生效。
  const isHeritageTheme = $derived(activeTheme === 'heritage-void');
  // 自定义上传背景（规范 §8 增补④）：blob 持久化在 IndexedDB（bg-store.ts），
  // 这里只持有其对象 URL；null = 未启用。加载/清除逻辑随设置面板接线。
  let customBgUrl = $state<string | null>(null);
  const isStaticScene = $derived(isStaticBgTheme(activeTheme) || customBgUrl !== null);

  /** 自定义背景开关同步：开 = 从 IndexedDB 取图；取不到（换原点/清库）诚实回落关开关。 */
  async function syncCustomBg(on: boolean): Promise<void> {
    if (!on) {
      if (customBgUrl) URL.revokeObjectURL(customBgUrl);
      customBgUrl = null;
      return;
    }
    const blob = await getCustomBg().catch(() => null);
    if (blob) {
      if (customBgUrl) URL.revokeObjectURL(customBgUrl);
      customBgUrl = URL.createObjectURL(blob);
    } else {
      customBgUrl = null;
      const back = {...config, customBg: false};
      config = back;
      saveConfig(back);
    }
  }

  // ---------- 开场动画（火之文明史序章）门禁 ----------
  // 【2026-08-22 封存】v1 审美验收未过（主人评：一言难尽），默认关闭不再自动播放，
  // 保留全部引擎代码待额度充足后重启打磨（详见 docs/design/intro-animation.md）。
  // 重看方式：?intro=1 强制重播；?it=<秒> 冻结开场时钟供无头截图（永不自然播完）；
  // prefers-reduced-motion → 强制参数也不播，直接进产品。
  // 播放期间 SceneLayer/PlanetLayer/BridgeLayer 全程挂载绝不卸载（无缝接缝的物理基础），
  // 全部 chrome 以 .intro-playing class 隐藏；落幅 1.5s IntroLayer 淡出，活舰桥显形。
  const introQuery =
    typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : null;
  const introForced = introQuery?.get('intro') === '1' || (introQuery?.has('it') ?? false);
  const introReduceMotion =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  // 封存期：仅显式强制参数才播（reduced-motion 仍有最终否决权）
  let introPlaying = $state(introForced && !introReduceMotion);

  function handleIntroComplete(): void {
    introPlaying = false;
    try {
      localStorage.setItem('ap-intro-seen', '1');
    } catch {
      /* 存储不可用时静默跳过 */
    }
  }

  // 初始视图：对话始终居中；工程/专注只切场景层，不再把主区换成页面层
  // 开发覆写 ?drawer=<id>&govtab=<tab>（与 ?mode= 同纪律：仅供无头截图/联调
  // 直接落到某个抽屉与卷宗 tab，正常启动不受影响）。
  const drawerQuery =
    typeof window !== 'undefined' ? new URLSearchParams(window.location.search).get('drawer') : null;
  const govtabQuery =
    typeof window !== 'undefined' ? new URLSearchParams(window.location.search).get('govtab') : null;
  const initialDrawer: DrawerId | null =
    drawerQuery === 'history' || drawerQuery === 'memory' || drawerQuery === 'diary' ||
    drawerQuery === 'tools' || drawerQuery === 'governance' || drawerQuery === 'status' ||
    drawerQuery === 'logs' || drawerQuery === 'settings'
      ? drawerQuery
      : null;
  // govInitialTab 可变：状态条「守卫计数 → 守卫 tab」入口需要指令式落 tab；
  // GovernanceView 的 initialTab 是挂载快照，配 govTabKey 重挂载生效。
  let govInitialTab = $state<GovernanceTabId>(
    govtabQuery === 'grants' || govtabQuery === 'guard' || govtabQuery === 'audit'
      ? govtabQuery
      : 'approvals',
  );
  let govTabKey = $state(0);
  let drawerSec = $state<DrawerId | null>(initialDrawer);
  let wbOpen = $state(false);
  let workbenchTurn = $state<WorkbenchTurn | null>(null);
  let openPanel = $state<'model' | 'ctx' | null>(null);
  let availableModels = $state<string[]>([]);
  let modelsLoading = $state(false);
  let modelQuery = $state('');

  // 场景受控机位：专注=临渊(1)，陪伴=远眺(0)，工程=null（深舱不透明盖住场景，引擎自管理）
  const sceneCamera = $derived(mode === 'focus' ? 1 : mode === 'engineering' ? null : 0);

  function setMode(next: ModeId): void {
    if (next === mode) return;
    mode = next;
    if (next !== 'focus') {
      drawerSec = null;
    }
  }

  // 点黑洞 = 进入专注模式（§4.2 临渊机位由引擎承担，此处管模式）；
  // 工程模式下深舱盖住黑洞，忽略穿透到场景的点击
  function handleBlackholeClick(): void {
    if (mode === 'engineering') return;
    setMode('focus');
  }

  // Esc 退出专注回陪伴
  function handleModeKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape' && mode === 'focus') setMode('companion');
  }

  // 舰内时刻（规范 §3 时间线照明）：默认跟随本地时钟，30s 心跳刷新；
  // 开发调试覆写 ?hour=22 强制指定时刻（截图验证各时间档用），覆写时不走时钟。
  const hourQuery =
    typeof window !== 'undefined' ? new URLSearchParams(window.location.search).get('hour') : null;
  const hourOverride =
    hourQuery !== null && hourQuery.trim() !== '' && Number.isFinite(Number(hourQuery))
      ? Number(hourQuery)
      : null;
  let timelineHour = $state(hourOverride ?? localClockHour());
  let config = $state<ApeirethConfig>(loadConfig());
  const activePersona = $derived(activePersonaOf(config));
  const personaList = $derived(
    config.personas && config.personas.length > 0 ? config.personas : DEFAULT_PERSONAS,
  );
  let personaMenuOpen = $state(false);

  function setActivePersona(id: string): void {
    const target = personaList.find((p) => p.id === id);
    if (!target) return;
    const next: ApeirethConfig = {...config, activePersonaId: id};
    if (target.model) next.model = target.model;
    config = next;
    saveConfig(next);
    agentRuntime = createAgentRuntime(next);
    personaMenuOpen = false;
  }

  let conversations = $state<Conversation[]>(loadConversations());
  // T0 聊天壳（00-PHILOSOPHY §3.1 消息列表是主页）：activeId 缺省为 null ——
  // 打开应用第一屏 = 会话列表（「谁找我了」），进入会话才是消息流 + composer。
  let activeId = $state<string | null>(null);
  // 会话级「待签」账本（SSE approval_required − approval_resolved 推导，真实信号）；
  // 主页列表据此打金色待签标。applyApprovalEventToPending 不可变更新。
  let pendingApprovalSessions = $state<ReadonlySet<string>>(new Set());
  // 主页账本重拉节拍（refreshConnection / SSE 事件后 bump）。
  let homeReloadKey = $state(0);
  // 三栏主从（2026-09-22 主人拍板）：窄窗（≤980px）下列表栏折叠为覆盖层，
  // 此开关只在窄窗生效（桌面端 .session-col 常驻，open class 无视觉效果）。
  let sessionColOpen = $state(false);
  // 背景显影区 caption（§8 增补①）：如实标注当前背景来源。
  const bgCaption = $derived(
    customBgUrl !== null
      ? '自定义上传图片'
      : `${themeLabel(activeTheme)} · ${isStaticBgTheme(activeTheme) ? '静态图' : '实时场景'}`,
  );
  // 从主页点开的 backend-only 会话：本机无消息副本，hero 区给诚实标注。
  let ledgerHint = $state<{id: string; episodeCount: number} | null>(null);
  let draft = $state('');
  let busy = $state(false);
  let error = $state('');
  /** 最近一次错误对象（保留 code/solution，供 ErrorSolutionBanner 使用）。 */
  let lastError = $state<unknown>(null);
  /** 非错误的状态提示（如"审批已提交、工具执行中"）——绝不挂"请重试"方案。 */
  let notice = $state('');
  let pendingApprovals = $state<ApprovalRequestItem[]>([]);
  let pendingCanonical = $state<CanonicalPendingApproval | null>(null);
  let approvalBusy = $state(false);
  let isReasoning = $state(false);
  let isExecutingTool = $state(false);
  let legacyToast = $state('');
  let agentRuntime = $state(createAgentRuntime(loadConfig()));

  // 会话级设置（GET/PATCH /v1/sessions/{id}/settings）与会话模型选择器数据。
  let sessionSettings = $state<SessionSettings | null>(null);
  let sessionModels = $state<ModelInfo[]>([]);

  // 全局权限预设（设置页 tools 区写入 localStorage）接入新会话创建：
  // 新建会话/分支时读入 pendingPreset[conversationId]，首次 send 完成后静默 PATCH 到后端。
  const GLOBAL_PRESET_KEY = 'apeireth-permission-preset-default';
  let pendingPreset = $state<Record<string, 'read_only' | 'standard' | 'full'>>({});
  // getSessionSettings 成功过的会话 id —— pendingPreset 不覆盖已有后端 settings。
  let sessionsWithSettings = $state<Record<string, boolean>>({});

  // 斜杠菜单（输入框聚焦且首字符 "/" 时显示）。
  let composerFocused = $state(false);
  let composerMenuOpen = $state(false);
  let composerTextarea = $state<HTMLTextAreaElement | null>(null);

  // ---------- 会话头审批策略（P1-2）----------
  // 4 档：只读 / 标准·每次审批 / 标准·会话内记住 / 完全放行。
  // approval_remember 仅对 standard 档有意义（会话内记住审批结果）。
  type SessionApprovalStrategy = {
    id: string;
    permission_preset: SessionSettings['permission_preset'];
    approval_remember: boolean;
    label: string;
    title: string;
  };
  const SESSION_PRESETS: SessionApprovalStrategy[] = [
    {id: 'read_only', permission_preset: 'read_only', approval_remember: false, label: '只读', title: '只读：工具仅读，写操作需审批'},
    {id: 'standard', permission_preset: 'standard', approval_remember: false, label: '标准·每次审批', title: '标准：常规工具，高危操作每次审批'},
    {id: 'standard_remember', permission_preset: 'standard', approval_remember: true, label: '标准·会话内记住', title: '标准：高危操作在本会话内记住审批结果'},
    {id: 'full', permission_preset: 'full', approval_remember: false, label: '完全放行', title: '完全：全部工具（仍受治理审批约束）'},
  ];

  // ---------- 斜杠命令（P1-6）----------
  const SLASH_COMMANDS = [
    {command: '/new', title: '新建会话', hint: '开启一个全新会话'},
    {command: '/clear', title: '清空上下文', hint: '清空当前会话消息'},
    {command: '/model', title: '切换模型', hint: '打开模型选择器'},
    {command: '/help', title: '帮助', hint: '查看快捷键与说明'},
  ];

  const currentSessionModel = $derived(sessionSettings?.model ?? config.model);
  // 会话头策略 active 态：permission_preset + approval_remember 共同决定。
  const activeApprovalStrategyId = $derived.by(() => {
    const preset = sessionSettings?.permission_preset ?? 'standard';
    const remember = sessionSettings?.approval_remember ?? false;
    return (
      SESSION_PRESETS.find(
        (p) => p.permission_preset === preset && p.approval_remember === remember,
      )?.id ?? 'standard'
    );
  });

  // ---- 对话栏位常驻控制（2026-10-11 主人反馈批：Kimi Desktop 式输入栏布局）----
  // 权限档位芯片的短标签（输入栏位空间紧，完整语义进弹层与 title）。
  const PRESET_SHORT_LABEL: Record<string, string> = {
    read_only: '只读',
    standard: '标准·每次',
    standard_remember: '标准·记住',
    full: '完全放行',
  };
  const activePresetShortLabel = $derived(
    PRESET_SHORT_LABEL[activeApprovalStrategyId] ?? '标准·每次',
  );
  const activePresetFull = $derived(
    SESSION_PRESETS.find((p) => p.id === activeApprovalStrategyId),
  );
  let composerPresetOpen = $state(false);
  async function pickComposerPreset(id: string): Promise<void> {
    composerPresetOpen = false;
    const preset = SESSION_PRESETS.find((p) => p.id === id);
    if (preset && preset.id !== activeApprovalStrategyId) {
      await selectSessionPreset(preset);
    }
  }

  function toEpochMs(value: unknown): number | undefined {
    if (typeof value === 'number') return Number.isFinite(value) ? value : undefined;
    if (typeof value === 'string') {
      const t = Date.parse(value);
      return Number.isNaN(t) ? undefined : t;
    }
    return undefined;
  }

  function ensureSessionSettings(patch: Partial<SessionSettings>): SessionSettings {
    const cur = sessionSettings;
    return {
      model: patch.model !== undefined ? patch.model : (cur?.model ?? null),
      permission_preset:
        patch.permission_preset ?? cur?.permission_preset ?? 'standard',
      approval_remember:
        patch.approval_remember ?? cur?.approval_remember ?? false,
    };
  }

  function readGlobalPreset(): 'read_only' | 'standard' | 'full' | null {
    try {
      const raw = localStorage.getItem(GLOBAL_PRESET_KEY);
      if (raw === 'read_only' || raw === 'standard' || raw === 'full') return raw;
    } catch {
      // localStorage 不可用时静默跳过。
    }
    return null;
  }

  function markPendingPreset(conversationId: string): void {
    const preset = readGlobalPreset();
    if (preset) {
      pendingPreset = {...pendingPreset, [conversationId]: preset};
    }
  }

  function clearPendingPreset(conversationId: string): void {
    if (!(conversationId in pendingPreset)) return;
    const next = {...pendingPreset};
    delete next[conversationId];
    pendingPreset = next;
  }

  /**
   * 全局权限预设接入会话创建：在 send() 完成后（成功/失败，只要 backend 会话已创建）
   * 静默 PATCH 一次。时机选在 finally 而非会话创建处，是因为 backend 在首个 chat 请求
   * 之前未必有该会话记录（GET/PATCH settings 会 404），创建处就打补丁会空耗一次失败。
   */
  async function applyPendingPreset(conversationId: string): Promise<void> {
    const preset = pendingPreset[conversationId];
    if (!preset) return;
    // 会话已有 settings（getSessionSettings 成功过）时不覆盖，避免冲掉后端真值。
    if (sessionsWithSettings[conversationId]) {
      clearPendingPreset(conversationId);
      return;
    }
    try {
      const updated = await patchSessionSettings(config, conversationId, {
        permission_preset: preset,
      });
      // 成功一次后清除，避免后续 send 重复打补丁。
      clearPendingPreset(conversationId);
      sessionsWithSettings = {...sessionsWithSettings, [conversationId]: true};
      if (activeId === conversationId) sessionSettings = updated;
    } catch {
      // 404 / 网络失败静默忽略；保留 pendingPreset，下次 send 后再试。
    }
  }

  async function loadSessionModels(): Promise<void> {
    try {
      const models = await listModels(config);
      sessionModels = models.length ? models : [{id: config.model}];
    } catch {
      // 拉取失败静默降级：picker 只显示全局模型。
      sessionModels = config.model ? [{id: config.model}] : [];
    }
  }

  async function loadSessionSettings(sessionId: string): Promise<void> {
    try {
      const settings = await getSessionSettings(config, sessionId);
      if (activeId === sessionId) {
        sessionSettings = settings;
        sessionsWithSettings = {...sessionsWithSettings, [sessionId]: true};
        // 后端已有 settings，不再用全局预设覆盖。
        clearPendingPreset(sessionId);
      }
    } catch {
      // 拉取失败静默降级：picker 回落到全局模型。
      if (activeId === sessionId) sessionSettings = null;
    }
  }

  async function selectSessionModel(id: string): Promise<void> {
    const sessionId = activeId;
    if (!sessionId) return;
    const prev = sessionSettings;
    sessionSettings = ensureSessionSettings({model: id});
    try {
      const updated = await patchSessionSettings(config, sessionId, {model: id});
      if (activeId === sessionId) sessionSettings = updated;
    } catch {
      if (activeId === sessionId) sessionSettings = prev;
    }
  }

  async function selectSessionPreset(
    strategy: SessionApprovalStrategy,
  ): Promise<void> {
    const sessionId = activeId;
    if (!sessionId) return;
    const prev = sessionSettings;
    sessionSettings = ensureSessionSettings({
      permission_preset: strategy.permission_preset,
      approval_remember: strategy.approval_remember,
    });
    try {
      const updated = await patchSessionSettings(config, sessionId, {
        permission_preset: strategy.permission_preset,
        approval_remember: strategy.approval_remember,
      });
      if (activeId === sessionId) {
        sessionSettings = updated;
        // 用户已在会话头显式选择策略，全局默认预设不再覆盖。
        clearPendingPreset(sessionId);
        // 预设变更不追溯: 此前产生的未决审批仍会弹。主动说明, 避免用户
        // 误以为"切了完全放行还在弹新审批" (2026-10-06 真机反馈)。
        const inbox = await fetchCanonicalApprovals(config, sessionId).catch(() => []);
        if (inbox.length > 0) {
          notice = `该会话还有 ${inbox.length} 个此前产生的未决审批——策略变更不追溯，请先批准或拒绝；之后的请求即按「${strategy.label}」执行。`;
        }
      }
    } catch {
      if (activeId === sessionId) sessionSettings = prev;
    }
  }

  function mapToolCallLifecycle(call: ToolCallDetails): {
    id: string;
    name: string;
    status: 'pending' | 'running' | 'success' | 'error';
    latencyMs?: number;
    error?: string;
    arguments?: unknown;
    result?: unknown;
  } {
    const statusMap: Record<
      ToolCallDetails['status'],
      'pending' | 'running' | 'success' | 'error'
    > = {
      pending: 'pending',
      running: 'running',
      succeeded: 'success',
      failed: 'error',
      cancelled: 'error',
    };
    return {
      id: call.id,
      name: call.name,
      status: statusMap[call.status] ?? 'pending',
      latencyMs: call.durationMs,
      error: call.error,
      arguments: call.args,
      result: call.resultFull ?? call.resultSummary,
    };
  }

  function updateComposerMenu(value: string): void {
    composerMenuOpen = composerFocused && value.trimStart().startsWith('/');
  }

  function handleComposerFocus(): void {
    composerFocused = true;
    updateComposerMenu(draft);
  }

  function handleComposerBlur(): void {
    composerFocused = false;
    composerMenuOpen = false;
  }

  function pickComposerItem(insert: string): void {
    draft = insert;
    composerMenuOpen = false;
    composerTextarea?.focus();
  }

  const errorCode = $derived.by(() => {
    if (lastError && typeof lastError === 'object') {
      // 优先后端错误帧的 machine-readable code (具体原因), 兜底前端分类码.
      const backend = (lastError as {backendCode?: unknown}).backendCode;
      if (typeof backend === 'string' && backend) return backend;
      const code = (lastError as {code?: unknown}).code;
      return typeof code === 'string' ? code : undefined;
    }
    return undefined;
  });

  // 守卫通报分流（§3.2 琥珀卡）：回合被治理面拦停（当前唯一真实信号 =
  // review_rejected，断点见 chat-shell/gateway-events.ts 头注）时在发生处
  // 浮出通报卡，取代红色错误 banner；其余错误仍走 ErrorSolutionBanner。
  const governanceNotice = $derived(classifyGovernanceNotice(lastError));

  type PendingApprovalWithDetails = CanonicalPendingApproval & {
    command_text?: string;
    arguments_summary?: string;
  };

  const approvalCardItem = $derived.by(() => {
    const p = pendingCanonical as PendingApprovalWithDetails | null;
    if (!p) {
      return {
        title: '需要批准的操作',
      } as {
        title: string;
        commandText?: string;
        argumentsSummary?: string;
        reason?: string;
        createdAt?: number;
        sandbox?: string;
        cwd?: string;
        isolation?: string;
      };
    }
    // W1 沙箱卷宗（2026-10-10）：批准前看见墙 —— 徽标 + 冻结 cwd + 隔离态。
    const effective = p.effective_invocation;
    const isolation =
      effective?.filesystem_isolation && effective?.network_isolation
        ? `文件 ${effective.filesystem_isolation} · 网络 ${effective.network_isolation}`
        : undefined;
    return {
      title: '需要批准的操作',
      commandText: typeof p.command_text === 'string' ? p.command_text : undefined,
      argumentsSummary:
        typeof p.arguments_summary === 'string' ? p.arguments_summary : undefined,
      reason: p.governance_reason || undefined,
      createdAt: toEpochMs(p.created_at),
      sandbox: typeof effective?.sandbox === 'string' ? effective.sandbox : undefined,
      cwd: typeof effective?.cwd === 'string' ? effective.cwd : undefined,
      isolation,
    };
  });

  // 深度运行时报告与健康状态
  let healthState = $state<HealthState>('connecting');
  let healthReport = $state<RuntimeHealthReport>({
    overall: 'connecting',
    baseUrl: loadConfig().baseUrl,
    subsystems: [],
    model: loadConfig().model,
  });
  let showRuntimeModal = $state(false);
  let workspacePickerOpen = $state(false);
  let currentWorkspace = $state('');
  let workspaceSuggestions = $state<string[]>([]);
  let showFirstRun = $state(false);
  const FIRST_RUN_DONE_KEY = 'apeireth-first-run-done';

  function completeFirstRun(next: ApeirethConfig): void {
    config = {...config, ...next, apiKey: ''};
    saveConfig(config);
    agentRuntime = createAgentRuntime(config);
    localStorage.setItem(FIRST_RUN_DONE_KEY, '1');
    showFirstRun = false;
    void pushProviderEnvAndRefresh(config);
  }

  function skipFirstRun(): void {
    localStorage.setItem(FIRST_RUN_DONE_KEY, '1');
    showFirstRun = false;
  }
  let showVoiceCall = $state(false);
  let isRefreshingHealth = $state(false);
  let guardStatus = $state<GuardStatus | null>(null);

  async function openVoiceCall() {
    showVoiceCall = true;
    await voiceCallManager.startCall();
  }

  async function handleVoiceMessage(userText: string): Promise<string> {
    if (!userText.trim()) return '';
    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      text: userText,
      time: new Date().toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'}),
      timestamp: Date.now(),
    };
    if (activeConversation) {
      activeConversation.messages.push(userMsg);
      activeConversation.updatedAt = Date.now();
      saveConversations(conversations);
    }
    try {
      const agent = createAgentRuntime(config);
      const resp = await agent.run(
        {
          messages: activeConversation?.messages.map((m) => ({
            role: m.role,
            content: m.text,
          })) || [{role: 'user', content: userText}],
          model: {id: config.model},
          sessionId: activeConversation?.id,
        },
        () => {},
      );
      const assistantMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'assistant',
        text: resp,
        time: new Date().toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'}),
        timestamp: Date.now(),
      };
      if (activeConversation) {
        activeConversation.messages.push(assistantMsg);
        activeConversation.updatedAt = Date.now();
        saveConversations(conversations);
      }
      return resp;
    } catch (e) {
      console.error('Voice turn failed:', e);
      return '抱歉，实时对话连接暂时中断。';
    }
  }

  // Runtime Capability Manifest — gate UI 按钮的依据 (不再 404-probing).
  let capabilities = $state<CapabilityManifest | null>(null);


  // 智能滚动状态管理
  let messagesContainer = $state<HTMLElement | null>(null);
  let isNearBottom = $state(true);
  let showScrollBottomBtn = $state(false);

  // 星尘条（规范 §5.3：memory_recall → 对话流中的「他想起了 N 段记忆」，脱敏，不含原文）。
  // 会话内瞬态：不持久化——星尘是「此刻」的痕迹，刷新即散。按会话 id 分桶。
  // 蛰伏断点（0 装，00-PHILOSOPHY §9 / 契约 §8a）：现役 canonical 总线没有
  // memory_recall 事件（v1 legacy donor 才有；MemoryRecallModule 的 prompt-overlay
  // 召回路径不上 RuntimeEvent 总线）——卡片只接真实 presence 信号，无信号即蛰伏，
  // 禁止假数据演示。presence_state 已落地（§8a）但不含召回计数，不构成此卡数据源。
  interface Stardust {
    id: string;
    found: number;
    keywords: string[];
    ts: number;
  }
  let stardusts = $state<Record<string, Stardust[]>>({});

  // 后端信号驱动的伴随体表现态 (严禁前端造假). Reconciled from master.
  const companionPresentation = $derived.by<CompanionPresentationState>(() => {
    if (pendingApprovals.length > 0) return 'concerned';
    if (isExecutingTool) return 'working';
    if (isReasoning) return 'thinking';
    if (busy) return 'speaking';
    return 'idle';
  });

  const activeConversation = $derived(
    conversations.find((item) => item.id === activeId) || null,
  );

  // 主页置顶行「他」的状态词 —— 全部由前端真实状态推导（流式/审批/健康探测），
  // 无假数据；「他停下了」用金（00-PHILOSOPHY §7：审批卡的金是因为他停下了）。
  const himAttention = $derived(pendingCanonical !== null || pendingApprovals.length > 0);
  const himStatus = $derived(
    himAttention
      ? '他停下了 · 等你签字'
      : busy
        ? '正在输出…'
        : isReasoning
          ? '思考中…'
          : healthState === 'offline'
            ? '离线'
            : '在',
  );

  const activeMessages = $derived(activeConversation?.messages || []);

  // 对话流 = 消息 + 星尘条，按时间戳归并（同刻消息优先于星尘）
  type FlowItem =
    | {kind: 'msg'; id: string; ts: number; message: ChatMessage}
    | {kind: 'dust'; id: string; ts: number; dust: Stardust};

  const flowItems = $derived.by<FlowItem[]>(() => {
    const items: FlowItem[] = activeMessages.map((m) => ({
      kind: 'msg',
      id: m.id,
      ts: m.timestamp ?? 0,
      message: m,
    }));
    const dusts = (activeId ? stardusts[activeId] : undefined) ?? [];
    for (const d of dusts) items.push({kind: 'dust', id: d.id, ts: d.ts, dust: d});
    items.sort((a, b) => a.ts - b.ts || (a.kind === b.kind ? 0 : a.kind === 'msg' ? -1 : 1));
    return items;
  });

  // 他的卡片左缘光晕强度：由真实 presence 状态驱动（规范 §5.3 光晕随 bright 呼吸）；
  // 无数据时取静息微光 —— 金线本身不消失，消失的只是呼吸。
  // 公式与 heuristic_v0 增益收敛在 presence.ts derivePresenceGlow（契约 §8a）。
  const presenceGlow = $derived(derivePresenceGlow($presenceStore.current));

  const healthLabel: Record<HealthState, string> = {
    connecting: '连接中…',
    online: '后端已连接',
    ready: '后端已连接',
    degraded: '降级运行',
    generating: '正在生成…',
    error: '运行异常',
    offline: '后端离线',
  };

  const quickPrompts = [
    '聊聊今天',
    '查看我的记忆',
    '帮我处理一件事',
    '检查系统状态',
  ];

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
      model: config.model,
      personaId: activePersona?.id,
      personaName: activePersona?.name,
    };
    conversations = [conversation, ...conversations];
    activeId = conversation.id;
    markPendingPreset(conversation.id);
    persist();
    return conversation;
  }

  function persist(): void {
    saveConversations(conversations);
  }

  function updateConversation(id: string, patch: Partial<Conversation>): void {
    conversations = conversations.map((item) =>
      item.id === id ? {...item, ...patch, updatedAt: Date.now()} : item,
    );
    persist();
  }

  function updateMessage(id: string, messageId: string, patch: Partial<ChatMessage>): void {
    conversations = conversations.map((item) => {
      if (item.id !== id) return item;
      return {
        ...item,
        updatedAt: Date.now(),
        messages: item.messages.map((m) => (m.id === messageId ? {...m, ...patch} : m)),
      };
    });
    persist();
  }

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
        messages: item.messages.map((m) =>
          m.id === messageId ? {...m, text: m.text + delta} : m,
        ),
      };
    });
    persist();
  }

  /** 按 id 原子拼接推理思考 delta. Reconciled from master. */
  function appendReasoningDelta(conversationId: string, messageId: string, delta: string): void {
    conversations = conversations.map((item) => {
      if (item.id !== conversationId) return item;
      return {
        ...item,
        updatedAt: Date.now(),
        messages: item.messages.map((m) => (m.id === messageId ? {...m, reasoning: (m.reasoning || '') + delta} : m)),
      };
    });
    persist();
  }

  function updateMessageToolCall(
    conversationId: string,
    messageId: string,
    toolCall: ToolCallDetails,
  ): void {
    conversations = conversations.map((item) => {
      if (item.id !== conversationId) return item;
      return {
        ...item,
        updatedAt: Date.now(),
        messages: item.messages.map((m) => {
          if (m.id !== messageId) return m;
          const list = m.toolCalls ? [...m.toolCalls] : [];
          const idx = list.findIndex((t) => t.id === toolCall.id);
          if (idx >= 0) {
            list[idx] = toolCall;
          } else {
            list.push(toolCall);
          }
          return {...m, toolCalls: list};
        }),
      };
    });
    persist();
  }

  /** 工具翻面：tool_completed/tool_failed（或审批决议）把对应 toolCall 从
   *  「运行中/待批」翻成终态。此前 tool-result 事件被吞，工具永远显示「执行中」。 */
  function finishMessageToolCall(
    conversationId: string,
    messageId: string,
    toolCallId: string,
    ok: boolean,
    summary?: string,
  ): void {
    conversations = conversations.map((item) => {
      if (item.id !== conversationId) return item;
      return {
        ...item,
        updatedAt: Date.now(),
        messages: item.messages.map((m) => {
          if (m.id !== messageId || !m.toolCalls) return m;
          const list = m.toolCalls.map((t) =>
            t.id === toolCallId && (t.status === 'pending' || t.status === 'running')
              ? {
                  ...t,
                  status: (ok ? 'succeeded' : 'failed') as ToolCallDetails['status'],
                  endTime: Date.now(),
                  durationMs: t.startTime ? Date.now() - t.startTime : undefined,
                  resultSummary: summary,
                }
              : t,
          );
          return {...m, toolCalls: list};
        }),
      };
    });
    persist();
  }

  /**
   * 他主动开口（legacy `[他说] …` 行，契约 §5.1；initiative/spoke 的完整话术由此送达）：
   * 按规范 §5.3 走与「他的消息」相同的卡片语言进入对话流。
   */
  function appendProactiveMessage(text: string): void {
    const conversation = ensureConversation();
    pushMessage(conversation.id, {
      id: crypto.randomUUID(),
      role: 'assistant',
      text,
      time: new Date().toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'}),
      timestamp: Date.now(),
      proactive: 'initiative',
    });
  }

  // 滚动位置监听与控制
  function handleScroll() {
    if (!messagesContainer) return;
    const {scrollTop, scrollHeight, clientHeight} = messagesContainer;
    const distanceToBottom = scrollHeight - scrollTop - clientHeight;
    isNearBottom = distanceToBottom < 80;
    showScrollBottomBtn = distanceToBottom > 150;
  }

  function scrollToBottom(smooth = false) {
    if (!messagesContainer) return;
    if (smooth) {
      messagesContainer.scrollTo({
        top: messagesContainer.scrollHeight,
        behavior: 'smooth',
      });
    } else {
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    }
    isNearBottom = true;
    showScrollBottomBtn = false;
  }

  async function triggerAutoScroll() {
    if (isNearBottom) {
      await tick();
      scrollToBottom(false);
    }
  }

  async function refreshConnection(): Promise<void> {
    isRefreshingHealth = true;
    try {
      await adoptSupervisorEndpoint();
      const report = await checkHealthDetailed(config.baseUrl, config.apiKey, config.model, config.provider);
      healthReport = report;
      if (!busy) {
        healthState = report.overall;
      }
      // health 之后拉取 capability manifest (runtime version 变化/重连时刷新).
      // 不每次 render 重复 fetch — 仅在 refreshConnection (节拍/手动) 时.
      if (report.overall !== 'offline') {
        const prevVersion = capabilities?.runtime.version;
        const fresh = await fetchCapabilities(config);
        // 仅在 version 变化或首次加载时更新 (避免节拍无谓刷新覆盖).
        if (!capabilities || fresh.runtime.version !== prevVersion || fresh.legacy !== capabilities.legacy) {
          capabilities = fresh;
        }
        // 会话模型选择器数据 + 会话级设置（拉取失败静默降级）。
        if (!sessionModels.length) void loadSessionModels();
        const guard = await fetchGuardStatus(config);
        guardStatus = 'error' in guard ? null : guard;
        // 状态条两个窗口计数（能力门内取数；失败 = null = 诚实「读取失败」而非零）。
        if (capabilityAvailable(capabilities, 'safety.guard.events.read')) {
          const events = await fetchGuardEvents(config, GUARD_EVENTS_WINDOW);
          if (Array.isArray(events)) {
            guardEventCount = events.length;
            guardEventLatestTs = events.reduce((max, e) => Math.max(max, e.timestamp_ms ?? 0), 0);
            if (guardSeenLatestTs < 0) guardSeenLatestTs = guardEventLatestTs; // 首载基线不闪
          } else {
            guardEventCount = null;
          }
        } else {
          guardEventCount = null;
        }
        if (capabilityAvailable(capabilities, 'memory.read')) {
          memoryEpisodeCount = await fetchMemoryEpisodes(config, '', MEMORY_EPISODES_WINDOW)
            .then((list) => list.length)
            .catch(() => null);
        } else {
          memoryEpisodeCount = null;
        }
        if (activeId) {
          if (!sessionSettings) void loadSessionSettings(activeId);
          const inbox = await fetchCanonicalApprovals(config, activeId).catch(() => []);
          pendingApprovals = inbox.map((item) => ({
            id: item.approval_id,
            tool: item.tool_name,
            reason: item.governance_reason,
            status: 'pending' as const,
          }));
          // 主页待签账本同步（当前会话的 inbox 真值；其他会话由 SSE 事件维护）。
          const nextPending = new Set(pendingApprovalSessions);
          if (inbox.length > 0) nextPending.add(activeId);
          else nextPending.delete(activeId);
          if (
            nextPending.size !== pendingApprovalSessions.size ||
            [...nextPending].some((id) => !pendingApprovalSessions.has(id))
          ) {
            pendingApprovalSessions = nextPending;
          }
          // 自愈（2026-09-28 卡死根因类）：弹窗以"后端真值"为准——
          // 若弹窗持有的审批 id 已不在待审批列表（过期/已被处理），
          // 换成后端最新一条；后端说没有待审批就关掉陈旧弹窗。
          if (inbox.length > 0) {
            const currentId = pendingCanonical?.approval_id ?? null;
            const stillPending = inbox.some((item) => item.approval_id === currentId);
            if (!pendingCanonical || !stillPending) {
              pendingCanonical = inbox[0];
            }
          } else if (pendingCanonical) {
            pendingCanonical = null;
          }
        } else {
          pendingApprovals = [];
        }
      } else {
        pendingApprovals = [];
        guardStatus = null;
      }
    } finally {
      isRefreshingHealth = false;
      // 主页账本节拍：健康探测/审批同步后让会话列表对齐一次后端账本。
      homeReloadKey += 1;
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
    isReasoning = false;
    isExecutingTool = false;
    healthState = 'generating';
    error = '';
    lastError = null;
    notice = '';
    // presence 遗留整合点 2：对话请求开始 → thinking（等首字节）；首段文本到达 → speaking
    presenceStore.setChatActive(true);

    const userMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      text,
      time: new Date().toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'}),
      timestamp: Date.now(),
    };
    const assistantMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'assistant',
      text: '',
      time: new Date().toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'}),
      timestamp: Date.now(),
      streaming: true,
      toolCalls: [],
      reasoning: '',
      modelInfo: {id: config.model, provider: 'apeireth'},
    };

    pushMessage(conversationId, userMessage);
    pushMessage(conversationId, assistantMessage);

    if (conversation.messages.length <= 2) {
      updateConversation(conversationId, {title: text.slice(0, 24)});
    }

    await tick();
    scrollToBottom(true);

    try {
      const full = await agentRuntime.run(
        {
          messages: [...history, {role: 'user', content: text}],
          model: {id: config.model, provider: 'apeireth'},
          sessionId: conversationId,
          context: {user: '主人'},
        },
        (event) => {
          if (event.type === 'text-delta') {
            isReasoning = false;
            presenceStore.setSpeaking(true); // 流式输出进行中 = 他在说话
            appendDelta(conversationId, assistantMessage.id, event.text);
            void triggerAutoScroll();
          } else if (event.type === 'reasoning-delta') {
            isReasoning = true;
            appendReasoningDelta(conversationId, assistantMessage.id, event.text);
          } else if (event.type === 'tool-call') {
            isExecutingTool = true;
            updateMessageToolCall(conversationId, assistantMessage.id, event.toolCall);
            void triggerAutoScroll();
          } else if (event.type === 'tool-result') {
            isExecutingTool = false;
            // 状态翻面：completed/failed 事件到达 → 对应工具从「运行中」翻成终态。
            finishMessageToolCall(conversationId, assistantMessage.id, event.toolCallId, event.ok, event.summary);
            void triggerAutoScroll();
          } else if (event.type === 'approval-required') {
            pendingCanonical = event.pending;
          }
        },
      );
      updateMessage(conversationId, assistantMessage.id, {
        text: full || '(空响应)',
        streaming: false,
      });
      void refreshWorkbenchTurn();
    } catch (caught) {
      if (caught instanceof ApprovalRequiredError) {
        pendingCanonical = caught.pending;
        updateMessage(conversationId, assistantMessage.id, {
          streaming: false,
          text: `等待批准：${caught.pending.tool_name}`,
        });
        return;
      }
      const isAborted =
        (caught instanceof Error && caught.name === 'AbortError') ||
        (typeof caught === 'object' && caught !== null && (caught as any).code === 'aborted');
      const msg =
        typeof caught === 'string'
          ? caught
          : caught instanceof Error
            ? caught.message
            : typeof caught === 'object' && caught !== null && 'message' in caught
              ? String((caught as any).message)
              : String(caught);
      if (isAborted) {
        updateMessage(conversationId, assistantMessage.id, {streaming: false, aborted: true});
      } else {
        // "is waiting for approval" = 会话还挂着审批，用户却在发新消息。
        // 说人话 + 自动把审批弹窗拉出来（refreshConnection 的 inbox 自愈）。
        // 这是状态提示不是错误：走 notice，绝不挂"请重试"方案。
        const waiting = /is waiting for approval/.test(msg);
        if (waiting) {
          error = '';
          notice = '本轮还在等待你的审批——先在审批弹窗里点"批准"或"拒绝"，再发新消息。';
        } else {
          error = msg;
        }
        // 保留错误对象供 ErrorSolutionBanner 读取 code/solution；
        // waiting 分支是前端人话提示，不保留原始错误码。
        lastError = waiting ? null : caught;
        // 保留已流出的正文，把失败原因作为附注渲染在下方——
        // 而不是清空文字让"回了话又消失"（2026-09-28 议会拦停实况）。
        const currentText =
          conversations
            .find((item) => item.id === conversationId)
            ?.messages.find((m) => m.id === assistantMessage.id)?.text ?? '';
        updateMessage(conversationId, assistantMessage.id, {
          text: currentText,
          streaming: false,
          error: waiting ? undefined : msg,
        });
        healthState = 'error';
        if (waiting) {
          void refreshConnection();
        }
      }
    } finally {
      busy = false;
      isReasoning = false;
      isExecutingTool = false;
      presenceStore.setSpeaking(false);
      presenceStore.setChatActive(false);
      // 全局权限预设: send 已把 sessionId 送达后端（会话此时已创建），
      // 成功/失败都先静默补一次预设；失败会保留 pendingPreset 待下次 send 重试。
      await applyPendingPreset(conversationId);
      // 生成结束: 恢复真实 health (backend 可能已离线)
      await refreshConnection();
      await tick();
      void triggerAutoScroll();
    }
  }

  async function resolvePending(decision: 'approve' | 'reject'): Promise<void> {
    if (!pendingCanonical || approvalBusy) return;
    approvalBusy = true;
    const conversationId = activeId;
    const pending = pendingCanonical;
    try {
      // 网关 resolve 会同步等待工具执行完成（最长数分钟）；用 15s 超时
      // 解绑 UI，避免弹窗按钮全部锁死（2026-09-28 真机卡死：拒绝/批准/叉
      // 全无反应 = approvalBusy 卡 true）。超时后结果晚到仍自动回填。
      const request = resolveCanonicalApproval(config, pending, decision);
      const result = await Promise.race([
        request,
        new Promise<'timeout'>((resolve) => setTimeout(() => resolve('timeout'), 15_000)),
      ]);
      if (result === 'timeout') {
        pendingCanonical = null;
        // 状态提示，不是错误：走 notice，不挂"请重试"方案。
        error = '';
        notice = '已提交，工具正在执行（最长 5 分钟）；结果稍后自动回填，你可以继续操作。';
        lastError = null;
        void request
          .then((late) => applyResolvedResult(late, conversationId, decision))
          .catch((caught) => {
            // 晚到结果失败也不能静默: 至少留提示, 并定时重拉 inbox 自救
            // (2026-10-06 真机: 这里曾经 .catch(() => {}) 导致永远无后续)。
            notice =
              caught instanceof Error
                ? `审批结果未返回：${caught.message}`
                : '审批结果未返回，请稍后刷新会话。';
            scheduleApprovalRecovery();
          });
        // 后端可能在 15s 后继续执行并 mint 下一个审批/完成结果;
        // 定时重拉 inbox 把下一个审批弹出来, 而不是让用户干等。
        scheduleApprovalRecovery();
        return;
      }
      await applyResolvedResult(result, conversationId, decision);
    } catch (caught) {
      // 失败就关弹窗 + 留错误信息，绝不让同一个弹窗卡死；
      // finally 里的 refreshConnection 会从后端重新拉取真值（若有新审批会再弹）。
      pendingCanonical = null;
      error = describeCaughtSafe(caught);
      lastError = caught;
      scheduleApprovalRecovery();
    } finally {
      approvalBusy = false;
      await refreshConnection();
    }
  }

  /** 审批解析后的自救: 延迟重拉 inbox, 把执行中 mint 出的下一个审批弹出。 */
  function scheduleApprovalRecovery(): void {
    for (const delayMs of [10_000, 30_000]) {
      setTimeout(() => {
        if (approvalBusy || pendingCanonical) return;
        void refreshConnection();
      }, delayMs);
    }
  }

  async function applyResolvedResult(
    result: Awaited<ReturnType<typeof resolveCanonicalApproval>>,
    conversationId: string | null,
    decision: 'approve' | 'reject',
  ): Promise<void> {
    if (result.kind === 'pending') {
      pendingCanonical = result.pending;
      // 同步消息文案, 让"等待批准"始终指向当前待批的工具。
      if (conversationId) {
        const conversation = conversations.find((item) => item.id === conversationId);
        const last = conversation?.messages.filter((m) => m.role === 'assistant').at(-1);
        if (last) {
          updateMessage(conversationId, last.id, {
            text: `等待批准：${result.pending.tool_name ?? '工具'}`,
            streaming: false,
          });
        }
      }
      return;
    }
    pendingCanonical = null;
    if (conversationId) {
      const conversation = conversations.find((item) => item.id === conversationId);
      const last = conversation?.messages.filter((m) => m.role === 'assistant').at(-1);
      if (last) {
        updateMessage(conversationId, last.id, {
          text: result.text || (decision === 'approve' ? '工具执行完成' : '已拒绝该工具调用'),
          streaming: false,
        });
        // 审批决议携带的 canonical events 此前被整包丢弃 → 工具永远「执行中」。
        // 现在把它们应用到最近一条助手消息的 toolCalls（started 补录 / completed·failed 翻面）。
        applyCanonicalEvents(result.events, {
          onToolCall: (toolCall) => updateMessageToolCall(conversationId, last.id, toolCall),
          onToolResult: (toolCallId, ok, summary) =>
            finishMessageToolCall(conversationId, last.id, toolCallId, ok, summary),
        });
        // 拒绝且无 events 回包时，诚实标注待批工具为「已取消」，不停在「执行中」。
        if (decision === 'reject' && !result.events?.length && last.toolCalls?.length) {
          conversations = conversations.map((item) => {
            if (item.id !== conversationId) return item;
            return {
              ...item,
              messages: item.messages.map((m) =>
                m.id !== last.id || !m.toolCalls
                  ? m
                  : {
                      ...m,
                      toolCalls: m.toolCalls.map((t) =>
                        t.status === 'pending' || t.status === 'running'
                          ? {...t, status: 'cancelled' as const, endTime: Date.now()}
                          : t,
                      ),
                    },
              ),
            };
          });
          persist();
        }
      }
    }
    if (wbOpen) void refreshWorkbenchTurn();
  }

  /** X / Esc：仅收起待签文书为 slim 金线，不做业务决策（区别于"拒绝"按钮）。
   *  收起选择按 approval_id 记忆：自愈重拉不打扰，新文书自动重新浮起。 */
  let approvalDockCollapsed = $state(false);
  let lastApprovalDocId = $state('');
  function dismissPendingApproval(): void {
    approvalDockCollapsed = true;
  }

  function describeCaughtSafe(caught: unknown): string {
    if (caught instanceof Error) return caught.message;
    return String(caught);
  }

  /**
   * 打断当前回合（前端近似，gap-plan §5：后端无 POST /v1/turn/interrupt）。
   * 语义 = 「不再听他说」：abort 只切断本地流式读取与输入复位；
   * 后端回合仍跑完，barge_in 的 interrupt 事件广播暂无 HTTP 面可达。
   * 消息侧以 message.aborted 标记此语义（MessageContent 渲染诚实注记）。
   */
  function stop(): void {
    agentRuntime.abort();
  }

  /** 重试一条 assistant 消息: 找到上一条用户消息重新发送 */
  function retryAssistantMessage(messageId: string): void {
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
    // 截断该 assistant 消息及之后的消息
    const filtered = msgs.slice(0, idx);
    updateConversation(activeConversation.id, {messages: filtered});
    if (userText) {
      void send(userText);
    }
  }

  /** 编辑用户消息仅保存 */
  function editUserMessageSave(messageId: string, newText: string): void {
    if (!activeConversation) return;
    updateMessage(activeConversation.id, messageId, {text: newText});
  }

  /** 编辑用户消息并重新生成回答（截断后续回答从新文本开始） */
  function editUserMessageAndRegenerate(messageId: string, newText: string): void {
    if (busy || !activeConversation) return;
    const msgs = activeConversation.messages;
    const idx = msgs.findIndex((m) => m.id === messageId);
    if (idx < 0) return;
    // 截断该用户消息及之后的所有消息
    const filtered = msgs.slice(0, idx);
    updateConversation(activeConversation.id, {messages: filtered});
    void send(newText);
  }

  /** 分支会话：从 messageId 处截取历史，创建新分支会话并跳转 */
  function branchFromMessage(messageId: string): void {
    if (!activeConversation) return;
    const msgs = activeConversation.messages;
    const idx = msgs.findIndex((m) => m.id === messageId);
    if (idx < 0) return;
    const sliced = JSON.parse(JSON.stringify(msgs.slice(0, idx + 1))) as ChatMessage[];
    const now = Date.now();
    const branchConv: Conversation = {
      id: crypto.randomUUID(),
      title: `${activeConversation.title.replace(/\s*\(分支.*\)$/, '')} (分支 ${new Date(now).toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'})})`,
      createdAt: now,
      updatedAt: now,
      messages: sliced,
      scope: 'global',
      model: config.model,
      // 分支继承父会话的工作区 (同一项目上下文).
      workspace: activeConversation?.workspace,
    };
    conversations = [branchConv, ...conversations];
    activeId = branchConv.id;
    markPendingPreset(branchConv.id);
    persist();
  }

  function newConversation(): void {
    const now = Date.now();
    const conversation: Conversation = {
      id: crypto.randomUUID(),
      title: '新对话',
      createdAt: now,
      updatedAt: now,
      messages: [],
      scope: 'global',
      model: config.model,
      personaId: activePersona?.id,
      personaName: activePersona?.name,
    };
    conversations = [conversation, ...conversations];
    activeId = conversation.id;
    markPendingPreset(conversation.id);
    drawerSec = null;
    persist();
    // 每个新会话自动弹出工作区选择 (2026-10-06 用户需求): 不选则跟随全局默认.
    void openWorkspacePicker();
  }

  /** 打开工作区选择器: 刷新候选与当前值. */
  async function openWorkspacePicker(): Promise<void> {
    workspaceSuggestions = (await listWorkspaceSuggestions()) ?? [];
    currentWorkspace = (await getWorkspaceDir()) ?? '';
    workspacePickerOpen = true;
  }

  /** 应用会话级工作区: 持久化到会话 + 侧车快速重根 (supervisor 自动重启). */
  async function applyConversationWorkspace(dir: string): Promise<void> {
    workspacePickerOpen = false;
    const applied = await setWorkspaceDir(dir);
    if (applied && activeId) updateConversation(activeId, {workspace: applied});
    if (applied) {
      currentWorkspace = applied;
      void refreshConnection();
    }
  }

  function openConversation(id: string): void {
    activeId = id;
    drawerSec = null;
    sessionColOpen = false; // 窄窗覆盖层：选中即收（桌面端无视觉效果）
    if (wbOpen) void refreshWorkbenchTurn();
    // 会话级工作区随切换生效: 与侧车当前根不同则重根 (supervisor 快速重启).
    const conv = conversations.find((item) => item.id === id);
    if (conv?.workspace) {
      void (async () => {
        const current = await getWorkspaceDir();
        if ((current ?? '') !== conv.workspace) {
          const applied = await setWorkspaceDir(conv.workspace as string);
          if (applied) {
            currentWorkspace = applied;
            void refreshConnection();
          }
        }
      })();
    }
  }

  function archiveConversation(id: string): void {
    const conv = conversations.find((item) => item.id === id);
    if (conv) updateConversation(id, {archived: !conv.archived});
  }

  // ---------- T0 壳：主页（会话列表）导航 ----------
  /** 回到会话列表主页（微信范式：消息列表是主页，会话可返回）。 */
  function backToList(): void {
    activeId = null;
    ledgerHint = null;
    drawerSec = null;
    homeReloadKey += 1; // 回主页即对齐一次他的账本
  }

  /** 主页置顶行「他」：进入最近的进行中的会话，没有则新建。 */
  function openHim(): void {
    const latest = conversations.filter((c) => !c.archived).sort((a, b) => b.updatedAt - a.updatedAt)[0];
    if (latest) openConversation(latest.id);
    else newConversation();
  }

  /**
   * 主页点开一行会话：
   * - 本地有副本 → 直接进入；
   * - backend-only（本机无消息副本）→ 以同一 session id 建本地壳（之后的发言
   *   后端会续上该会话账本），并在 hero 区诚实标注内容在他的账本里。
   */
  function openHomeSession(item: HomeSessionItem): void {
    const local = conversations.find((c) => c.id === item.id);
    if (!local) {
      const now = Date.now();
      const conv: Conversation = {
        id: item.id,
        title: item.title,
        createdAt: item.lastActiveAt || now,
        updatedAt: item.lastActiveAt || now,
        messages: [],
        scope: 'global',
        model: config.model,
        personaId: activePersona?.id,
        personaName: activePersona?.name,
      };
      conversations = [conv, ...conversations];
      persist();
      if (item.origin === 'backend' && item.messageCount > 0) {
        ledgerHint = {id: item.id, episodeCount: item.messageCount};
      }
      markPendingPreset(conv.id);
    } else {
      ledgerHint = null;
    }
    openConversation(item.id);
  }

  function deleteConversation(id: string): void {
    conversations = conversations.filter((item) => item.id !== id);
    if (activeId === id) activeId = null;
    persist();
  }

  function applyQuickPrompt(promptText: string) {
    draft = promptText;
  }

  function relativeTime(ts: number): string {
    const d = Date.now() - ts;
    if (d < 60_000) return '刚刚';
    if (d < 3_600_000) return `${Math.max(1, Math.round(d / 60_000))} 分钟前`;
    if (d < 86_400_000) return '今天';
    if (d < 172_800_000) return '昨天';
    return `${Math.round(d / 86_400_000)} 天前`;
  }

  const drawerMeta = $derived(drawerSec ? DRAWER_META[drawerSec] : null);
  const modelLetter = $derived(
    (config.model.match(/[A-Za-z]/)?.[0] ?? 'M').toUpperCase(),
  );
  const hdState = $derived(
    busy
      ? '正在输出'
      : healthState === 'offline'
        ? '离线'
        : healthState === 'error'
          ? '异常'
          : healthState === 'degraded'
            ? '降级'
            : '在线',
  );
  const suggestions = $derived(
    conversations
      .filter((c) => !c.archived)
      .slice(0, 3)
      .map((c) => ({id: c.id, title: c.title, src: relativeTime(c.updatedAt)})),
  );
  const ctxUsage = $derived.by(() => {
    const chars = activeMessages.reduce((n, m) => n + (m.text?.length ?? 0), 0);
    const tokens = Math.max(0, Math.round(chars / 4));
    const cap = 200_000;
    const pct = Math.min(100, Math.round((tokens / cap) * 100));
    const circ = 2 * Math.PI * 9;
    return {tokens, cap, pct, dashoffset: circ * (1 - pct / 100), dasharray: circ};
  });
  const filteredModels = $derived(
    availableModels.filter((id) =>
      modelQuery.trim() ? id.toLowerCase().includes(modelQuery.trim().toLowerCase()) : true,
    ),
  );

  function openDrawer(id: DrawerId): void {
    drawerSec = id;
    openPanel = null;
  }

  function closeDrawer(): void {
    drawerSec = null;
  }

  function toggleRail(id: 'chat' | DrawerId): void {
    if (id === 'chat') {
      // 「对话」= 往来主页（00-PHILOSOPHY §3.1：消息列表是主页）
      backToList();
      return;
    }
    if (drawerSec === id) closeDrawer();
    else openDrawer(id);
  }

  function onDrawerAction(): void {
    if (drawerSec === 'history') newConversation();
    else if (drawerSec === 'status') showRuntimeModal = true;
  }

  function toggleWb(force?: boolean): void {
    wbOpen = force === undefined ? !wbOpen : force;
    if (wbOpen) void refreshWorkbenchTurn();
  }

  /** 工作台后端真值（/v1/workbench/turn）：工具终态、代理状态。
   *  打开工作台/切换会话/回合结束/审批决议后拉取，让本地持久化里
   *  卡在「运行中」的历史 toolCall 按后端真值翻面。 */
  async function refreshWorkbenchTurn(): Promise<void> {
    if (!activeId) {
      workbenchTurn = null;
      return;
    }
    const result = await fetchWorkbenchTurn(config, activeId).catch(() => null);
    workbenchTurn = result && 'tools' in result ? result : null;
  }

  function closePanels(): void {
    openPanel = null;
  }

  function togglePanel(id: 'model' | 'ctx'): void {
    openPanel = openPanel === id ? null : id;
    if (openPanel === 'model') void loadModelList();
  }

  async function loadModelList(): Promise<void> {
    modelsLoading = true;
    try {
      const ids = await listModels(config.baseUrl, config.apiKey);
      availableModels = ids.length ? ids : [config.model];
    } catch {
      availableModels = config.model ? [config.model] : [];
    } finally {
      modelsLoading = false;
    }
  }

  function selectModel(id: string): void {
    if (!id || id === config.model) {
      closePanels();
      return;
    }
    config = {...config, model: id};
    saveConfig(config);
    agentRuntime = createAgentRuntime(config);
    closePanels();
  }

  function handleComposerInput(event: Event): void {
    const el = event.currentTarget as HTMLTextAreaElement;
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight}px`;
    updateComposerMenu(el.value);
  }

  // ---- Ctrl+K 命令面板（00-PHILOSOPHY §6 原则 3「一个入口」；gap-plan §4.3 P0）----
  // 注册表驱动：命令是数据，执行闭包在此处注入；筛选/别名/最近优先的纯逻辑在
  // lib/commands/registry.ts（Node 单测覆盖）。最近榜 localStorage 诚实持久化。
  let paletteOpen = $state(false);
  let commandRecentIds = $state<string[]>(loadRecentIds());

  /** 主题切换（与 SettingsView onSave 同路径：persist + applyDocument）。 */
  function setTheme(next: Theme): void {
    if (next === activeTheme) return;
    config = {...config, theme: next};
    saveConfig(config);
    activeTheme = resolveTheme(next, themeQuery);
    applyDocumentTheme(activeTheme);
  }

  const THEME_PINYIN: Record<Theme, string> = {
    'heritage-void': 'yichan',
    essence: 'essence',
    night: 'shenkong',
    day: 'riguang',
    paper: 'zhimian',
    ocean: 'shenhai',
    forest: 'linhai',
  };

  interface PaletteCommand extends CommandItem {
    run: () => void | Promise<void>;
  }

  const paletteCommands = $derived.by<PaletteCommand[]>(() => {
    const approveReason = !capabilities
      ? '运行时能力清单未到达'
      : !capabilityAvailable(capabilities, 'permissions.approval.resolve')
        ? (capabilityUnavailableReason(capabilities, 'permissions.approval.resolve') ??
          '当前运行时不支持审批签批')
        : approvalBusy
          ? '上一笔签批仍在路上'
          : !pendingCanonical
            ? '当前没有等待签字的文书'
            : undefined;
    return [
      // —— 导航：视图切换（纯前端，恒可用）——
      {id: 'nav.chat', title: '打开对话', aliases: ['duihua', 'chat', 'dh'], group: '导航',
        run: () => { closeDrawer(); backToList(); }},
      {id: 'nav.history', title: '打开会话历史', aliases: ['lishi', 'history', 'ls'], group: '导航',
        run: () => openDrawer('history')},
      {id: 'nav.governance', title: '打开治理卷宗', aliases: ['zhili', 'governance', 'gov', 'zl'], group: '导航',
        run: () => openDrawer('governance')},
      {id: 'nav.memory', title: '打开记忆', aliases: ['jiyi', 'memory', 'jy'], group: '导航',
        run: () => openDrawer('memory')},
      {id: 'nav.diary', title: '打开他的日记', aliases: ['riji', 'diary', 'rj'], group: '导航',
        hint: '纸面档案调 · 空态契约页',
        run: () => openDrawer('diary')},
      {id: 'nav.tools', title: '打开工具', aliases: ['gongju', 'tools', 'gj'], group: '导航',
        run: () => openDrawer('tools')},
      {id: 'nav.status', title: '打开状态', aliases: ['zhuangtai', 'status'], group: '导航',
        run: () => openDrawer('status')},
      {id: 'nav.logs', title: '打开日志', aliases: ['rizhi', 'logs', 'rz', 'activity'], group: '导航',
        run: () => openDrawer('logs')},
      {id: 'nav.settings', title: '打开设置', aliases: ['shezhi', 'settings', 'sz'], group: '导航',
        run: () => openDrawer('settings')},
      // —— 主题：现役主题（THEME_CATALOG 目录驱动，2026-09-23 起三套）——
      ...THEME_CATALOG.map((t) => ({
        id: `theme.${t.id}`,
        title: `主题：${t.label}`,
        aliases: ['zhuti', 'theme', t.id, THEME_PINYIN[t.id]],
        group: '主题',
        hint: activeTheme === t.id ? `当前 · ${t.desc}` : t.desc,
        run: () => setTheme(t.id),
      })),
      // —— 动作 ——
      {id: 'act.new', title: '新建会话', aliases: ['xinjian', 'new', 'xj'], group: '动作',
        run: () => newConversation()},
      {id: 'act.reconnect', title: '健康检查重连', aliases: ['chonglian', 'reconnect', 'health', 'cl'], group: '动作',
        run: () => refreshConnection()},
      {id: 'act.approve', title: '批准当前待签文书', aliases: ['pizhun', 'approve', 'pz'], group: '动作',
        hint: pendingCanonical ? `待签：${pendingCanonical.tool_name}` : undefined,
        disabledReason: approveReason,
        run: () => resolvePending('approve')},
      {id: 'act.interrupt', title: '打断当前回合', aliases: ['daduan', 'interrupt', 'stop', 'dd'], group: '动作',
        hint: busy ? '不再听他说完（后端回合仍会跑完）' : undefined,
        disabledReason: busy ? undefined : '当前没有进行中的回合',
        run: () => stop()},
    ];
  });

  function executePaletteCommand(id: string): void {
    const cmd = paletteCommands.find((c) => c.id === id);
    if (!cmd || cmd.disabledReason) return; // 置灰命令不入账不执行（0 装）
    paletteOpen = false;
    commandRecentIds = pushRecentId(commandRecentIds, id);
    saveRecentIds(commandRecentIds);
    void cmd.run();
  }

  // ---- 底部状态条（余光投影；00-PHILOSOPHY §2 / gap-plan §4.3 P1）----
  // 四指标推导纯逻辑在 lib/statusbar.ts（Node 单测覆盖）；此处只接线取数与点击。
  // 窗口口径：guard events limit 50 / memory episodes limit 100，端点不供总数——
  // 满窗显示「N+」，拉取失败 = null（诚实「读取失败」，不是零）。
  const GUARD_EVENTS_WINDOW = 50;
  const MEMORY_EPISODES_WINDOW = 100;
  let guardEventCount = $state<number | null>(null);
  let guardEventLatestTs = $state(0);
  /** 首载基线前为 -1：首载不闪「新事件」；点守卫计数即视为已读。 */
  let guardSeenLatestTs = $state(-1);
  let memoryEpisodeCount = $state<number | null>(null);

  const sbSse = $derived(
    sseIndicator(
      sseSourceOf({
        // 清单未到达/离线 = 未知（null），不是缺席——按连接事实报。
        supported:
          capabilities === null ? null : capabilityAvailable(capabilities, 'activity.sse'),
        connected: $presenceStore.connected,
        simulated: $presenceStore.simulated,
      }),
    ),
  );
  const sbTurn = $derived(turnIndicator(busy));
  const sbGuard = $derived(
    guardIndicator({
      supported: capabilityAvailable(capabilities, 'safety.guard.events.read'),
      count: guardEventCount,
      limit: GUARD_EVENTS_WINDOW,
      hasNew: guardSeenLatestTs >= 0 && guardEventLatestTs > guardSeenLatestTs,
    }),
  );
  const sbMemory = $derived(
    memoryIndicator({
      supported: capabilityAvailable(capabilities, 'memory.read'),
      count: memoryEpisodeCount,
      limit: MEMORY_EPISODES_WINDOW,
    }),
  );

  /** 指令式落治理卷宗某 tab：initialTab 是挂载快照，靠 govTabKey 重挂载生效。 */
  function openGovernanceTab(tab: GovernanceTabId): void {
    govInitialTab = tab;
    govTabKey += 1;
    openDrawer('governance');
  }

  function handleSbGuardClick(): void {
    guardSeenLatestTs = guardEventLatestTs; // 看过即不新
    openGovernanceTab('guard');
  }

  function handleChromeKey(e: KeyboardEvent): void {
    handleModeKeydown(e);
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      paletteOpen = !paletteOpen;
      return;
    }
    // 面板开着时 Esc/方向键归面板管——不穿到抽屉/文书层。
    if (paletteOpen) return;
    if (e.key === 'Escape') {
      closePanels();
      closeDrawer();
      // 待签文书：Escape 仅收起为 slim 金线（与「拒绝」按钮分离，不做业务决策）。
      if (pendingCanonical) dismissPendingApproval();
    }
  }

  function pickSuggestion(item: {id: string; title: string}): void {
    draft = item.title;
  }

  const overallLabel = $derived(
    healthReport.overall === 'online'
      ? '正常'
      : healthReport.overall === 'degraded'
        ? '降级'
        : healthReport.overall === 'offline'
          ? '离线'
          : healthReport.overall === 'error'
            ? '异常'
            : '连接中',
  );

  // 星尘条：蛰伏断点（0 装，00-PHILOSOPHY §9 / 契约 §8a）。
  // 旧实现消费 presenceStore.recentEvents 里的 memory_recall 事件；presence_state 契约
  // （§8a）不含召回计数，v0 总线上不存在任何星尘数据源——此 effect 整体退役。
  // Stardust 接口与 flowItems 归并保留：渲染空列表，不假数据演示；待契约给出
  // 召回信号后在此复活。

  // 待签文书卡（§3.2/§6.4）：新文书（approval_id 变化）自动重新浮起并滚动到发生处；
  // 同一文书保持用户的收起选择（自愈重拉 inbox 不打扰）。
  $effect(() => {
    const id = pendingCanonical?.approval_id ?? '';
    if (id === lastApprovalDocId) return;
    lastApprovalDocId = id;
    approvalDockCollapsed = false;
    if (id) void triggerAutoScroll();
  });

  // 会话切换 / 首次激活时拉取会话级设置（失败静默降级，picker 回落全局模型）。
  $effect(() => {
    const id = activeId;
    if (!id) {
      sessionSettings = null;
      return;
    }
    sessionSettings = null;
    void loadSessionSettings(id);
  });

  /**
   * In packaged desktop mode the BackendSupervisor allocates an ephemeral port
   * at each launch, so a persisted baseUrl points at a port nothing is
   * listening on. The supervisor is authoritative; adopt its endpoint before
   * the first health probe. No-op in web mode.
   */
  async function adoptSupervisorEndpoint(): Promise<void> {
    if (!isDesktop()) return;
    const endpoint = await resolveBackendEndpoint(config.baseUrl);
    if (endpoint && endpoint !== config.baseUrl) {
      config = {...config, baseUrl: endpoint};
      // Rebuild the runtime so in-flight transport targets the live port.
      agentRuntime = createAgentRuntime(config);
      healthReport = {...healthReport, baseUrl: endpoint};
    }
  }

  /**
   * Push the Settings-UI configuration (provider + advanced-capability
   * toggles) into the sidecar's environment. The supervisor restarts the
   * backend exactly once when anything changed, which allocates a fresh
   * port — so after applying we re-adopt the endpoint before probing.
   * No-op in web mode.
   */
  async function pushProviderEnvAndRefresh(cfg: ApeirethConfig): Promise<void> {
    if (!isDesktop()) {
      void refreshConnection();
      return;
    }
    const provider = backendProviderEnvFromConfig(cfg);
    if (!provider) {
      void refreshConnection();
      return;
    }
    const capabilities = capabilityEnvFromConfig(cfg.capabilities);
    const endpoint = await applyBackendConfig(provider, capabilities);
    if (endpoint && endpoint !== cfg.baseUrl) {
      config = {...cfg, baseUrl: endpoint};
      agentRuntime = createAgentRuntime(config);
      healthReport = {...healthReport, baseUrl: endpoint};
    }
    void refreshConnection();
  }

  // presence 频道主订阅（波次 2 壳层整合点）：EventSource + 指数退避 + SIM 纪律。
  // ⑤ 修复：原在 onMount 同步门控，mount 时清单恒 null → 订阅从不启动，
  // presenceStore.connected/simulated 成死值（§5.4 SIM 纪律空转，状态条恒误报
  // 「重连中」——静是默认的反面）。改为清单真正到达且声明 activity.sse 可用时
  // 启动一次（presenceStarted 幂等守卫）；endpoint adopt 先于首次清单拉取完成，
  // 此刻 config.baseUrl 已是真端口。双订阅去重设计见 presence.ts dedupKey。
  let unsubscribePresence: (() => void) | null = null;
  let presenceStarted = false;
  $effect(() => {
    if (
      !presenceStarted &&
      capabilitySupported(capabilities, 'activity.sse') &&
      capabilityAvailable(capabilities, 'activity.sse')
    ) {
      presenceStarted = true;
      unsubscribePresence = subscribePresence(config.baseUrl);
    }
  });

  onMount(() => {
    applyDocumentTheme(activeTheme);
    applyDocumentAccent(resolveAccent(config.accent));
    // 自定义背景（§8 增补④）：开关开着就从 IndexedDB 取图；取不到 = 诚实回落关开关
    if (config.customBg) void syncCustomBg(true);
    // T0 壳（§3.1）：不再自动进入最近会话 —— 第一屏 = 会话列表（谁找我了）。
    if (window.innerWidth < 1180) wbOpen = false;
    // 首启向导：桌面模式下未完成过向导时先引导选 provider / 填 key。
    if (isDesktop() && !localStorage.getItem(FIRST_RUN_DONE_KEY)) {
      showFirstRun = true;
    }
    // Resolve the real endpoint first, then probe: in packaged mode a probe
    // against the stale persisted port would report a false offline state.
    // Pushing the config first makes the sidecar pick up the persisted
    // provider endpoints/models and capability toggles before the first
    // health probe (a config change restarts the backend on a fresh port,
    // which the subsequent endpoint adoption handles).
    void (async () => {
      if (isDesktop()) {
        const env = backendProviderEnvFromConfig(config);
        if (env) {
          await applyBackendConfig(env, capabilityEnvFromConfig(config.capabilities));
        }
      }
      await adoptSupervisorEndpoint();
      void refreshConnection();
    })();

    // 舰内时刻心跳：无 ?hour= 覆写时每 30s 对齐本地时钟（照明过渡由 CSS/rAF 慢性子承担）
    const hourTimer =
      hourOverride === null
        ? window.setInterval(() => {
            timelineHour = localClockHour();
          }, 30000)
        : null;

    // Capability gate for the legacy /v1/apeireth/events subscriber below.
    // subscribeCompanionEvents retries on an exponential backoff loop that
    // never gives up, so gate it on both static support and live availability.
    // ⚠ 诚实标注（⑤ 发现，保持现状待拍板）：mount 时 capabilities 恒为 null
    // （清单异步拉取），此门永假——legacy 伴随体订阅实际从未启动。点亮它会
    // 同时唤醒审批推送/[他说]主动开口等整条行为链，超出状态条任务 scope。
    const eventStreamSupported =
      capabilitySupported(capabilities, 'activity.sse') &&
      capabilityAvailable(capabilities, 'activity.sse');

    // 订阅 SSE 伴随体事件通道 (主动涌现与反思通知). Reconciled from master.
    // 契约 §8a：presence_state 具名帧由 subscribePresence 独立订阅（presence.ts），
    // 不在此通道分流；此处只剩 legacy 文本行（[他说]/测试事件，canonical 上本就罕见）。
    // 波次 2：`[他说]` 行 = 他主动开口 → 进入对话流（规范 §5.3）；
    // 其余 legacy 行（如测试事件）→ 轻量 toast，不进对话。
    const unsubscribeEvents = !eventStreamSupported ? () => {} : subscribeCompanionEvents(config, (event) => {
      // T0 壳（§3.2）：审批事件 = 待签文书实时推入对话，不进 toast 流。
      // 当前会话 → 重拉 inbox（卡在发生处浮起/resolved 后自愈收起）；
      // 其他会话 → 只更新主页列表的金色待签标。
      if (event.kind === 'approval_required' || event.kind === 'approval_resolved') {
        const info = parseApprovalEventPayload(event.payload);
        if (info) {
          pendingApprovalSessions = applyApprovalEventToPending(
            pendingApprovalSessions,
            info,
            event.kind,
          );
        }
        if (!info || !info.session || info.session === activeId) {
          void refreshConnection();
        } else {
          homeReloadKey += 1;
        }
        return;
      }
      const text = event.text.trim();
      if (text.startsWith('[他说]')) {
        const said = text.slice('[他说]'.length).trim();
        if (said) {
          appendProactiveMessage(said);
          void triggerAutoScroll();
        }
        return;
      }
      legacyToast = text;
      window.setTimeout(() => {
        if (legacyToast === text) legacyToast = '';
      }, 12000);
    });

    // 后台健康轮询与审批请求同步 (真实 HTTP /health + capability manifest).
    const timer = window.setInterval(() => {
      void refreshConnection();
    }, 15000);

    return () => {
      window.clearInterval(timer);
      if (hourTimer !== null) window.clearInterval(hourTimer);
      unsubscribeEvents();
      unsubscribePresence?.();
    };
  });
</script>

<svelte:window onkeydown={handleChromeKey} />

<div
  class="app-root"
  class:busy
  class:theme-essence={isEssenceTheme}
  class:theme-heritage={isHeritageTheme}
  class:theme-day={activeTheme === 'day'}
  class:theme-paper={activeTheme === 'paper'}
  class:theme-ocean={activeTheme === 'ocean'}
  class:theme-forest={activeTheme === 'forest'}
  class:custom-bg={customBgUrl !== null}
  class:mode-focus={mode === 'focus'}
  class:mode-engineering={mode === 'engineering'}
  class:intro-playing={introPlaying}
>
  <!-- 静态背景层（规范 §8 增补）：essence/heritage-void 主题各带默认图，
       自定义上传（customBgUrl）以 inline 背景覆盖主题默认图 -->
  <div
    class="static-bg"
    style:background-image={customBgUrl ? `url(${customBgUrl})` : undefined}
    aria-hidden="true"
  ></div>
  <div class="scene-underlay">
    <SceneLayer
      presence={$presenceStore.current}
      hour={timelineHour}
      interactive={!drawerSec && !showRuntimeModal && !openPanel}
      cameraIndex={sceneCamera}
      paused={isStaticScene}
      onBlackholeClick={handleBlackholeClick}
    />
    <div class="planet-xfade" class:layer-off={mode === 'focus'}>
      <PlanetLayer hour={timelineHour} />
    </div>
    <div class="layer-xfade" class:layer-off={mode !== 'companion'}>
      <BridgeLayer hour={timelineHour} />
    </div>
    <div class="layer-xfade" class:layer-off={mode !== 'engineering'}>
      <DeepCabinLayer hour={timelineHour} />
    </div>
  </div>

  <div id="presence" aria-hidden="true"></div>
  <div id="vignette" aria-hidden="true"></div>

  <div class="shell">
    <nav class="rail" aria-label="主导航">
      <div class="rail-brand" title="Apeireth">燧</div>
      <div class="rail-nav">
        <button
          class="rail-btn"
          class:active={!drawerSec}
          onclick={() => toggleRail('chat')}
          title="当前对话"
        >
          <MessageCircleMore size={17} class="shell-icon" />
          <span class="rail-label">对话</span>
        </button>
        <button
          class="rail-btn"
          class:active={drawerSec === 'history'}
          onclick={() => toggleRail('history')}
          title="历史"
        >
          <History size={17} class="shell-icon" />
          <span class="rail-label">历史</span>
        </button>
        <button
          class="rail-btn"
          class:active={drawerSec === 'memory'}
          onclick={() => toggleRail('memory')}
          title="记忆卷宗（纸面档案调）"
        >
          <Layers3 size={17} class="shell-icon" />
          <span class="rail-label">记忆</span>
        </button>
        <button
          class="rail-btn"
          class:active={drawerSec === 'diary'}
          onclick={() => toggleRail('diary')}
          title="他的日记（纸面档案调 · 空态契约）"
        >
          <BookOpen size={17} class="shell-icon" />
          <span class="rail-label">日记</span>
        </button>
        <button
          class="rail-btn"
          class:active={drawerSec === 'tools'}
          onclick={() => toggleRail('tools')}
          title="工具管理"
        >
          <Wrench size={17} class="shell-icon" />
          <span class="rail-label">工具</span>
        </button>
        <button
          class="rail-btn"
          class:active={drawerSec === 'governance'}
          onclick={() => toggleRail('governance')}
          title="治理卷宗——审批 / 授权 / 守卫 / 审计的账"
        >
          <Landmark size={17} class="shell-icon" />
          <span class="rail-label">治理</span>
        </button>
      </div>
      <div class="rail-foot">
        <div class="rail-sep"></div>
        <button
          class="rail-btn"
          class:active={drawerSec === 'status'}
          onclick={() => toggleRail('status')}
          title="系统状态"
        >
          <Activity size={17} class="shell-icon" />
          <span class="rail-label">状态</span>
        </button>
        <button
          class="rail-btn"
          class:active={drawerSec === 'logs'}
          onclick={() => toggleRail('logs')}
          title="日志"
        >
          <ScrollText size={17} class="shell-icon" />
          <span class="rail-label">日志</span>
        </button>
        <button
          class="rail-btn"
          class:active={drawerSec === 'settings'}
          onclick={() => toggleRail('settings')}
          title="设置"
        >
          <Settings size={17} class="shell-icon" />
          <span class="rail-label">设置</span>
        </button>
      </div>
    </nav>

    <div
      id="drawerScrim"
      class:on={drawerSec !== null}
      role="button"
      tabindex="-1"
      aria-label="关闭侧边面板"
      onclick={closeDrawer}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') closeDrawer();
      }}
    ></div>
    <div
      id="scrim"
      class:show={openPanel !== null}
      role="button"
      tabindex="-1"
      aria-label="关闭弹出层"
      onclick={closePanels}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') closePanels();
      }}
    ></div>

    <aside class="drawer" class:open={drawerSec !== null} aria-label="侧边面板">
      {#if drawerMeta}
        <div class="drawer-head">
          <div>
            <p class="eyebrow">{drawerMeta.eyebrow}</p>
            <h2>{drawerMeta.title}</h2>
            <p class="sub">{drawerMeta.sub}</p>
          </div>
          <div class="drawer-actions">
            {#if drawerMeta.action}
              <button class="quiet-btn" onclick={onDrawerAction}>{drawerMeta.action}</button>
            {/if}
            <button class="mini" onclick={closeDrawer} aria-label="关闭">
              <X size={15} class="shell-icon-sm" />
            </button>
          </div>
        </div>
      {/if}
      <div class="drawer-body" class:embed={drawerSec !== 'status' && drawerSec !== null}>
        {#if drawerSec === 'history'}
          <ConversationsView
            {conversations}
            activeId={activeId || ''}
            {config}
            {capabilities}
            onOpen={openConversation}
            onCreate={newConversation}
            onArchive={archiveConversation}
            onDelete={deleteConversation}
            onRename={(id, title) => updateConversation(id, {title})}
            onPin={(id) => {
              const conv = conversations.find((item) => item.id === id);
              if (conv) updateConversation(id, {pinned: !conv.pinned});
            }}
          />
        {:else if drawerSec === 'memory'}
          <MemoryView {config} {capabilities} />
        {:else if drawerSec === 'diary'}
          <DiaryView />
        {:else if drawerSec === 'tools'}
          <ToolsView {config} {capabilities} />
        {:else if drawerSec === 'governance'}
          {#key govTabKey}
            <GovernanceView
              {config}
              {capabilities}
              initialTab={govInitialTab}
              onOpenChat={() => {
                closeDrawer();
                backToList();
              }}
            />
          {/key}
        {:else if drawerSec === 'logs'}
          <ActivityView {config} {capabilities} />
        {:else if drawerSec === 'settings'}
          <SettingsView
            {config}
            onSave={(newCfg) => {
              const customBgToggled = (newCfg.customBg ?? false) !== (config.customBg ?? false);
              config = newCfg;
              saveConfig(newCfg);
              agentRuntime = createAgentRuntime(newCfg);
              const nextTheme = resolveTheme(newCfg.theme, themeQuery);
              activeTheme = nextTheme;
              applyDocumentTheme(nextTheme);
              applyDocumentAccent(resolveAccent(newCfg.accent));
              // 自定义背景开关变化（§8 增补④）：从 IndexedDB 取图或回落
              if (customBgToggled) void syncCustomBg(newCfg.customBg === true);
              // Provider changes must reach the sidecar environment; the
              // push re-adopts the endpoint (a restart allocates a new port)
              // and then re-probes.
              void pushProviderEnvAndRefresh(newCfg);
            }}
            onClearLocalData={() => {
              conversations = [];
              activeId = null;
              persist();
            }}
          />
        {:else if drawerSec === 'status'}
          <div class="stats">
            <div class="stat">
              <div
                class="num"
                class:ok={overallLabel === '正常'}
                class:bad={overallLabel === '离线' || overallLabel === '异常'}
              >
                {overallLabel}
              </div>
              <div class="lbl">总体状态</div>
            </div>
            <div class="stat">
              <div class="num">
                {healthReport.latencyMs ?? '—'}{#if healthReport.latencyMs}<small> ms</small>{/if}
              </div>
              <div class="lbl">总延迟</div>
            </div>
            <div class="stat">
              <div class="num">{config.model}</div>
              <div class="lbl">活动模型</div>
            </div>
            <div class="stat">
              <div class="num">{healthReport.subsystems.length}</div>
              <div class="lbl">子系统</div>
            </div>
          </div>
          <h3 class="sec-title">子系统</h3>
          {#each healthReport.subsystems as sub (sub.key)}
            <div class="rowline">
              <span
                class="dot-st"
                class:ok={sub.status === 'ok'}
                class:warn={sub.status === 'degraded'}
                class:bad={sub.status === 'offline'}
              ></span>
              <span class="k">{sub.name}</span>
              <code>{sub.endpoint}</code>
              <span class="v">{sub.latencyMs != null ? `${sub.latencyMs} ms` : sub.detail || sub.status}</span>
            </div>
          {:else}
            <p class="wb-empty">尚未完成探测。点「深度诊断」查看详情。</p>
          {/each}
          <h3 class="sec-title">行为安全</h3>
          {#if guardStatus}
            <div class="rowline">
              <span class="dot-st" class:ok={guardStatus.enabled} class:bad={!guardStatus.enabled}></span>
              <span class="k">Guard</span>
              <code>{guardStatus.ml_classifier_available ? (guardStatus.ml_model_version || '模型已启用') : '确定性模式'}</code>
              <span class="v">评估 {guardStatus.total_evaluations} · 拒绝 {guardStatus.total_denied} · 待审批 {guardStatus.total_approval_required}</span>
            </div>
            <div class="rowline">
              <span class="dot-st" class:ok={guardStatus.dataset_recording_enabled}></span>
              <span class="k">数据集</span>
              <code>{guardStatus.dataset_recording_enabled ? 'recording' : 'off'}</code>
              <span class="v">分类与运行结果按 action_id 关联</span>
            </div>
          {:else}
            <p class="wb-empty">Guard 状态暂不可用。</p>
          {/if}
        {/if}
      </div>
    </aside>

    <div class="main">
      <button
        class="wb-toggle"
        class:on={wbOpen}
        title="工作台"
        aria-label="工作台"
        aria-pressed={wbOpen}
        onclick={() => toggleWb()}
      >
        <PanelRight size={14} class="shell-icon-sm" />
        <span>工作台</span>
      </button>
      <!-- 三栏主从骨架（2026-09-22 主人拍板，微信电脑版/QQ 桌面范式）：
           图标轨（已有）｜会话列表栏（常驻 ~300px，点行不跳转）｜聊天区。
           未选中会话时右栏 = 个性化背景显影区（§8 增补①），不再是列表页跳转。 -->
      <div class="chat-columns">
        <aside class="session-col" class:open={sessionColOpen} aria-label="往来列表">
          <SessionListHome
            {conversations}
            {config}
            {capabilities}
            {pendingApprovalSessions}
            {activeId}
            defaultWorkspace={currentWorkspace || null}
            himName={activePersona?.name || '他'}
            {himStatus}
            {himAttention}
            reloadKey={homeReloadKey}
            onOpen={openHomeSession}
            onOpenHim={openHim}
            onNew={newConversation}
            onRename={(id, title) => updateConversation(id, {title})}
            onTogglePin={(id) => {
              const conv = conversations.find((item) => item.id === id);
              if (conv) updateConversation(id, {pinned: !conv.pinned});
            }}
            onToggleArchive={archiveConversation}
            onDelete={deleteConversation}
          />
        </aside>
        <div class="chat-area">
          <!-- 窄窗折叠（≤980px，诚实断点）时的列表开关；桌面恒隐 -->
          <button
            class="col-toggle"
            onclick={() => (sessionColOpen = !sessionColOpen)}
            aria-expanded={sessionColOpen}
            aria-label="往来列表"
            title="往来列表"
          >
            <MessageCircleMore size={13} />
            往来
          </button>
      <div
        class="scroll"
        id="chatScroll"
        bind:this={messagesContainer}
        onscroll={handleScroll}
        style:--presence-glow={presenceGlow.toFixed(3)}
      >
        {#if activeId === null}
          <!-- 未选中态 = 个性化背景显影区：内容随用户设置（主题默认图/自定义上传/实时场景），
               只放一句指引与背景署名，背景本身即是内容 -->
          <section class="bg-reveal" aria-label="个性化背景显影区">
            <p class="bg-hint">从左边选一段往来，或开始新的一段。</p>
            <p class="bg-caption">背景 · {bgCaption}</p>
          </section>
        {:else if !flowItems.length}
          <section class="home col">
            <svg class="ember" viewBox="0 0 56 56" aria-hidden="true">
              <circle class="halo" cx="28" cy="30" r="19"></circle>
              <circle class="core" cx="28" cy="30" r="7"></circle>
              <path class="halo" d="M28 6v9"></path>
            </svg>
            <h1 class="ask">今天想干些什么？</h1>
            <p class="lede">与 Apeireth 交流你的想法、创意与工作。</p>
            {#if ledgerHint && ledgerHint.id === activeId}
              <!-- backend-only 会话：本机无消息副本的诚实标注（0 装空态契约） -->
              <p class="ledger-hint" role="status">
                这段会话在他的账本里有 {ledgerHint.episodeCount} 条记录；本机没有消息副本——从下一句开始，他会接着同一本账继续。
              </p>
            {/if}
            <div class="sugs">
              {#if suggestions.length}
                {#each suggestions as item (item.id)}
                  <button class="sug" onclick={() => pickSuggestion(item)}>
                    <Sparkles size={13} class="shell-icon-sm" />
                    <span>{item.title}</span>
                    <span class="src">{item.src}</span>
                  </button>
                {/each}
              {:else}
                {#each quickPrompts as prompt}
                  <button class="sug" onclick={() => applyQuickPrompt(prompt)}>
                    <Plus size={13} class="shell-icon-sm" />
                    <span>{prompt}</span>
                    <span class="src">开始</span>
                  </button>
                {/each}
              {/if}
            </div>
            <p class="sug-note">若干个性化条目 · 由近期记忆与对话生成</p>
            <div class="orbar">或者</div>
            <div class="calls">
              <button
                class="call"
                onclick={openVoiceCall}
                title={capabilityAvailable(capabilities, 'voice.duplex') ? '开始语音通话' : '全双工语音服务尚未组装 (not_assembled)'}
              >
                <PhoneCall size={13} />
                语音通话
                {#if !capabilityAvailable(capabilities, 'voice.duplex')}
                  <span style="opacity: 0.6; font-size: 11px;">(未组装)</span>
                {/if}
              </button>
            </div>
          </section>
        {:else}
          <section class="col">
            <div class="chat-head">
              <div class="chat-head-main">
                <!-- 三栏主从下取消「‹ 往来」返回键：列表栏常驻，不存在返回；
                     长标题横排单行省略（flex:1 + min-width:0 收缩链） -->
                <h2 class="chat-title">{activeConversation?.title || '新对话'}</h2>
              </div>
              <!-- 权限档位 / 新对话 / 会话模型已移至对话栏位（2026-10-11 主人反馈批：
                   头部随滚动消失，输入栏位常驻——布局参照 Kimi Desktop） -->
              <!-- 状态行：头部第二行，独占整宽（打回修复②轮） -->
              <div class="statusline">
                <div class="persona-menu">
                  <button
                    class="persona-trigger"
                    onclick={() => (personaMenuOpen = !personaMenuOpen)}
                    title="切换伙伴身份"
                    aria-label="切换伙伴身份"
                    aria-expanded={personaMenuOpen}
                  >
                    <span>{activePersona?.name || '伙伴'}</span>
                    <ChevronDown size={12} />
                  </button>
                  {#if personaMenuOpen}
                    <div class="persona-pop" role="menu">
                      {#each personaList as p (p.id)}
                        <button
                          class="persona-item"
                          class:active={p.id === activePersona?.id}
                          role="menuitem"
                          onclick={() => setActivePersona(p.id)}
                        >
                          <span class="persona-item-name">{p.name}</span>
                          {#if p.model}
                            <span class="persona-item-model">{p.model}</span>
                          {/if}
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
                <span class="mono-note" style="opacity:.4">·</span>
                <button class="mono-note live" onclick={() => (showRuntimeModal = true)}>{hdState}</button>
                {#if $presenceStore.simulated}
                  <span class="sim-badge" title="presence 频道断连：当前为本机中性默认值">SIM</span>
                {/if}
              </div>
            </div>
            <div class="thread">
              {#each flowItems as item (item.id)}
                {#if item.kind === 'dust'}
                  <div class="stardust" role="status">
                    <span class="stardust-line"></span>
                    <span class="stardust-text">他想起了 {item.dust.found} 段记忆</span>
                    {#if item.dust.keywords.length}
                      <span class="stardust-keys">{item.dust.keywords.slice(0, 4).join(' · ')}</span>
                    {/if}
                    <span class="stardust-line"></span>
                  </div>
                {:else if item.message.role === 'user'}
                  <div class="row user">
                    <div class="user-card">
                      <MessageContent
                        message={item.message}
                        onRetry={(msgId) => retryAssistantMessage(msgId)}
                        onEditSave={(msgId, newText) => editUserMessageSave(msgId, newText)}
                        onEditAndRegenerate={(msgId, newText) => editUserMessageAndRegenerate(msgId, newText)}
                        onBranch={(msgId) => branchFromMessage(msgId)}
                      />
                    </div>
                  </div>
                {:else}
                  <div class="row">
                    <div class="ai-card">
                      {#if item.message.toolCalls?.length}
                        <div class="tool-lifecycle-list">
                          {#each item.message.toolCalls as toolCall (toolCall.id)}
                            <ToolCallLifecycleCard call={mapToolCallLifecycle(toolCall)} />
                          {/each}
                        </div>
                      {/if}
                      <MessageContent
                        message={item.message}
                        onRetry={(msgId) => retryAssistantMessage(msgId)}
                        onEditSave={(msgId, newText) => editUserMessageSave(msgId, newText)}
                        onEditAndRegenerate={(msgId, newText) => editUserMessageAndRegenerate(msgId, newText)}
                        onBranch={(msgId) => branchFromMessage(msgId)}
                      />
                    </div>
                  </div>
                {/if}
              {/each}
              {#if pendingCanonical && pendingCanonical.session === activeId}
                <!-- 待签文书（§3.2/§6.4）：对话内浮起，在发生处批准/拒绝，不跳窗。
                     按 session 归属渲染：切换会话后文书只留在它自己的会话里。 -->
                <PendingDocumentDock
                  approvalId={pendingCanonical.approval_id}
                  item={approvalCardItem}
                  busy={approvalBusy}
                  collapsed={approvalDockCollapsed}
                  onExpand={() => (approvalDockCollapsed = false)}
                  onAllow={() => void resolvePending('approve')}
                  onReject={() => void resolvePending('reject')}
                  onDismiss={dismissPendingApproval}
                />
              {/if}
              {#if governanceNotice}
                <!-- 守卫通报卡（琥珀，§3.2）：治理面拦停在发生处呈现，不跳窗 -->
                <GuardNoticeCard
                  title={governanceNotice.title}
                  message={error}
                  solution={describeError(lastError).solution}
                  onClose={() => {
                    error = '';
                    lastError = null;
                  }}
                />
              {:else if error}
                <ErrorSolutionBanner
                  code={errorCode}
                  message={error}
                  solution={describeError(lastError).solution}
                  onClose={() => {
                    error = '';
                    lastError = null;
                  }}
                />
              {/if}
              {#if notice}
                <div class="notice-banner" role="status">
                  <Info size={14} />
                  <span class="notice-text">{notice}</span>
                  <button
                    class="notice-close"
                    onclick={() => (notice = '')}
                    aria-label="关闭提示"
                  >
                    <X size={13} />
                  </button>
                </div>
              {/if}
            </div>
          </section>
        {/if}
      </div>

      {#if showScrollBottomBtn}
        <button class="scroll-bottom-btn" onclick={() => scrollToBottom(true)} aria-label="回到底部">
          <ChevronDown size={16} />
          <span>回到底部</span>
        </button>
      {/if}

      <div class="dock">
        <div class="col dock-col">
          <div class="composer-row">
            <div class="composer">
              <div class="editor">
                <button class="round-btn" title="新对话" onclick={newConversation} aria-label="新对话">
                  <Plus size={16} />
                </button>
                <!-- 权限档位芯片（Kimi Desktop 式输入栏左侧常驻；弹层向上翻） -->
                <div class="composer-preset">
                  <button
                    class="composer-preset-trigger"
                    class:open={composerPresetOpen}
                    onclick={() => (composerPresetOpen = !composerPresetOpen)}
                    disabled={busy}
                    title={`权限档位：${activePresetFull?.title ?? ''}（点击切换）`}
                    aria-haspopup="menu"
                    aria-expanded={composerPresetOpen}
                  >
                    <ShieldCheck size={13} class="shell-icon-sm" />
                    <span>{activePresetShortLabel}</span>
                    <ChevronDown size={12} />
                  </button>
                  {#if composerPresetOpen}
                    <div class="composer-preset-scrim" onclick={() => (composerPresetOpen = false)} aria-hidden="true"></div>
                    <div class="composer-preset-pop" role="menu" aria-label="权限档位">
                      {#each SESSION_PRESETS as preset (preset.id)}
                        <button
                          class="composer-preset-item"
                          class:active={activeApprovalStrategyId === preset.id}
                          role="menuitemcheckbox"
                          aria-checked={activeApprovalStrategyId === preset.id}
                          onclick={() => void pickComposerPreset(preset.id)}
                        >
                          <span class="composer-preset-label">{preset.label}</span>
                          <span class="composer-preset-title">{preset.title}</span>
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
                <textarea
                  bind:value={draft}
                  bind:this={composerTextarea}
                  rows="1"
                  placeholder="与 Apeireth 交流……"
                  disabled={busy}
                  oninput={handleComposerInput}
                  onfocus={handleComposerFocus}
                  onblur={handleComposerBlur}
                  onkeydown={(event) => {
                    if (event.key === 'Enter' && !event.shiftKey && !composerMenuOpen) {
                      event.preventDefault();
                      void send();
                    }
                  }}
                ></textarea>
              </div>
              <!-- 斜杠菜单：菜单项 mousedown 阻止默认，避免 textarea 失焦抢跑。 -->
              <div role="presentation" onmousedown={(e) => e.preventDefault()}>
                <ComposerMenu
                  open={composerMenuOpen}
                  query={draft.slice(1)}
                  slashCommands={SLASH_COMMANDS}
                  files={[]}
                  onPick={pickComposerItem}
                  onClose={() => (composerMenuOpen = false)}
                />
              </div>
            </div>

            <div class="composer-side">
              <!-- 会话模型切换（2026-10-11 从会话头迁入对话栏位；弹层向上翻） -->
              <div class="composer-session-model">
                <SessionModelPicker
                  models={sessionModels}
                  value={currentSessionModel}
                  onSelect={(id) => void selectSessionModel(id)}
                  disabled={busy}
                  up
                />
              </div>
              <div class="composer-caps" aria-label="模型与上下文">
                <div class="panel" class:show={openPanel === 'ctx'} id="panel-ctx" role="dialog" aria-label="上下文窗口">
                  <h2>上下文窗口</h2>
                  <div class="bar"><i style:width={`${ctxUsage.pct}%`}></i></div>
                  <div class="bar-head">
                    <b>{ctxUsage.pct}%</b>
                    <span>{ctxUsage.tokens} / {ctxUsage.cap}</span>
                  </div>
                  <div class="kv"><span class="dot"></span>用户消息<span class="v">{ctxUsage.tokens}</span></div>
                  <h2>本轮</h2>
                  <div class="kv"><span class="dot"></span>消息数<span class="v">{activeMessages.length}</span></div>
                  <div class="kv"><span class="dot"></span>模型<span class="v">{config.model}</span></div>
                </div>

                <div class="panel" class:show={openPanel === 'model'} id="panel-model" role="dialog" aria-label="模型选择器">
                  <h2>当前模型</h2>
                  <div class="cur">
                    <span class="provider">{modelLetter}</span>
                    <span>{config.model}</span>
                  </div>
                  <div class="search" style="margin:14px 0 4px">
                    <Search size={13} class="shell-icon-sm" />
                    <input placeholder="切换模型" bind:value={modelQuery} />
                  </div>
                  {#if modelsLoading}
                    <p class="wb-empty">正在拉取模型列表…</p>
                  {:else if filteredModels.length}
                    {#each filteredModels as id (id)}
                      <button
                        class="model"
                        aria-current={id === config.model ? 'true' : undefined}
                        onclick={() => selectModel(id)}
                      >
                        {id}
                      </button>
                    {/each}
                  {:else}
                    <p class="wb-empty">暂无可用模型。可在设置中配置提供商。</p>
                  {/if}
                </div>

                <button
                  class="cap-btn pill-model"
                  aria-expanded={openPanel === 'model'}
                  onclick={() => togglePanel('model')}
                  title={`模型：${config.model}`}
                  aria-label={`切换模型：${config.model}`}
                >
                  <span class="provider">{modelLetter}</span>
                </button>
                <span class="cap-sep" aria-hidden="true"></span>
                <button
                  class="cap-btn pill-ctx"
                  aria-expanded={openPanel === 'ctx'}
                  title={`上下文窗口 ${ctxUsage.pct}%`}
                  aria-label={`上下文窗口 ${ctxUsage.pct}%`}
                  onclick={() => togglePanel('ctx')}
                >
                  <svg class="ctx-ring" viewBox="0 0 24 24" aria-hidden="true">
                    <circle class="bg" cx="12" cy="12" r="9"></circle>
                    <circle
                      class="fg"
                      cx="12"
                      cy="12"
                      r="9"
                      stroke-dasharray={ctxUsage.dasharray}
                      stroke-dashoffset={ctxUsage.dashoffset}
                    ></circle>
                  </svg>
                </button>
              </div>

              <div class="composer-send">
                {#if busy}
                  <!-- 打断按钮（任务 B，gap-plan §5 缺口的前端近似）：
                       后端无 POST /v1/turn/interrupt，此钮只切断本地收听
                       （agentRuntime.abort → 关闭流式读取、复位输入态）；
                       后端回合仍跑完，下一条消息开新轮。显隐信号 = busy
                       （本地流真值），不取 SSE turn_*：那是网关级事件，
                       本地打断后后端仍在跑，挂它会违背按钮的真实语义。
                       语言克制、非金色（金=他；打断是主人的动作）。 -->
                  <button
                    class="send stop"
                    onclick={stop}
                    aria-label="打断当前回合"
                    title="不再听他说完——后端回合仍会跑完（打断的是收听，不是他）"
                  >
                    <Square size={14} />
                  </button>
                {:else}
                  <button
                    class="send"
                    onclick={() => send()}
                    disabled={!draft.trim() || healthState === 'offline'}
                    aria-label="发送"
                  >
                    <ArrowUp size={16} />
                  </button>
                {/if}
              </div>
            </div>
          </div>
          <p class="hint">ENTER 发送 · SHIFT+ENTER 换行</p>
        </div>
      </div>
        </div>
      </div>

      <!-- 余光投影（00-PHILOSOPHY §2 第四个投影 / gap-plan §4.3 P1）：
           四指标一行通栏，静是默认，异常才挣色；入口不是面板。 -->
      <StatusBar
        sse={sbSse}
        turn={sbTurn}
        guard={sbGuard}
        memory={sbMemory}
        onSseClick={() => void refreshConnection()}
        onTurnClick={() => {
          if (busy) stop();
        }}
        onGuardClick={handleSbGuardClick}
        onMemoryClick={() => openDrawer('memory')}
      />
    </div>

    <Workbench
      conversation={activeConversation}
      {busy}
      closed={!wbOpen}
      turn={workbenchTurn}
      onClose={() => toggleWb(false)}
    />
  </div>

  {#if !isStaticScene}
    <nav class="mode-switch" aria-label="模式切换">
      {#each modes as item (item.id)}
        <button
          class="mode-btn"
          class:active={mode === item.id}
          onclick={() => setMode(item.id)}
          title={item.label}
          aria-label={item.label}
          aria-current={mode === item.id ? 'page' : undefined}
        >
          <item.icon size={16} />
        </button>
      {/each}
    </nav>
  {/if}

  <button class="focus-exit" onclick={() => setMode('companion')}>返回舰桥</button>

  {#if legacyToast}
    <div class="legacy-toast" role="status">
      <Sparkles size={12} />
      <span>{legacyToast}</span>
    </div>
  {/if}

  {#if introPlaying}
    <IntroLayer onComplete={handleIntroComplete} />
  {/if}
</div>

<VoiceCallModal
  isOpen={showVoiceCall}
  padState={{
    pleasure: (($presenceStore.current?.p ?? 0.4) + 1) / 2,
    arousal: (($presenceStore.current?.a ?? -0.2) + 1) / 2,
    dominance: (($presenceStore.current?.d ?? 0.2) + 1) / 2,
  }}
  onClose={() => (showVoiceCall = false)}
  onSendMessage={handleVoiceMessage}
/>

<RuntimeModal
  open={showRuntimeModal}
  report={healthReport}
  {config}
  {capabilities}
  isRefreshing={isRefreshingHealth}
  onClose={() => (showRuntimeModal = false)}
  onRefresh={refreshConnection}
/>

<CommandPalette
  open={paletteOpen}
  commands={paletteCommands}
  recentIds={commandRecentIds}
  onExecute={executePaletteCommand}
  onClose={() => (paletteOpen = false)}
/>

<WorkspacePickerModal
  open={workspacePickerOpen}
  current={currentWorkspace}
  suggestions={workspaceSuggestions}
  onPick={(dir) => void applyConversationWorkspace(dir)}
  onCancel={() => (workspacePickerOpen = false)}
/>

{#if showFirstRun}
  <FirstRunWizard onComplete={completeFirstRun} onSkip={skipFirstRun} />
{/if}

<style>
  .app-root {
    position: relative;
    height: 100vh;
    overflow: hidden;
    background: var(--ap-space-void, #07070c);
    color: var(--ap-bone);
  }
  .scene-underlay {
    position: absolute;
    inset: 0;
    z-index: 0;
  }
  .layer-xfade {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    transition: opacity 0.8s ease;
  }
  .layer-xfade.layer-off {
    opacity: 0;
  }
  .app-root.mode-focus .layer-xfade {
    transition-duration: 0.6s;
  }
  .planet-xfade {
    pointer-events: none;
    transition: opacity 0.8s ease;
  }
  .planet-xfade.layer-off {
    opacity: 0;
  }
  .app-root.mode-focus .planet-xfade {
    transition-duration: 0.6s;
  }
  .app-root.mode-focus .shell,
  .app-root.intro-playing .shell,
  .app-root.mode-focus #presence,
  .app-root.intro-playing #presence,
  .app-root.mode-focus #vignette,
  .app-root.intro-playing #vignette {
    opacity: 0;
    pointer-events: none;
  }
  .app-root.mode-focus .shell,
  .app-root.intro-playing .shell {
    transition: opacity 0.6s ease;
  }
  .focus-exit {
    position: absolute;
    left: 50%;
    bottom: 28px;
    transform: translateX(-50%);
    z-index: 6;
    display: none;
    padding: 7px 18px;
    border-radius: 999px;
    border: 1px solid rgba(255, 210, 122, 0.45);
    background: rgba(7, 7, 12, 0.72);
    color: var(--ap-gold);
    font-size: 12px;
    letter-spacing: 0.16em;
    cursor: pointer;
    pointer-events: auto;
  }
  .app-root.mode-focus .focus-exit {
    display: inline-flex;
  }
  .legacy-toast {
    position: absolute;
    left: 50%;
    bottom: 98px;
    transform: translateX(-50%);
    z-index: 8;
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: min(560px, 80vw);
    padding: 7px 16px;
    border-radius: 999px;
    background: var(--ap-panel);
    border: 1px solid var(--ap-line);
    backdrop-filter: blur(12px);
    color: rgba(232, 224, 204, 0.75);
    font-size: 11px;
    letter-spacing: 0.06em;
    pointer-events: auto;
  }
  .legacy-toast :global(svg) {
    color: var(--ap-gold);
    flex: none;
  }
  .drawer-body code {
    font-family: var(--ap-font-mono);
    font-size: 10.5px;
    color: var(--ap-bone-68);
    background: rgba(0, 0, 0, 0.3);
    padding: 1px 5px;
    border-radius: 3px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--ap-line);
    border-radius: 5px;
    padding: 7px 11px;
    color: var(--ap-bone-30);
  }
  .search input {
    flex: 1;
    border: 0;
    outline: 0;
    background: transparent;
    font-size: 12px;
    color: var(--ap-bone);
  }

  /* ---------- 会话头权限预设已迁至对话栏位（2026-10-11 主人反馈批） ---------- */

  /* ---------- 对话栏位常驻控制（2026-10-11 主人反馈批：Kimi Desktop 式输入栏） ---------- */
  .composer-preset {
    position: relative;
    flex: none;
  }
  .composer-preset-trigger {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 9px;
    border: 1px solid var(--ap-line);
    border-radius: 7px;
    background: var(--ap-panel);
    color: var(--ap-bone-62);
    font-size: 11.5px;
    letter-spacing: 0.02em;
    cursor: pointer;
    white-space: nowrap;
    transition: border-color 0.15s, color 0.15s;
  }
  .composer-preset-trigger:hover:not(:disabled),
  .composer-preset-trigger.open {
    border-color: var(--amber-line);
    color: var(--ap-bone);
  }
  .composer-preset-trigger:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .composer-preset-scrim {
    position: fixed;
    inset: 0;
    z-index: 210;
  }
  .composer-preset-pop {
    position: absolute;
    bottom: calc(100% + 10px);
    left: 0;
    z-index: 220;
    min-width: 240px;
    padding: 5px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .composer-preset-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 7px 10px;
    border: 0;
    background: transparent;
    border-radius: 7px;
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .composer-preset-item:hover {
    background: var(--surface-3);
  }
  .composer-preset-item.active {
    box-shadow: inset 2px 0 0 var(--amber);
  }
  .composer-preset-label {
    font-size: 12.5px;
    font-weight: 600;
  }
  .composer-preset-title {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
  }
  .composer-session-model {
    flex: none;
  }
  .composer-session-model :global(.model-picker .trigger) {
    max-width: 200px;
    padding: 6px 10px;
    font-size: 12px;
  }
  /* 窄输入栏：会话模型芯片优先，全局字母丸让位（双击状态栏仍可进设置改全局模型） */
  @media (max-width: 760px) {
    .composer-session-model :global(.model-picker .trigger) {
      max-width: 120px;
    }
  }

  /* ---------- 工具生命周期卡（P1-5） ----------
     旧 ToolCallCard 由 MessageContent 内部渲染；此处用新卡接管展示，
     隐藏旧容器避免双份工具条目。 */
  :global(.tool-calls-container) {
    display: none;
  }
  .tool-lifecycle-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 8px;
  }

  /* ---------- 状态提示横幅（非错误，不挂"请重试"方案） ---------- */
  .notice-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    padding: 10px 12px;
    border: 1px solid var(--blue);
    border-radius: 10px;
    background: var(--blue-wash);
    color: var(--text);
    font-size: 13px;
  }
  .notice-text {
    flex: 1;
  }
  .notice-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    padding: 2px;
    border-radius: 6px;
  }
  .notice-close:hover {
    color: var(--text);
    background: rgba(255, 255, 255, 0.06);
  }

  /* ---------- 待签文书卡的视觉语言在 chat-shell/PendingDocumentDock.svelte ---------- */

  /* ---------- 斜杠菜单锚点（P1-6） ---------- */
  .composer {
    position: relative;
  }

  /* ---------- T0 壳：会话头 + backend-only 诚实标注（三栏主从下返回键已取消） ---------- */
  .chat-head-main {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    min-width: 0;
  }
  .ledger-hint {
    margin: 0 0 18px;
    max-width: 52ch;
    font-family: var(--ap-font-mono);
    font-size: 10.5px;
    letter-spacing: 0.06em;
    line-height: 1.8;
    color: var(--ap-bone-42);
  }
</style>
