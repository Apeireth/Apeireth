<script lang="ts" module>
  // 「性格与记忆」一键预设值表（性格养成第一铲）。module 作用域导出：
  // 测试（tests/self-tuning-mapping.mjs）从源码正则抽取做镜像校验——.svelte
  // 无法被 node 直接 import。取值域：memoryFade/curiosityStrength ∈
  // [0.25, 4]（0.25 步进）、toneSaturation ∈ [0, 2]（0.2 步进）、
  // consolidationCadence ∈ [1, 10]（整数）。预设只动这 4 个体验参数。
  export const DISPOSITION_PRESETS = {
    '省心': {memoryFade: 1.5, curiosityStrength: 0.75, toneSaturation: 0.8, consolidationCadence: 2},
    '均衡': {memoryFade: 1.0, curiosityStrength: 1.0, toneSaturation: 1.0, consolidationCadence: 1},
    '深度记忆': {memoryFade: 0.5, curiosityStrength: 1.5, toneSaturation: 1.2, consolidationCadence: 1},
  } as const;

  /** 「恢复基线」：四旋钮回基线 + 「从使用中学习」关回默认关（fail-closed）。 */
  export const DISPOSITION_BASELINE_RESET = {
    memoryFade: 1.0,
    curiosityStrength: 1.0,
    toneSaturation: 1.0,
    consolidationCadence: 1,
    selfTuning: false,
  } as const;
</script>

<script lang="ts">
  import {untrack} from 'svelte';
  import {
    Settings,
    Server,
    Key,
    Cpu,
    User,
    Layers3,
    Shield,
    Activity,
    Trash2,
    Code,
    Check,
    RotateCcw,
    Lock,
    Eye,
    EyeOff,
    AlertTriangle,
    Globe,
    Bot,
    Sparkles,
    CheckCircle2,
    XCircle,
    Info,
    ChevronDown,
    ChevronUp,
    Plus,
    Palette,
    Search,
    Folder,
    Brain,
    Scale,
    Wrench,
    HeartHandshake,
    Network,
    BookOpen,
    Dumbbell,
    Users,
    GitBranch,
    Landmark,
    SlidersHorizontal,
    ShieldCheck,
    ShieldOff,
    FolderSearch,
    Terminal,
    RefreshCcw,
    Radar,
    Workflow,
    Timer,
    MessageSquarePlus,
    Archive,
    Filter,
    Tag,
    Copy,
  } from 'lucide-svelte';
  import PageHeader from '../../components/PageHeader.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import ThemeSettingsPanel from '../components/ThemeSettingsPanel.svelte';
  import SessionModelPicker from '../components/SessionModelPicker.svelte';
  import WorkspacePickerModal from '../components/WorkspacePickerModal.svelte';
  import ErrorSolutionBanner from '../components/ErrorSolutionBanner.svelte';
  import type {ApeirethConfig, RuntimeHealthReport, ProviderProtocol, ProviderConfig, PersonaProfile, CapabilityToggles, ModelInfo, AdminConfigPatch} from '../types';
  import {DEFAULT_CAPABILITY_TOGGLES, RECOMMENDED_CAPABILITY_PRESET} from '../types';
  import {
    checkHealthDetailed,
    listModels,
    testProviderConnection,
    DEFAULT_PERSONAS,
    applyAdminConfig,
    getAdminConfig,
    describeError,
    describeCaught,
    HttpError,
    normalizeBaseUrl,
  } from '../runtime';
  import {
    getProviderKey,
    setProviderKey,
    deleteProviderKey,
    getWorkspaceDir,
    setWorkspaceDir,
    listWorkspaceSuggestions,
  } from '../tauri-bridge';
  import type {TuningLogEntry} from '../desktop-bridge';
  import {isDesktop, readTuningLog} from '../desktop-bridge';

  let {
    config,
    onSave,
    onClearLocalData,
  }: {
    config: ApeirethConfig;
    onSave: (newConfig: ApeirethConfig) => void;
    onClearLocalData?: () => void;
  } = $props();

  type SettingsSection =
    | 'appearance'
    | 'models'
    | 'personality'
    | 'cognition'
    | 'disposition'
    | 'governance'
    | 'tools'
    | 'runtime'
    | 'data'
    | 'developer';

  let activeSection = $state<SettingsSection>('appearance');

  // Gateway backend fields
  let editBaseUrl = $state('');
  let saveSuccess = $state(false);
  let showAdvancedGateway = $state(false);

  // Backend advanced-capability toggles (fail-closed defaults, applied on save)
  // 草稿语义：只取 config 初值快照（保存时才回写），untrack 显式声明不跟踪。
  let capabilities = $state<CapabilityToggles>(
    untrack(() => ({
      ...DEFAULT_CAPABILITY_TOGGLES,
      ...(config.capabilities ?? {}),
    })),
  );

  // 认知深度预设：轻量 / 平衡 / 深度 / 自定义（judge + council 的快捷档位）。
  type CognitiveDepth = 'light' | 'balanced' | 'deep' | 'custom';

  function deriveCognitiveDepth(caps: CapabilityToggles): CognitiveDepth {
    if (!caps.judge && !caps.council) return 'light';
    if (caps.judge && !caps.council) return 'balanced';
    if (caps.judge && caps.council) return 'deep';
    return 'custom';
  }

  let cognitiveDepth = $state<CognitiveDepth>('light');

  // 仅在外部 config 变化时重新派生初始档位；不跟踪本地 capabilities，
  // 保证页内手动改 judge/council 开关后 select 稳定停在「自定义」。
  $effect(() => {
    cognitiveDepth = deriveCognitiveDepth({
      ...DEFAULT_CAPABILITY_TOGGLES,
      ...(config.capabilities ?? {}),
    });
  });

  function applyCognitiveDepth(): void {
    if (cognitiveDepth === 'balanced') {
      capabilities = {...capabilities, judge: true, council: false};
    } else if (cognitiveDepth === 'deep') {
      capabilities = {...capabilities, judge: true, council: true};
    } else if (cognitiveDepth === 'light') {
      capabilities = {...capabilities, judge: false, council: false};
    }
    // custom: 用户手动改 judge/council，保持现状。
  }

  function handleCapabilityToggle(key: keyof CapabilityToggles, checked: boolean): void {
    capabilities = {...capabilities, [key]: checked};
    if (key === 'judge' || key === 'council') {
      cognitiveDepth = 'custom';
    }
  }

  // ---- 能力中心（2026-10-10 W2/W3 收官批）：分组卡片式旋钮注册表 ----
  // 设计语言：电脑版微信/QQ 设置页——图标 + 标题 + 副文案 + 右侧开关的分组列表。
  // 每个旋钮如实标注后端 env 名（工程诚实），env 芯片用等宽弱色，不抢视觉。

  type CapDef = {
    key: keyof CapabilityToggles;
    icon: typeof Network;
    label: string;
    desc: string;
    env: string;
    /** 反向旋钮：开启 = 不注入（如 DISABLE_TYPED_RECALL，默认开）。 */
    invert?: boolean;
    /** 前置能力：未开启时本行不可用（如沙箱行依赖 shell）。 */
    requires?: keyof CapabilityToggles;
  };

  /** 记忆流：六历史流体系的运行态开关。 */
  const MEMORY_FLOW_DEFS: CapDef[] = [
    {key: 'memoryInjection', icon: MessageSquarePlus, label: '记忆注入', env: 'APEIRETH_ENABLE_MEMORY_INJECTION', desc: '把召回的相关记忆注进每轮上下文，越聊越懂你。'},
    {key: 'consolidation', icon: Archive, label: '记忆固化', env: 'APEIRETH_ENABLE_CONSOLIDATION', desc: '后台把散碎对话提炼成长期记忆（consolidation）。'},
    {key: 'reflexion', icon: RefreshCcw, label: '反思沉淀', env: 'APEIRETH_ENABLE_REFLEXION', desc: 'reflexion 文件回流：从错误与复盘里沉淀经验。'},
    {key: 'proactiveRecall', icon: Radar, label: '前瞻召回', env: 'APEIRETH_ENABLE_PROACTIVE_RECALL', desc: '闲置时主动浮现可能相关的记忆，不打断但在场。'},
    {key: 'typedRecall', icon: Layers3, label: '类型化召回', env: 'APEIRETH_DISABLE_TYPED_RECALL', invert: true, desc: '按事件/偏好/事实等类型分别召回（默认开）。'},
  ];

  /** 认知增强：W2 接线批落地的五件 + 器官链/偏好学习。 */
  const COGNITION_DEFS: CapDef[] = [
    {key: 'organs', icon: Network, label: '器官链（9 organs）', env: 'APEIRETH_ENABLE_ORGANS', desc: '回合后跑反事实推演/好奇心/情绪记忆等（AfterTurn，不阻塞回复）。'},
    {key: 'preferenceLearning', icon: HeartHandshake, label: '偏好学习', env: 'APEIRETH_ENABLE_PREFERENCE_LEARNING', desc: '把主人偏好写成双索引记忆，后续召回按主题展开。'},
    {key: 'partnerBond', icon: Users, label: '伙伴羁绊', env: 'APEIRETH_ENABLE_PARTNER_BOND', desc: '关系阶段/深度注入语气校准，回合后确定性演化（W2 §4.2）。'},
    {key: 'morphologyRecall', icon: SlidersHorizontal, label: '检索深度自适应', env: 'APEIRETH_ENABLE_MORPHOLOGY_RECALL', desc: '按形态学读数收紧检索条数——只收紧不放大（W2 §4.3）。'},
    {key: 'education', icon: BookOpen, label: 'Dx-Check 教育工具', env: 'APEIRETH_ENABLE_EDUCATION', desc: '模型可调用自查式教学工具，回答前先自检（W2 §4.3）。'},
    {key: 'absorptionInsight', icon: Dumbbell, label: '认知体操', env: 'APEIRETH_ENABLE_ABSORPTION_INSIGHT', desc: '四算法洞察注入（betti/残差金字塔/river/kuramoto，实验性，W2 §4.4）。'},
  ];

  /** 社区与账本：W3 移植批的检索路由与可溯源记账。 */
  const COMMUNITY_DEFS: CapDef[] = [
    {key: 'communityTriage', icon: GitBranch, label: '图社区分诊', env: 'APEIRETH_ENABLE_COMMUNITY_TRIAGE', desc: '检索前置双路路由：命中实体走实体链，否则给社区摘要（W3 §1）。'},
    {key: 'oneringLedger', icon: Landmark, label: 'onering 账本', env: 'APEIRETH_ENABLE_ONERING_LEDGER', desc: '每回合 user/assistant 留痕入 context_ledger，全程可溯源（W3）。'},
  ];

  /** 决策：评审/议会 + 深度预设。 */
  const DECISION_DEFS: CapDef[] = [
    {key: 'judge', icon: Scale, label: '评审 (Judge)', env: 'APEIRETH_COGNITIVE_JUDGE', desc: '模型回复后由评审模块判一次（每次评审最多一次 side-call）。'},
    {key: 'council', icon: Users, label: '议会 (Council)', env: 'APEIRETH_COGNITIVE_COUNCIL', desc: '决策环节多顾问并行裁决；一般运行时不常开，只在重要决策用。'},
  ];

  /** 治理：三洋葱三段门 + 子代理 worktree 隔离。 */
  const GUARD_DEFS: CapDef[] = [
    {key: 'onionLayer', icon: ShieldCheck, label: '三洋葱治理层', env: 'APEIRETH_ENABLE_ONION_LAYER', desc: 'L3-L5 三段门：HA 离线 = 物理隔离拒绝；只收紧已放行的（W3 工作项 6）。'},
    {key: 'worktreeSandbox', icon: Workflow, label: '子代理 worktree 隔离', env: 'APEIRETH_ENABLE_WORKTREE_SANDBOX', desc: '子代理在独立 git worktree 里干活，物理目录级隔离，收束后清理。'},
  ];

  /** 工具：三类可授予的工具权限。 */
  const TOOL_DEFS: CapDef[] = [
    {key: 'shell', icon: Terminal, label: 'Shell 命令工具', env: 'APEIRETH_ENABLE_SHELL', desc: '模型可提议本地命令——每次执行前仍需你在审批卡点头。'},
    {key: 'fetch', icon: Globe, label: '网络读取工具', env: 'APEIRETH_ENABLE_FETCH', desc: '公网 GET 只读请求，无凭据转发。'},
    {key: 'localReadTools', icon: FolderSearch, label: '本地只读工具', env: 'APEIRETH_ENABLE_LOCAL_READ_TOOLS', desc: '文件/搜索/仓库读侧工具（file / search / repo）。'},
  ];

  /** Beta 功能（开发者选项页）：默认关、随时可撤，稳定后晋升正式设置页。 */
  const BETA_DEFS: CapDef[] = [
    {key: 'reasoningEnabled', icon: Brain, label: '思考模式 (Reasoning)', env: 'APEIRETH_REASONING_ENABLED', desc: '把模型的 reasoning_content 分流展示——思考过程单独渲染，不占正文。'},
  ];

  function isCapOn(def: CapDef): boolean {
    const value = capabilities[def.key];
    return def.invert ? value === false : value === true;
  }

  function toggleCap(def: CapDef): void {
    handleCapabilityToggle(def.key, def.invert ? capabilities[def.key] === true : capabilities[def.key] !== true);
  }

  function capDisabled(def: CapDef): boolean {
    return def.requires !== undefined && capabilities[def.requires] !== true;
  }

  function enabledCount(defs: CapDef[]): number {
    return defs.filter((def) => isCapOn(def) && !capDisabled(def)).length;
  }

  function setCouncilAdvisors(value: number): void {
    capabilities = {...capabilities, councilAdvisors: Math.min(7, Math.max(1, Math.round(value) || 1))};
  }

  function setCouncilTimeout(value: number): void {
    capabilities = {...capabilities, councilTimeoutMs: Math.max(1000, Math.round(value) || 30000)};
  }

  function setMorphologyTemperature(value: number): void {
    capabilities = {...capabilities, morphologyTemperature: Math.min(2, Math.max(0.1, Math.round(value * 10) / 10))};
  }

  function setReasoningFilters(value: string): void {
    capabilities = {...capabilities, reasoningModelFilters: value};
  }

  function setReasoningTag(value: string): void {
    capabilities = {...capabilities, reasoningTag: value.trim() || 'think'};
  }

  // ---- 「推荐配置」一键预设（能力中心） ----
  // 开启记忆核心族三件 + 记忆固化/反思沉淀/器官链（RECOMMENDED_CAPABILITY_PRESET），
  // 绝不包含 shell/fetch 等危险工具；其余开关保持现状。带确认说明，确认后走
  // 与「保存设置」同一条 apply 路径（onSave → src-tauri env 注入 + 侧车重启/热应用）。
  let showRecommendedConfirm = $state(false);

  function applyRecommendedPreset(): void {
    const next: CapabilityToggles = {...capabilities};
    for (const key of RECOMMENDED_CAPABILITY_PRESET) {
      next[key] = true;
    }
    capabilities = next;
    showRecommendedConfirm = false;
    void handleSaveSettings();
  }

  // ---- 「性格与记忆」性格养成第一铲（体验层四旋钮 + 自学习开关 + 学习日志） ----
  // 治理分级：只暴露四个体验层数值旋钮（遗忘衰减/好奇/语气/整合节奏）与一个
  // 「从使用中学习」开关；治理与内核参数不可调、也不在此出现。取值域与 Rust
  // 侧 self_tuning.rs / backend_supervisor.rs 镜像一致（tests/self-tuning-mapping.mjs
  // 做源码级镜像校验）。四个 setter 镜像 setMorphologyTemperature 风格：钳到
  // [min,max] 并吸附步进，非有限值回基线。

  function setMemoryFade(value: number): void {
    // [0.25, 4]、0.25 步进；基线 1.0。
    const snapped = Number.isFinite(value) ? Math.round(value * 4) / 4 : 1.0;
    capabilities = {...capabilities, memoryFade: Math.min(4, Math.max(0.25, snapped))};
  }

  function setCuriosityStrength(value: number): void {
    // [0.25, 4]、0.25 步进；基线 1.0。
    const snapped = Number.isFinite(value) ? Math.round(value * 4) / 4 : 1.0;
    capabilities = {...capabilities, curiosityStrength: Math.min(4, Math.max(0.25, snapped))};
  }

  function setToneSaturation(value: number): void {
    // [0, 2]、0.2 步进（×5 取整再 /5）；基线 1.0。
    const snapped = Number.isFinite(value) ? Math.round(value * 5) / 5 : 1.0;
    capabilities = {...capabilities, toneSaturation: Math.min(2, Math.max(0, snapped))};
  }

  function setConsolidationCadence(value: number): void {
    // [1, 10] 整数；基线 1（每回合 = 现行为）。
    const rounded = Number.isFinite(value) ? Math.round(value) : 1;
    capabilities = {...capabilities, consolidationCadence: Math.min(10, Math.max(1, rounded))};
  }

  function setSelfTuning(on: boolean): void {
    capabilities = {...capabilities, selfTuning: on === true};
  }

  // 预设/恢复基线：带确认框（同「应用推荐配置」模式），确认后走与「保存设置」
  // 同一条 apply 路径（onSave → 配置持久化 + env 注入 + 网关热应用/重启）。
  let dispositionPresetPending = $state<'省心' | '均衡' | '深度记忆' | '恢复基线' | null>(null);

  /** 待确认预设的四旋钮取值（恢复基线 = 基线四值）；null = 无待确认。 */
  const pendingDispositionValues = $derived(
    dispositionPresetPending === null
      ? null
      : dispositionPresetPending === '恢复基线'
        ? {memoryFade: 1.0, curiosityStrength: 1.0, toneSaturation: 1.0, consolidationCadence: 1}
        : DISPOSITION_PRESETS[dispositionPresetPending],
  );

  function confirmDispositionPreset(): void {
    const name = dispositionPresetPending;
    dispositionPresetPending = null;
    if (name === '恢复基线') {
      // 恢复基线：四旋钮回基线 + 「从使用中学习」关回默认关（fail-closed）。
      capabilities = {...capabilities, ...DISPOSITION_BASELINE_RESET};
    } else if (name !== null) {
      const preset = DISPOSITION_PRESETS[name];
      capabilities = {
        ...capabilities,
        memoryFade: preset.memoryFade,
        curiosityStrength: preset.curiosityStrength,
        toneSaturation: preset.toneSaturation,
        consolidationCadence: preset.consolidationCadence,
      };
    } else {
      return;
    }
    void handleSaveSettings();
  }

  // ---- 「学习日志」：只读展示后端写入的自动调整记录 ----
  let tuningLog = $state<TuningLogEntry[] | null>(null);
  let tuningLogLoading = $state(false);

  async function refreshTuningLog(): Promise<void> {
    tuningLogLoading = true;
    try {
      tuningLog = await readTuningLog();
    } finally {
      tuningLogLoading = false;
    }
  }

  // 进入「性格与记忆」板块时读一次学习日志（桌面版接口；非桌面环境返回 null）。
  $effect(() => {
    if (activeSection === 'disposition') {
      void refreshTuningLog();
    }
  });

  /** 日志按 seq 倒序（最新在前）。 */
  const tuningLogSorted = $derived([...(tuningLog ?? [])].sort((a, b) => b.seq - a.seq));

  // snake_case 参数名 → 中文标签（与上方滑杆标签一致）。
  const TUNING_PARAM_LABELS: Record<string, string> = {
    memory_fade: '遗忘衰减强度',
    curiosity_strength: '好奇心强度',
    tone_saturation: '语气情绪饱和度',
    consolidation_cadence: '整合节奏',
  };

  function tuningParamLabel(param: string): string {
    return TUNING_PARAM_LABELS[param] ?? param;
  }

  function formatTuningTime(epochMs: number): string {
    const d = new Date(epochMs);
    return Number.isNaN(d.getTime()) ? String(epochMs) : d.toLocaleString('zh-CN');
  }

  // 「撤销」语义：把该参数的滑杆值拨回记录里的 previous（经 setter 钳到滑杆
  // 范围），然后立即走现有保存/应用路径（handleSaveSettings：配置持久化 +
  // env 注入 + 网关热应用/重启），把值写回去。之所以不另立 revert-request
  // 文件：现有 apply 路径是既定架构，独立的撤销请求需要后端新增一条摄取通路，
  // 超出本次「性格养成第一铲」范围。
  function undoTuningEntry(entry: TuningLogEntry): void {
    switch (entry.param) {
      case 'memory_fade':
        setMemoryFade(entry.previous);
        break;
      case 'curiosity_strength':
        setCuriosityStrength(entry.previous);
        break;
      case 'tone_saturation':
        setToneSaturation(entry.previous);
        break;
      case 'consolidation_cadence':
        setConsolidationCadence(entry.previous);
        break;
      default:
        return;
    }
    void handleSaveSettings();
  }

  // Model Provider protocol & preset configurations
  const OPENAI_PRESETS = [
    {
      id: 'openai',
      name: 'OpenAI 官方',
      baseUrl: 'https://api.openai.com/v1',
      defaultModel: 'gpt-4o',
      models: ['gpt-4o', 'gpt-4o-mini', 'o3-mini', 'o1'],
    },
    {
      id: 'deepseek',
      name: 'DeepSeek',
      baseUrl: 'https://api.deepseek.com/v1',
      defaultModel: 'deepseek-v4-flash',
      models: ['deepseek-v4-flash', 'deepseek-chat', 'deepseek-reasoner'],
    },
    {
      id: 'minimax',
      name: 'MiniMax',
      baseUrl: 'https://api.minimax.chat/v1',
      defaultModel: 'MiniMax-M3',
      models: ['MiniMax-M3', 'MiniMax-Text-01'],
    },
    {
      id: 'ollama',
      name: 'Ollama 本地',
      baseUrl: 'http://localhost:11434/v1',
      defaultModel: 'llama3.3',
      models: ['llama3.3', 'qwen2.5-coder', 'deepseek-r1'],
    },
    {
      id: 'custom',
      name: '自定义 OpenAI 端点',
      baseUrl: '',
      defaultModel: '',
      models: [],
    },
  ];

  const ANTHROPIC_PRESETS = [
    {
      id: 'anthropic',
      name: 'Anthropic 官方',
      baseUrl: 'https://api.anthropic.com',
      defaultModel: 'claude-3-7-sonnet-20250219',
      models: ['claude-3-7-sonnet-20250219', 'claude-3-5-sonnet-20241022', 'claude-3-5-haiku-20241022'],
    },
    {
      id: 'minimax_anthropic',
      name: 'MiniMax (Anthropic 网关)',
      baseUrl: 'https://api.minimaxi.com/anthropic',
      defaultModel: 'MiniMax-M3',
      models: ['MiniMax-M3'],
    },
    {
      id: 'custom',
      name: '自定义 Anthropic 端点',
      baseUrl: '',
      defaultModel: '',
      models: [],
    },
  ];

  let activeProtocol = $state<ProviderProtocol>('openai');
  let activePreset = $state<string>('openai');
  let providerBaseUrl = $state<string>('https://api.openai.com/v1');
  let providerApiKey = $state<string>('');
  let showApiKey = $state<boolean>(false);
  let providerModel = $state<string>('gpt-4o');
  let anthropicVersion = $state<string>('2023-06-01');

  // Buffer configs for protocol switching
  let openaiBuffer = $state({
    preset: 'openai',
    baseUrl: 'https://api.openai.com/v1',
    apiKey: '',
    model: 'gpt-4o',
  });

  let anthropicBuffer = $state({
    preset: 'anthropic',
    baseUrl: 'https://api.anthropic.com',
    apiKey: '',
    model: 'claude-3-7-sonnet-20250219',
    anthropicVersion: '2023-06-01',
  });

  // Test connection state
  let isTestingConnection = $state(false);
  let testResult = $state<{
    ok: boolean;
    message: string;
    latencyMs?: number;
    models?: string[];
  } | null>(null);

  // ---- 多 Agent 人设 (数据驱动, 本地编辑, 保存设置后生效) ----
  let personas = $state<PersonaProfile[]>([]);
  let activePersonaId = $state<string>('');
  $effect(() => {
    const source = config.personas && config.personas.length > 0 ? config.personas : DEFAULT_PERSONAS;
    personas = source.map((p) => ({...p}));
    activePersonaId = config.activePersonaId || personas[0]?.id || '';
  });

  function addPersona(): void {
    personas = [...personas, {id: crypto.randomUUID(), name: '新伙伴', persona: ''}];
  }

  function removePersona(id: string): void {
    if (personas.length <= 1) return;
    personas = personas.filter((p) => p.id !== id);
    if (activePersonaId === id) activePersonaId = personas[0]?.id || '';
  }

  // Sync initial config from props
  $effect(() => {
    editBaseUrl = config.baseUrl;

    if (config.openaiConfig) {
      openaiBuffer = {
        preset: config.openaiConfig.preset || 'openai',
        baseUrl: config.openaiConfig.baseUrl || 'https://api.openai.com/v1',
        apiKey: config.openaiConfig.apiKey || '',
        model: config.openaiConfig.model || 'gpt-4o',
      };
    }

    if (config.anthropicConfig) {
      anthropicBuffer = {
        preset: config.anthropicConfig.preset || 'anthropic',
        baseUrl: config.anthropicConfig.baseUrl || 'https://api.anthropic.com',
        apiKey: config.anthropicConfig.apiKey || '',
        model: config.anthropicConfig.model || 'claude-3-7-sonnet-20250219',
        anthropicVersion: config.anthropicConfig.anthropicVersion || '2023-06-01',
      };
    }

    if (config.provider) {
      activeProtocol = config.provider.protocol;
      activePreset = config.provider.preset || 'openai';
      providerBaseUrl = config.provider.baseUrl;
      providerApiKey = config.provider.apiKey || '';
      providerModel = config.provider.model || config.model;
      anthropicVersion = config.provider.anthropicVersion || '2023-06-01';
    } else {
      activeProtocol = 'openai';
      activePreset = 'openai';
      providerBaseUrl = 'https://api.openai.com/v1';
      providerApiKey = '';
      providerModel = config.model || 'gpt-4o';
    }
  });

  // Api key update modal for Gateway
  let showApiKeyModal = $state(false);
  let tempApiKey = $state('');

  // ---- wave-2 集成状态 (P0-1 钥匙串 / P1-1 无重启 / P0-2 模型 / P1-2 权限 / P1-4 工作区) ----

  // P0-1: 系统钥匙串回显状态（只存打码值，永不回显完整密钥）
  let storedKeyMasked = $state('');
  let storedKeyExists = $state(false);
  let keychainLoading = $state(false);
  let keychainSaving = $state(false);
  let keychainDeleting = $state(false);
  let keychainActionError = $state('');

  // P1-1: 无重启热应用状态
  let applying = $state(false);
  let applyError = $state<{code?: string; message: string; solution?: string} | null>(null);
  let applyResult = $state<{ok: boolean; warnings: string[]} | null>(null);
  let effectiveConfig = $state<{provider?: string; base_url?: string; api_key?: string; model?: string} | null>(null);

  // P0-2: 从 /v1/models 拉取的模型列表
  let discoveredModels = $state<ModelInfo[]>([]);
  let modelsLoading = $state(false);
  let modelsLoadedOnce = false;
  let modelsError = $state('');

  // P1-2: 全局权限预设默认值。admin config 契约无 permission_preset 字段
  // (它是会话级 session settings)，这里仅作 UI 状态 + 本地记录，不进入保存 patch。
  const PERMISSION_PRESET_KEY = 'apeireth-permission-preset-default';
  let permissionPreset = $state<'read_only' | 'standard' | 'full'>(initialPermissionPreset());

  // P1-4: 工作区目录
  let workspaceDir = $state('');
  let workspaceLoading = $state(false);
  let workspaceLoadedOnce = false;
  let workspaceError = $state('');
  let showWorkspacePicker = $state(false);
  let workspaceSuggestions = $state<string[]>([]);

  // Clear data confirmation modal
  let showClearConfirm = $state(false);

  // Runtime report
  let runtimeReport = $state<RuntimeHealthReport | null>(null);
  let checkingRuntime = $state(false);

  const hasApiKey = $derived(!!config.apiKey && config.apiKey.trim().length > 0);

  const currentPresets = $derived(activeProtocol === 'openai' ? OPENAI_PRESETS : ANTHROPIC_PRESETS);
  const currentPresetObj = $derived(currentPresets.find((p) => p.id === activePreset) || currentPresets[currentPresets.length - 1]);
  const recommendedModels = $derived(currentPresetObj?.models || []);

  let modelSearchQuery = $state('');

  const filteredDiscoveredModels = $derived.by(() => {
    const list = testResult?.models ?? [];
    const q = modelSearchQuery.trim().toLowerCase();
    if (!q) return list;
    return list.filter((m) => m.toLowerCase().includes(q));
  });

  const providerStatusLabel = $derived(
    testResult?.ok ? '已连通' : testResult && !testResult.ok ? '连接失败' : providerApiKey.trim() ? '待测试' : '未配置密钥',
  );

  const sections = [
    {id: 'appearance', label: '外观与主题', icon: Palette},
    {id: 'models', label: '模型与提供商', icon: Cpu},
    {id: 'personality', label: '伙伴人设与行为', icon: User},
    {id: 'cognition', label: '记忆与认知', icon: Brain},
    {id: 'disposition', label: '性格与记忆', icon: Sparkles},
    {id: 'governance', label: '决策与治理', icon: Scale},
    {id: 'tools', label: '工具与安全', icon: Wrench},
    {id: 'runtime', label: '运行时与诊断', icon: Activity},
    {id: 'data', label: '数据与存储', icon: Trash2},
    {id: 'developer', label: '开发者选项', icon: Code},
  ] as const;

  function switchProtocol(protocol: ProviderProtocol) {
    if (activeProtocol === protocol) return;

    // Save current to buffer
    if (activeProtocol === 'openai') {
      openaiBuffer = {
        preset: activePreset,
        baseUrl: providerBaseUrl,
        apiKey: providerApiKey,
        model: providerModel,
      };
    } else {
      anthropicBuffer = {
        preset: activePreset,
        baseUrl: providerBaseUrl,
        apiKey: providerApiKey,
        model: providerModel,
        anthropicVersion,
      };
    }

    // Switch and restore from target buffer
    activeProtocol = protocol;
    testResult = null;

    if (protocol === 'openai') {
      activePreset = openaiBuffer.preset;
      providerBaseUrl = openaiBuffer.baseUrl;
      providerApiKey = openaiBuffer.apiKey;
      providerModel = openaiBuffer.model;
    } else {
      activePreset = anthropicBuffer.preset;
      providerBaseUrl = anthropicBuffer.baseUrl;
      providerApiKey = anthropicBuffer.apiKey;
      providerModel = anthropicBuffer.model;
      anthropicVersion = anthropicBuffer.anthropicVersion || '2023-06-01';
    }
  }

  function selectPreset(presetId: string) {
    activePreset = presetId;
    testResult = null;
    const p = currentPresets.find((item) => item.id === presetId);
    if (p && p.id !== 'custom') {
      providerBaseUrl = p.baseUrl;
      if (p.defaultModel && (!providerModel || p.models.includes(providerModel) || providerModel === 'gpt-4o' || providerModel === 'MiniMax-M3' || providerModel === 'claude-3-7-sonnet-20250219')) {
        providerModel = p.defaultModel;
      }
    }
  }

  async function handleTestProviderConnection() {
    isTestingConnection = true;
    testResult = null;
    try {
      const currentProvider: ProviderConfig = {
        protocol: activeProtocol,
        preset: activePreset,
        baseUrl: providerBaseUrl.trim(),
        apiKey: providerApiKey.trim(),
        model: providerModel.trim(),
        anthropicVersion: activeProtocol === 'anthropic' ? anthropicVersion.trim() : undefined,
      };
      const res = await testProviderConnection(currentProvider);
      testResult = res;
    } finally {
      isTestingConnection = false;
    }
  }

  const DEFAULT_MODEL_ID = 'deepseek-v4-flash';

  /** 钥匙串 / env family 标识：OpenAI 兼容预设统一用 'openai'。 */
  function providerFamily(): 'openai' | 'minimax' | 'anthropic' {
    if (activeProtocol === 'anthropic') return 'anthropic';
    if (activePreset === 'minimax') return 'minimax';
    return 'openai';
  }

  /** 与后端 mask_api_key 一致：首 3 + **** + 末 3，短密钥整体打码。 */
  function maskApiKey(key: string): string {
    const value = key.trim();
    if (value.length <= 8) return '****';
    return `${value.slice(0, 3)}****${value.slice(-3)}`;
  }

  function errorBannerFrom(err: unknown): {code?: string; message: string; solution?: string} {
    const described = describeError(err);
    if (err instanceof HttpError) {
      return {
        code: err.code ?? (err.status ? `HTTP ${err.status}` : undefined),
        message: err.message,
        solution: err.solution ?? described.solution,
      };
    }
    return {
      message: describeCaught(err),
      solution: described.solution,
    };
  }

  function initialPermissionPreset(): 'read_only' | 'standard' | 'full' {
    try {
      const raw = localStorage.getItem(PERMISSION_PRESET_KEY);
      if (raw === 'read_only' || raw === 'standard' || raw === 'full') return raw;
    } catch {
      // localStorage 不可用时保持内存默认值。
    }
    return 'standard';
  }

  async function loadModels() {
    modelsLoading = true;
    modelsError = '';
    discoveredModels = [];
    try {
      discoveredModels = await listModels(config);
    } catch {
      modelsError = '从 /v1/models 加载失败 — 检查密钥配置';
    } finally {
      modelsLoading = false;
    }
  }

  async function openApiKeyModal() {
    tempApiKey = '';
    keychainActionError = '';
    storedKeyExists = false;
    storedKeyMasked = '';
    keychainLoading = true;
    showApiKeyModal = true;
    const stored = await getProviderKey(providerFamily());
    if (stored) {
      storedKeyExists = true;
      storedKeyMasked = maskApiKey(stored);
    }
    keychainLoading = false;
  }

  async function deleteStoredKey() {
    keychainDeleting = true;
    keychainActionError = '';
    const removed = await deleteProviderKey(providerFamily());
    if (!removed) {
      keychainDeleting = false;
      keychainActionError = '删除钥匙串密钥失败，请重试。';
      return;
    }
    storedKeyExists = false;
    storedKeyMasked = '';
    keychainDeleting = false;

    // 同步清掉内存中的 provider 密钥，避免删除后测试连接仍带上旧 key。
    providerApiKey = '';
    const updated: ApeirethConfig = {
      ...config,
      apiKey: '',
      provider: config.provider ? {...config.provider, apiKey: ''} : config.provider,
      openaiConfig: config.openaiConfig
        ? {...config.openaiConfig, apiKey: ''}
        : config.openaiConfig,
    };
    onSave(updated);
  }

  async function loadWorkspace() {
    workspaceLoading = true;
    workspaceError = '';
    const dir = await getWorkspaceDir();
    workspaceDir = dir ?? '';
    workspaceLoading = false;
  }

  async function openWorkspacePicker() {
    workspaceError = '';
    showWorkspacePicker = true;
    const suggestions = await listWorkspaceSuggestions();
    workspaceSuggestions = suggestions ?? [];
  }

  async function handleWorkspacePick(dir: string) {
    workspaceError = '';
    const applied = await setWorkspaceDir(dir);
    if (applied !== null) {
      workspaceDir = applied;
    } else {
      workspaceError = '设置工作区目录失败，请检查路径权限后重试。';
    }
    showWorkspacePicker = false;
  }

  // 首次挂载即拉取模型列表与工作区目录。
  $effect(() => {
    if (!modelsLoadedOnce) {
      modelsLoadedOnce = true;
      void loadModels();
    }
    if (!workspaceLoadedOnce) {
      workspaceLoadedOnce = true;
      void loadWorkspace();
    }
  });

  // 权限预设默认值仅做本地记录（会话级设置，不属于 admin config patch）。
  $effect(() => {
    try {
      localStorage.setItem(PERMISSION_PRESET_KEY, permissionPreset);
    } catch {
      // 忽略 localStorage 不可用
    }
  });

  async function handleSaveSettings() {
    const currentProvider: ProviderConfig = {
      protocol: activeProtocol,
      preset: activePreset,
      baseUrl: providerBaseUrl.trim(),
      apiKey: providerApiKey.trim(),
      model: providerModel.trim(),
      anthropicVersion: activeProtocol === 'anthropic' ? anthropicVersion.trim() : undefined,
    };

    const currentOpenai = activeProtocol === 'openai'
      ? { preset: activePreset, baseUrl: providerBaseUrl.trim(), apiKey: providerApiKey.trim(), model: providerModel.trim() }
      : openaiBuffer;

    const currentAnthropic = activeProtocol === 'anthropic'
      ? { preset: activePreset, baseUrl: providerBaseUrl.trim(), apiKey: providerApiKey.trim(), model: providerModel.trim(), anthropicVersion: anthropicVersion.trim() }
      : anthropicBuffer;

    const updated: ApeirethConfig = {
      ...config,
      baseUrl: editBaseUrl.trim(),
      model: providerModel.trim() || DEFAULT_MODEL_ID,
      provider: currentProvider,
      openaiConfig: currentOpenai,
      anthropicConfig: currentAnthropic,
      capabilities,
      personas: personas.length > 0 ? personas : config.personas,
      activePersonaId,
    };

    onSave(updated);
    saveSuccess = true;
    setTimeout(() => {
      saveSuccess = false;
    }, 1500);

    // P1-1: 无重启热应用，成功后回显网关生效配置。
    applying = true;
    applyError = null;
    applyResult = null;
    effectiveConfig = null;
    try {
      const patch: AdminConfigPatch = {
        provider: providerFamily(),
        base_url: normalizeBaseUrl(providerBaseUrl),
        model: providerModel.trim() || DEFAULT_MODEL_ID,
      };
      applyResult = await applyAdminConfig(updated, patch);
      effectiveConfig = await getAdminConfig(updated);
    } catch (err) {
      applyError = errorBannerFrom(err);
    } finally {
      applying = false;
    }
  }

  async function saveNewApiKey() {
    const key = tempApiKey.trim();
    if (!key) {
      keychainActionError = '请输入 API Key';
      return;
    }

    keychainSaving = true;
    keychainActionError = '';

    // P0-1: 密钥写入系统钥匙串（不落盘明文）。
    const stored = await setProviderKey(providerFamily(), key);
    if (!stored) {
      keychainSaving = false;
      keychainActionError = '写入系统钥匙串失败：请检查系统钥匙串是否可用后重试。';
      return;
    }

    // 同时热更新到运行中网关，避免重启。
    try {
      await applyAdminConfig(config, {api_key: key});
    } catch (err) {
      applyError = errorBannerFrom(err);
    }

    // 2026-09-28 real-world trap: this modal used to set only the vestigial
    // gateway `apiKey` field, which never reaches the sidecar — users filled
    // it, saved, and chat still failed with "missing API key". The key must
    // ALSO land in the provider config, because that is what the desktop
    // pushes into the sidecar environment via apply_backend_config.
    const updated: ApeirethConfig = {
      ...config,
      apiKey: key,
      provider: config.provider ? {...config.provider, apiKey: key} : config.provider,
      openaiConfig: config.openaiConfig
        ? {...config.openaiConfig, apiKey: key}
        : config.openaiConfig,
    };
    onSave(updated);
    providerApiKey = key;
    storedKeyExists = true;
    storedKeyMasked = maskApiKey(key);
    tempApiKey = '';
    showApiKeyModal = false;
    keychainSaving = false;
  }

  async function checkDiagnostics() {
    checkingRuntime = true;
    try {
      runtimeReport = await checkHealthDetailed(config.baseUrl, config.apiKey, providerModel);
    } finally {
      checkingRuntime = false;
    }
  }

  // ---- 开发者选项：客户端配置 JSON 复制 ----
  let configJsonCopied = $state(false);
  const configJsonPreview = $derived(
    JSON.stringify({baseUrl: config.baseUrl, model: providerModel, provider: config.provider, hasApiKey}, null, 2),
  );

  async function copyConfigJson(): Promise<void> {
    try {
      await navigator.clipboard.writeText(configJsonPreview);
      configJsonCopied = true;
      setTimeout(() => {
        configJsonCopied = false;
      }, 1500);
    } catch {
      // 剪贴板不可用时静默降级——配置仍可视。
    }
  }
</script>

<section class="settings-view">
  <PageHeader
    eyebrow="首选项"
    title="系统设置"
    subtitle="配置模型提供商、记忆与认知、决策治理、工具安全与数据存储。"
  >
    <button class="primary-button" onclick={handleSaveSettings}>
      <Check size={14} />
      <span>{saveSuccess ? '已保存！' : '保存设置'}</span>
    </button>
  </PageHeader>

  <div class="settings-layout">
    <!-- Left Navigation -->
    <aside class="settings-subnav">
      {#each sections as sec}
        <button
          class="subnav-btn"
          class:active={activeSection === sec.id}
          onclick={() => {
            activeSection = sec.id as SettingsSection;
            if (sec.id === 'runtime' && !runtimeReport) void checkDiagnostics();
          }}
        >
          <sec.icon size={15} />
          <span>{sec.label}</span>
        </button>
      {/each}
    </aside>

    <!-- Right Settings Panel -->
    <div class="settings-content">
      {#if activeSection === 'appearance'}
        <div class="setting-block">
          <h3 class="block-title">外观与主题</h3>
          <p class="block-desc">选择界面照明档位与背景风格，立即生效并写入本地配置。</p>
          <ThemeSettingsPanel {config} {onSave} />
        </div>

      {:else if activeSection === 'models'}
        <div class="setting-block">
          <h3 class="block-title">模型与提供商</h3>
          <p class="block-desc">配置 LLM 协议、服务商端点与活动模型；与 Dock 模型选择器共用同一配置源。</p>

          <div class="provider-summary">
            <div class="summary-main">
              <span class="summary-badge">{activeProtocol === 'openai' ? 'OpenAI 兼容' : 'Anthropic'}</span>
              <strong>{currentPresetObj?.name ?? '自定义'}</strong>
              <span class="summary-model">{providerModel || '未指定模型'}</span>
            </div>
            <div class="summary-side">
              <span class="status-pill" class:ok={testResult?.ok} class:warn={!testResult?.ok && testResult} class:idle={!testResult}>
                {providerStatusLabel}
              </span>
              <span class="summary-url">{providerBaseUrl || '—'}</span>
            </div>
          </div>

          {#if applying}
            <div class="apply-status"><RotateCcw size={13} class="spin" /><span>正在应用配置…</span></div>
          {/if}

          {#if applyError}
            <ErrorSolutionBanner
              code={applyError.code}
              message={applyError.message}
              solution={applyError.solution}
              onClose={() => (applyError = null)}
            />
          {/if}

          {#if applyResult && applyResult.warnings.length > 0}
            <div class="warnings-box">
              <strong>配置已应用，但有 {applyResult.warnings.length} 条警告：</strong>
              <ul>
                {#each applyResult.warnings as warning (warning)}
                  <li>{warning}</li>
                {/each}
              </ul>
            </div>
          {/if}

          {#if effectiveConfig}
            <div class="effective-config info-card">
              <strong class="info-title">网关生效配置 (无重启热应用)</strong>
              <div class="effective-grid">
                <span>Provider: <code>{effectiveConfig.provider ?? '—'}</code></span>
                <span>Base URL: <code>{effectiveConfig.base_url ?? '—'}</code></span>
                <span>Model: <code>{effectiveConfig.model ?? '—'}</code></span>
                <span>API Key: <code>{effectiveConfig.api_key ?? '未配置'}</code></span>
              </div>
            </div>
          {/if}

          <div class="config-step">
            <span class="step-num">1</span>
            <div class="step-body">
              <h4 class="step-title">选择协议</h4>
              <div class="protocol-tabs">
                <button
                  class="protocol-tab"
                  class:selected={activeProtocol === 'openai'}
                  onclick={() => switchProtocol('openai')}
                >
                  <Globe size={15} />
                  <div class="proto-text">
                    <span class="proto-title">OpenAI 兼容协议</span>
                    <span class="proto-sub">OpenAI · DeepSeek · MiniMax · Ollama · vLLM</span>
                  </div>
                </button>
                <button
                  class="protocol-tab"
                  class:selected={activeProtocol === 'anthropic'}
                  onclick={() => switchProtocol('anthropic')}
                >
                  <Sparkles size={15} />
                  <div class="proto-text">
                    <span class="proto-title">Anthropic Claude 协议</span>
                    <span class="proto-sub">Messages API · Claude 3.5 / 3.7</span>
                  </div>
                </button>
              </div>
            </div>
          </div>

          <div class="config-step">
            <span class="step-num">2</span>
            <div class="step-body">
              <h4 class="step-title">选择服务商预设</h4>
              <div class="presets-row">
                {#each currentPresets as p}
                  <button
                    class="preset-chip"
                    class:selected={activePreset === p.id}
                    onclick={() => selectPreset(p.id)}
                  >
                    <span>{p.name}</span>
                  </button>
                {/each}
              </div>
            </div>
          </div>

          <div class="config-step">
            <span class="step-num">3</span>
            <div class="step-body">
              <h4 class="step-title">端点、密钥与模型</h4>

              <div class="form-row-2">
                <div class="form-group">
                  <label for="provider-url-input">API 端点 (Base URL)</label>
                  <input
                    id="provider-url-input"
                    type="text"
                    bind:value={providerBaseUrl}
                    placeholder={activeProtocol === 'openai' ? 'https://api.openai.com/v1' : 'https://api.anthropic.com'}
                  />
                </div>
                <div class="form-group">
                  <label for="provider-model-input">活动模型 ID</label>
                  <input
                    id="provider-model-input"
                    type="text"
                    bind:value={providerModel}
                    placeholder={activeProtocol === 'openai' ? 'gpt-4o' : 'claude-3-7-sonnet-20250219'}
                  />
                </div>
              </div>

              <div class="form-group">
                <span class="group-label">从 /v1/models 选择模型</span>
                <div class="model-picker-row">
                  <SessionModelPicker
                    models={discoveredModels.map((m) => ({id: m.id, ownedBy: m.ownedBy}))}
                    value={providerModel || DEFAULT_MODEL_ID}
                    onSelect={(id) => (providerModel = id)}
                    disabled={modelsLoading}
                  />
                  <button
                    class="quiet-button"
                    onclick={() => void loadModels()}
                    disabled={modelsLoading}
                  >
                    <RotateCcw size={13} class={modelsLoading ? 'spin' : ''} />
                    <span>{modelsLoading ? '加载中…' : '重新加载'}</span>
                  </button>
                </div>
                {#if modelsError}
                  <small class="field-hint error-hint">{modelsError}</small>
                {/if}
                <small class="field-hint">默认模型 deepseek-v4-flash；列表来自网关 /v1/models。</small>
              </div>

              <div class="form-group">
                <label for="provider-key-input">
                  {activeProtocol === 'openai' ? 'API Key (Bearer)' : 'API Key (x-api-key)'}
                </label>
                <div class="key-input-wrapper">
                  <input
                    id="provider-key-input"
                    type={showApiKey ? 'text' : 'password'}
                    bind:value={providerApiKey}
                    placeholder={activeProtocol === 'openai' ? 'sk-...' : 'sk-ant-...'}
                    autocomplete="off"
                  />
                  <button
                    class="key-toggle-btn"
                    type="button"
                    onclick={() => (showApiKey = !showApiKey)}
                    title={showApiKey ? '隐藏密钥' : '显示密钥'}
                  >
                    {#if showApiKey}
                      <EyeOff size={14} />
                    {:else}
                      <Eye size={14} />
                    {/if}
                  </button>
                </div>
              </div>

              {#if activeProtocol === 'anthropic'}
                <div class="form-group">
                  <label for="anthropic-ver-input">anthropic-version</label>
                  <input id="anthropic-ver-input" type="text" bind:value={anthropicVersion} placeholder="2023-06-01" />
                </div>
              {/if}

              <div class="model-actions">
                <button
                  class="primary-button"
                  onclick={handleTestProviderConnection}
                  disabled={isTestingConnection || !providerBaseUrl}
                >
                  <RotateCcw size={13} class={isTestingConnection ? 'spin' : ''} />
                  <span>{isTestingConnection ? '探测中…' : '测试连接并拉取模型'}</span>
                </button>
                {#if recommendedModels.length > 0}
                  <span class="chip-label">预设推荐</span>
                  {#each recommendedModels as m}
                    <button
                      class="model-chip"
                      class:selected={providerModel === m}
                      onclick={() => (providerModel = m)}
                    >
                      {m}
                    </button>
                  {/each}
                {/if}
              </div>
            </div>
          </div>

          {#if testResult}
            <div class="test-result-box" class:success={testResult.ok} class:failed={!testResult.ok}>
              <div class="test-result-head">
                {#if testResult.ok}
                  <CheckCircle2 size={16} class="head-icon success-icon" />
                  <strong>连接成功</strong>
                {:else}
                  <XCircle size={16} class="head-icon failed-icon" />
                  <strong>连接失败</strong>
                {/if}
                {#if testResult.latencyMs !== undefined}
                  <span class="latency-badge">{testResult.latencyMs} ms</span>
                {/if}
              </div>
              <p class="test-result-msg">{testResult.message}</p>
              {#if testResult.models && testResult.models.length > 0}
                <div class="discovered-models">
                  <div class="disc-head">
                    <span class="disc-label">远端模型 ({testResult.models.length})</span>
                    <div class="disc-search">
                      <Search size={13} />
                      <input type="search" placeholder="筛选模型…" bind:value={modelSearchQuery} />
                    </div>
                  </div>
                  <div class="model-catalog">
                    {#each filteredDiscoveredModels.slice(0, 24) as dm}
                      <button
                        class="catalog-item"
                        class:selected={providerModel === dm}
                        onclick={() => (providerModel = dm)}
                      >
                        <span class="catalog-name">{dm}</span>
                        {#if providerModel === dm}
                          <Check size={12} />
                        {/if}
                      </button>
                    {:else}
                      <p class="catalog-empty">无匹配模型</p>
                    {/each}
                  </div>
                  {#if filteredDiscoveredModels.length > 24}
                    <p class="more-models">仅显示前 24 项，请用搜索缩小范围</p>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}

          <!-- Collapsible Gateway/Daemon Section -->
          <div class="advanced-box">
            <button
              class="advanced-toggle"
              onclick={() => showAdvancedGateway = !showAdvancedGateway}
            >
              <div class="adv-left">
                <Server size={14} />
                <span>Apeireth 核心网关与守护进程 (Advanced Gateway)</span>
              </div>
              {#if showAdvancedGateway}
                <ChevronUp size={14} />
              {:else}
                <ChevronDown size={14} />
              {/if}
            </button>

            {#if showAdvancedGateway}
              <div class="advanced-body">
                <div class="form-group">
                  <label for="endpoint-input">网关服务地址 (Gateway URL)</label>
                  <input
                    id="endpoint-input"
                    type="text"
                    bind:value={editBaseUrl}
                    placeholder="http://127.0.0.1:8080"
                  />
                  <small class="field-hint">默认为 Apeireth 核心网关端口 (:8080)。</small>
                </div>

                <div class="form-group">
                  <label for="api-key-status">网关认证密钥 (Gateway Auth Key)</label>
                  <div class="credential-row">
                    <div class="cred-status">
                      <Lock size={14} />
                      <span>{hasApiKey ? '已配置 (Configured)' : '未配置 (Not configured)'}</span>
                    </div>
                    <button class="quiet-button" onclick={() => void openApiKeyModal()}>
                      {hasApiKey ? '更换 Key' : '配置 Key'}
                    </button>
                  </div>
                  <small class="field-hint">
                    Apeireth 网关管理认证密钥（可选）。
                  </small>
                </div>
              </div>
            {/if}
          </div>
        </div>

      {:else if activeSection === 'personality'}
        <div class="setting-block">
          <h3 class="block-title">伙伴人设与行为 (Persona)</h3>
          <p class="block-desc">
            数据驱动的多 Agent 身份：可随时增删改，点「保存设置」后立即生效（人设作为 system 消息注入每次对话），无需重编译。
          </p>

          {#each personas as p, i (p.id)}
            <div class="persona-card">
              <div class="persona-card-head">
                <span class="persona-index">伙伴 {i + 1}</span>
                <div class="persona-card-actions">
                  <button
                    class="quiet-button"
                    class:selected={activePersonaId === p.id}
                    onclick={() => (activePersonaId = p.id)}
                    title="设为当前伙伴"
                  >
                    {activePersonaId === p.id ? '✓ 当前伙伴' : '设为当前'}
                  </button>
                  <button
                    class="quiet-button danger-text"
                    onclick={() => removePersona(p.id)}
                    disabled={personas.length <= 1}
                    title="删除该伙伴"
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              </div>
              <div class="form-group">
                <label for="persona-name-{p.id}">名称</label>
                <input id="persona-name-{p.id}" type="text" bind:value={p.name} placeholder="伙伴名称" />
              </div>
              <div class="form-group">
                <label for="persona-model-{p.id}">固定模型（可选，留空跟随全局设置）</label>
                <input
                  id="persona-model-{p.id}"
                  type="text"
                  value={p.model || ''}
                  oninput={(e) => (p.model = (e.currentTarget as HTMLInputElement).value.trim() || undefined)}
                  placeholder={config.model || 'deepseek-chat'}
                />
              </div>
              <div class="form-group">
                <label for="persona-text-{p.id}">人设文本（system 消息；留空 = 该伙伴不注入人设）</label>
                <textarea
                  id="persona-text-{p.id}"
                  rows={5}
                  bind:value={p.persona}
                  placeholder="你是「阿佩瑞斯」——Apeireth 基地的主管…"
                ></textarea>
              </div>
            </div>
          {/each}

          <button class="quiet-button" onclick={addPersona}>
            <Plus size={14} />
            <span>新增伙伴</span>
          </button>

          <div class="notice-box">
            <StatusBadge label="实时生效" variant="green" size="small" />
            <span>人设由客户端作为 system 消息注入每次请求；「保存设置」后立即生效，重启应用仍保留（不含任何密钥）。</span>
          </div>
        </div>

      {:else if activeSection === 'cognition'}
        <div class="setting-block">
          <h3 class="block-title">记忆与认知</h3>
          <p class="block-desc">
            记忆流运行态 + W2/W3 收官批落地的认知增强件。记忆核心族（记忆注入/前瞻召回/偏好学习）
            默认开启、可随时关；其余默认关闭、逐项显式开启。保存后注入侧车环境并重启网关（配置没变不会重启）。
          </p>

          <div class="cap-summary">
            <Brain size={14} />
            <span>本页已启用 <b>{enabledCount(MEMORY_FLOW_DEFS) + enabledCount(COGNITION_DEFS) + enabledCount(COMMUNITY_DEFS)}</b> / {MEMORY_FLOW_DEFS.length + COGNITION_DEFS.length + COMMUNITY_DEFS.length} 项认知能力</span>
          </div>

          <div class="preset-bar">
            <button class="quiet-button" onclick={() => (showRecommendedConfirm = true)}>
              <Sparkles size={14} />
              <span>应用推荐配置</span>
            </button>
            <span class="preset-hint">一键开启记忆核心族 + 记忆固化/反思沉淀/器官链，不含 shell/fetch。</span>
          </div>
          {#if showRecommendedConfirm}
            <div class="notice-box preset-confirm" role="alertdialog" aria-label="应用推荐配置确认">
              <span>
                将开启 6 项：前瞻召回、偏好学习、记忆注入、记忆固化、反思沉淀、器官链。
                <strong>不包含</strong> Shell / 网络读取等危险工具（它们保持现状、仍需逐项开启与审批），其余开关不动。
                确认后立即保存并走现有配置应用流程（注入侧车环境，配置变化时重启网关）。
              </span>
              <span class="preset-actions">
                <button class="quiet-button" onclick={applyRecommendedPreset}>保存并应用</button>
                <button class="quiet-button" onclick={() => (showRecommendedConfirm = false)}>取消</button>
              </span>
            </div>
          {/if}

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Layers3 size={13} /> 记忆流</span>
              <span class="cap-card-count">{enabledCount(MEMORY_FLOW_DEFS)}/{MEMORY_FLOW_DEFS.length}</span>
            </div>
            {#each MEMORY_FLOW_DEFS as def (def.key)}
              <button
                class="cap-row"
                class:dim={capDisabled(def)}
                onclick={() => toggleCap(def)}
                role="switch"
                aria-checked={isCapOn(def)}
                aria-label={def.label}
              >
                <span class="cap-icon"><def.icon size={15} /></span>
                <span class="cap-text">
                  <strong>{def.label}<code class="cap-env">{def.env}</code></strong>
                  <small>{def.desc}</small>
                </span>
                <span class="cap-switch" class:on={isCapOn(def)}><span class="cap-knob"></span></span>
              </button>
            {/each}
          </div>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Sparkles size={13} /> 认知增强 · W2 接线批</span>
              <span class="cap-card-count">{enabledCount(COGNITION_DEFS)}/{COGNITION_DEFS.length}</span>
            </div>
            {#each COGNITION_DEFS as def (def.key)}
              <button
                class="cap-row"
                class:dim={capDisabled(def)}
                onclick={() => toggleCap(def)}
                role="switch"
                aria-checked={isCapOn(def)}
                aria-label={def.label}
              >
                <span class="cap-icon"><def.icon size={15} /></span>
                <span class="cap-text">
                  <strong>{def.label}<code class="cap-env">{def.env}</code></strong>
                  <small>{def.desc}</small>
                </span>
                <span class="cap-switch" class:on={isCapOn(def)}><span class="cap-knob"></span></span>
              </button>
              {#if def.key === 'morphologyRecall' && capabilities.morphologyRecall}
                <div class="cap-row cap-row-static">
                  <span class="cap-icon"><SlidersHorizontal size={15} /></span>
                  <span class="cap-text">
                    <strong>检索温度<code class="cap-env">APEIRETH_MORPHOLOGY_TEMPERATURE</code></strong>
                    <small>形态学读数活跃度，默认 1.0；越高检索面越宽。</small>
                  </span>
                  <span class="cap-slider-wrap">
                    <input
                      type="range"
                      min="0.1"
                      max="2"
                      step="0.1"
                      value={capabilities.morphologyTemperature}
                      aria-label="检索温度"
                      oninput={(e) => setMorphologyTemperature(Number((e.currentTarget as HTMLInputElement).value))}
                    />
                    <b>{capabilities.morphologyTemperature.toFixed(1)}</b>
                  </span>
                </div>
              {/if}
            {/each}
          </div>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><GitBranch size={13} /> 社区与账本 · W3 移植批</span>
              <span class="cap-card-count">{enabledCount(COMMUNITY_DEFS)}/{COMMUNITY_DEFS.length}</span>
            </div>
            {#each COMMUNITY_DEFS as def (def.key)}
              <button
                class="cap-row"
                class:dim={capDisabled(def)}
                onclick={() => toggleCap(def)}
                role="switch"
                aria-checked={isCapOn(def)}
                aria-label={def.label}
              >
                <span class="cap-icon"><def.icon size={15} /></span>
                <span class="cap-text">
                  <strong>{def.label}<code class="cap-env">{def.env}</code></strong>
                  <small>{def.desc}</small>
                </span>
                <span class="cap-switch" class:on={isCapOn(def)}><span class="cap-knob"></span></span>
              </button>
            {/each}
          </div>

          <div class="cap-default-card">
            <span class="cap-default-badge on"><CheckCircle2 size={11} /> 默认能力 · 显式触发</span>
            <strong class="cap-default-title">做梦与反思 (Dream & Reflection)</strong>
            <p class="cap-default-text">
              无需开关、永远可用：命令行 <code>apeireth dream</code> 即授权一次做梦周期，
              苏醒后把提炼结果写日记（source=dream），日记失败不伪装成功。
            </p>
          </div>

          <p class="cap-footnote">
            fail-closed 语义：以上旋钮关闭（默认）时不注入任何环境变量，后端能力完全不存在；
            开启只注入 "1"。唯一的例外是沙箱（后端默认开），见「工具与安全」。
          </p>
        </div>

      {:else if activeSection === 'disposition'}
        <div class="setting-block">
          <h3 class="block-title">性格与记忆</h3>
          <p class="block-desc">
            4 个体验参数可手动调校（未设 = 基线行为，零变化）；「从使用中学习」默认关。
            参数只覆盖体验层（记忆遗忘/好奇/语气/整合节奏），治理与内核参数不可调。
          </p>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><SlidersHorizontal size={13} /> 体验参数 · 手动调校</span>
              <span class="cap-card-count">未设 = 基线行为</span>
            </div>

            <div class="cap-row cap-row-static">
              <span class="cap-icon"><Archive size={15} /></span>
              <span class="cap-text">
                <strong>遗忘衰减强度<code class="cap-env">APEIRETH_TUNE_MEMORY_FADE</code></strong>
                <small>遗忘衰减强度倍率（有效遗忘半衰期 = 24h / 值；1.0 = 现行为）。</small>
              </span>
              <span class="cap-slider-wrap">
                <input
                  type="range"
                  min="0.25"
                  max="4"
                  step="0.25"
                  value={capabilities.memoryFade}
                  aria-label="遗忘衰减强度"
                  oninput={(e) => setMemoryFade(Number((e.currentTarget as HTMLInputElement).value))}
                />
                <span class="preset-hint">基线 1.0</span>
                <b>×{capabilities.memoryFade.toFixed(2)}</b>
              </span>
            </div>

            <div class="cap-row cap-row-static">
              <span class="cap-icon"><Radar size={15} /></span>
              <span class="cap-text">
                <strong>好奇心强度<code class="cap-env">APEIRETH_TUNE_CURIOSITY_STRENGTH</code></strong>
                <small>好奇强度倍率（好奇日预算 = 2000 × 值）。</small>
              </span>
              <span class="cap-slider-wrap">
                <input
                  type="range"
                  min="0.25"
                  max="4"
                  step="0.25"
                  value={capabilities.curiosityStrength}
                  aria-label="好奇心强度"
                  oninput={(e) => setCuriosityStrength(Number((e.currentTarget as HTMLInputElement).value))}
                />
                <span class="preset-hint">基线 1.0</span>
                <b>×{capabilities.curiosityStrength.toFixed(2)}</b>
              </span>
            </div>

            <div class="cap-row cap-row-static">
              <span class="cap-icon"><HeartHandshake size={15} /></span>
              <span class="cap-text">
                <strong>语气情绪饱和度<code class="cap-env">APEIRETH_TUNE_TONE_SATURATION</code></strong>
                <small>语气情绪饱和度倍率（情绪注入混合 × 值；0 = 纯关系基线）。</small>
              </span>
              <span class="cap-slider-wrap">
                <input
                  type="range"
                  min="0"
                  max="2"
                  step="0.2"
                  value={capabilities.toneSaturation}
                  aria-label="语气情绪饱和度"
                  oninput={(e) => setToneSaturation(Number((e.currentTarget as HTMLInputElement).value))}
                />
                <span class="preset-hint">基线 1.0</span>
                <b>×{capabilities.toneSaturation.toFixed(2)}</b>
              </span>
            </div>

            <div class="cap-row cap-row-static">
              <span class="cap-icon"><Timer size={15} /></span>
              <span class="cap-text">
                <strong>整合节奏<code class="cap-env">APEIRETH_TUNE_CONSOLIDATION_CADENCE</code></strong>
                <small>整合节奏：每 N 回合触发一次记忆整合（1 = 每回合 = 现行为）。</small>
              </span>
              <span class="cap-slider-wrap">
                <input
                  type="range"
                  min="1"
                  max="10"
                  step="1"
                  value={capabilities.consolidationCadence}
                  aria-label="整合节奏"
                  oninput={(e) => setConsolidationCadence(Number((e.currentTarget as HTMLInputElement).value))}
                />
                <span class="preset-hint">基线 1（每回合）</span>
                <b>每 {capabilities.consolidationCadence} 回合</b>
              </span>
            </div>
          </div>

          <div class="preset-bar">
            <button class="quiet-button" onclick={() => (dispositionPresetPending = '省心')}>
              <Sparkles size={14} />
              <span>省心</span>
            </button>
            <button class="quiet-button" onclick={() => (dispositionPresetPending = '均衡')}>
              <Sparkles size={14} />
              <span>均衡</span>
            </button>
            <button class="quiet-button" onclick={() => (dispositionPresetPending = '深度记忆')}>
              <Sparkles size={14} />
              <span>深度记忆</span>
            </button>
            <button class="quiet-button" onclick={() => (dispositionPresetPending = '恢复基线')}>
              <RotateCcw size={14} />
              <span>恢复基线</span>
            </button>
            <span class="preset-hint">预设只动这 4 个体验参数；确认后立即保存并走现有应用流程。</span>
          </div>
          {#if dispositionPresetPending && pendingDispositionValues}
            <div class="notice-box preset-confirm" role="alertdialog" aria-label="应用性格预设确认">
              <span>
                {#if dispositionPresetPending === '恢复基线'}
                  将把 4 个体验参数全部拨回基线（1.0 / 1.0 / 1.0 / 每 1 回合），
                  并把「从使用中学习」关回默认关。
                {:else}
                  将把体验参数调为「{dispositionPresetPending}」档：
                  遗忘衰减 {pendingDispositionValues.memoryFade} / 好奇 {pendingDispositionValues.curiosityStrength} /
                  语气 {pendingDispositionValues.toneSaturation} / 整合每 {pendingDispositionValues.consolidationCadence} 回合；
                  其余开关与「从使用中学习」保持现状。
                {/if}
                确认后立即保存并走现有配置应用流程（配置持久化 + 注入侧车环境，配置变化时重启网关）。
              </span>
              <span class="preset-actions">
                <button class="quiet-button" onclick={confirmDispositionPreset}>保存并应用</button>
                <button class="quiet-button" onclick={() => (dispositionPresetPending = null)}>取消</button>
              </span>
            </div>
          {/if}

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Sparkles size={13} /> 从使用中学习</span>
              <span class="cap-card-count">默认关</span>
            </div>
            <button
              class="cap-row"
              onclick={() => setSelfTuning(!capabilities.selfTuning)}
              role="switch"
              aria-checked={capabilities.selfTuning}
              aria-label="从使用中学习"
            >
              <span class="cap-icon"><Sparkles size={15} /></span>
              <span class="cap-text">
                <strong>从使用中学习<code class="cap-env">APEIRETH_ENABLE_SELF_TUNING</code></strong>
                <small>
                  打开后引擎按真实使用信号自动微调体验参数（当前真接信号 = 记忆检索命中/未命中）；
                  每次自动调整可见、可撤销，日志见下方学习日志。默认关。
                </small>
              </span>
              <span class="cap-switch" class:on={capabilities.selfTuning}><span class="cap-knob"></span></span>
            </button>
          </div>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Archive size={13} /> 学习日志（自动调整记录）</span>
              <span class="preset-actions">
                <button class="quiet-button" onclick={() => void refreshTuningLog()} disabled={tuningLogLoading}>
                  <RefreshCcw size={13} />
                  <span>{tuningLogLoading ? '读取中…' : '刷新'}</span>
                </button>
              </span>
            </div>
            {#if !isDesktop() || tuningLog === null}
              <div class="notice-box">
                <Info size={14} />
                <span>学习日志仅桌面版可读（接口已备）。</span>
              </div>
            {:else if tuningLogSorted.length === 0}
              <div class="notice-box">
                <Info size={14} />
                <span>暂无自动调整记录。</span>
              </div>
            {:else}
              {#each tuningLogSorted as entry (entry.seq)}
                <div class="cap-row cap-row-static">
                  <span class="cap-icon"><SlidersHorizontal size={15} /></span>
                  <span class="cap-text">
                    <strong>
                      #{entry.seq} {tuningParamLabel(entry.param)}
                      <code class="cap-env">{entry.param}</code>
                    </strong>
                    <small>
                      {Number(entry.previous).toFixed(2)} → {Number(entry.next).toFixed(2)} ·
                      {entry.reason} · {formatTuningTime(entry.at_epoch_ms)}
                    </small>
                  </span>
                  <button class="quiet-button" onclick={() => undoTuningEntry(entry)}>撤销</button>
                </div>
              {/each}
              <div class="cap-row cap-row-static">
                <span class="cap-text">
                  <small>「撤销」= 把该参数拨回记录原值并立即保存应用（写回值走现有应用路径，不做独立撤销请求）。</small>
                </span>
              </div>
            {/if}
          </div>

          <p class="cap-footnote">
            自动调整记录由后端写入数据目录的 tuning-log.jsonl，此处只读；未接的信号不会产生记录（0 装诚实）。
          </p>
        </div>

      {:else if activeSection === 'governance'}
        <div class="setting-block">
          <h3 class="block-title">决策与治理</h3>
          <p class="block-desc">
            评审/议会深度、三洋葱权限门与子代理隔离。议会默认 3 位顾问、日常轻量运行，
            重要决策再加深。
          </p>

          <div class="cap-summary">
            <Scale size={14} />
            <span>本页已启用 <b>{enabledCount(DECISION_DEFS) + enabledCount(GUARD_DEFS)}</b> / {DECISION_DEFS.length + GUARD_DEFS.length} 项治理件</span>
          </div>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Users size={13} /> 评审与议会</span>
              <span class="cap-card-count">{enabledCount(DECISION_DEFS)}/{DECISION_DEFS.length}</span>
            </div>

            <div class="cap-row cap-row-static">
              <span class="cap-icon"><Brain size={15} /></span>
              <span class="cap-text">
                <strong>认知深度</strong>
                <small>快捷档位：轻量 ≈1.1s / 平衡 ≈2.6s / 深度 ≈13s 每轮（实测 2026-10-06）。</small>
              </span>
              <span class="cap-select-wrap">
                <select id="cognitive-depth" bind:value={cognitiveDepth} onchange={() => applyCognitiveDepth()}>
                  <option value="light">轻量</option>
                  <option value="balanced">平衡</option>
                  <option value="deep">深度</option>
                  <option value="custom">自定义</option>
                </select>
              </span>
            </div>

            {#each DECISION_DEFS as def (def.key)}
              <button
                class="cap-row"
                class:dim={capDisabled(def)}
                onclick={() => toggleCap(def)}
                role="switch"
                aria-checked={isCapOn(def)}
                aria-label={def.label}
              >
                <span class="cap-icon"><def.icon size={15} /></span>
                <span class="cap-text">
                  <strong>{def.label}<code class="cap-env">{def.env}</code></strong>
                  <small>{def.desc}</small>
                </span>
                <span class="cap-switch" class:on={isCapOn(def)}><span class="cap-knob"></span></span>
              </button>
            {/each}

            {#if capabilities.council}
              <div class="cap-row cap-row-static">
                <span class="cap-icon"><Users size={15} /></span>
                <span class="cap-text">
                  <strong>顾问数量<code class="cap-env">APEIRETH_COUNCIL_ADVISORS</code></strong>
                  <small>1-7 位，Safety 恒首位；默认 3（7→3 收敛批）。</small>
                </span>
                <span class="cap-slider-wrap">
                  <input
                    type="range"
                    min="1"
                    max="7"
                    step="1"
                    value={capabilities.councilAdvisors}
                    aria-label="顾问数量"
                    oninput={(e) => setCouncilAdvisors(Number((e.currentTarget as HTMLInputElement).value))}
                  />
                  <b>{capabilities.councilAdvisors}</b>
                </span>
              </div>
              <div class="cap-row cap-row-static">
                <span class="cap-icon"><Timer size={15} /></span>
                <span class="cap-text">
                  <strong>单顾问超时 (ms)<code class="cap-env">APEIRETH_COUNCIL_TIMEOUT_MS</code></strong>
                  <small>默认 30000；思考型模型延迟高时可放宽。</small>
                </span>
                <span class="cap-slider-wrap">
                  <input
                    type="number"
                    min="1000"
                    step="5000"
                    value={capabilities.councilTimeoutMs}
                    aria-label="单顾问超时毫秒"
                    oninput={(e) => setCouncilTimeout(Number((e.currentTarget as HTMLInputElement).value))}
                  />
                </span>
              </div>
            {/if}
          </div>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><ShieldCheck size={13} /> 权限与子代理</span>
              <span class="cap-card-count">{enabledCount(GUARD_DEFS)}/{GUARD_DEFS.length}</span>
            </div>
            {#each GUARD_DEFS as def (def.key)}
              <button
                class="cap-row"
                class:dim={capDisabled(def)}
                onclick={() => toggleCap(def)}
                role="switch"
                aria-checked={isCapOn(def)}
                aria-label={def.label}
              >
                <span class="cap-icon"><def.icon size={15} /></span>
                <span class="cap-text">
                  <strong>{def.label}<code class="cap-env">{def.env}</code></strong>
                  <small>{def.desc}</small>
                </span>
                <span class="cap-switch" class:on={isCapOn(def)}><span class="cap-knob"></span></span>
              </button>
            {/each}
          </div>

          <div class="cap-default-card">
            <span class="cap-default-badge note"><Info size={11} /> 设计说明 · 非开关</span>
            <strong class="cap-default-title">议会语义（2026-10-10 改造批）</strong>
            <p class="cap-default-text">
              议会是决策环节的顾问团，不是每轮评审器：升级/部署批准、高危操作、
              待裁冲突批量裁决时启用，日常对话保持轻量。未配 LLM 时 fail-loud 不造假裁决。
            </p>
          </div>
        </div>

      {:else if activeSection === 'tools'}
        <div class="setting-block">
          <h3 class="block-title">工具与安全</h3>
          <p class="block-desc">
            工具权限授予 + 沙箱边界。开启工具 ≠ 无审批执行——shell 每次调用仍走审批卡。
          </p>

          <div class="cap-summary">
            <Wrench size={14} />
            <span>本页已授予 <b>{enabledCount(TOOL_DEFS)}</b> / {TOOL_DEFS.length} 类工具 · 沙箱{capabilities.shellSandbox ? '开' : '关'}</span>
          </div>

          <div class="form-group">
            <label for="permission-preset">全局权限预设</label>
            <select id="permission-preset" bind:value={permissionPreset}>
              <option value="read_only">read_only — 只读</option>
              <option value="standard">standard — 标准 (推荐)</option>
              <option value="full">full — 完全权限</option>
            </select>
            <small class="field-hint">作为新会话默认 (已生效)：仅对新会话生效，已有会话请在会话内改。</small>
          </div>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Terminal size={13} /> 工具授予</span>
              <span class="cap-card-count">{enabledCount(TOOL_DEFS)}/{TOOL_DEFS.length}</span>
            </div>
            {#each TOOL_DEFS as def (def.key)}
              <button
                class="cap-row"
                class:dim={capDisabled(def)}
                onclick={() => toggleCap(def)}
                role="switch"
                aria-checked={isCapOn(def)}
                aria-label={def.label}
              >
                <span class="cap-icon"><def.icon size={15} /></span>
                <span class="cap-text">
                  <strong>{def.label}<code class="cap-env">{def.env}</code></strong>
                  <small>{def.desc}</small>
                </span>
                <span class="cap-switch" class:on={isCapOn(def)}><span class="cap-knob"></span></span>
              </button>

              {#if def.key === 'shell' && capabilities.shell}
                <button
                  class="cap-row cap-row-nested"
                  class:sandbox-off={!capabilities.shellSandbox}
                  onclick={() => handleCapabilityToggle('shellSandbox', !capabilities.shellSandbox)}
                  role="switch"
                  aria-checked={capabilities.shellSandbox}
                  aria-label="AppContainer 沙箱执行"
                >
                  <span class="cap-icon">
                    {#if capabilities.shellSandbox}
                      <ShieldCheck size={15} />
                    {:else}
                      <ShieldOff size={15} />
                    {/if}
                  </span>
                  <span class="cap-text">
                    <strong>AppContainer 沙箱执行<code class="cap-env">APEIRETH_SHELL_SANDBOX</code></strong>
                    <small>
                      断网 + 用户空间零访问（双探针实证，默认开）。关闭 = 显式裸跑，
                      命令获得你的完整用户权限——危险操作会红标提醒。
                    </small>
                  </span>
                  <span class="cap-switch" class:on={capabilities.shellSandbox}><span class="cap-knob"></span></span>
                </button>
                {#if !capabilities.shellSandbox}
                  <div class="cap-warning">
                    <AlertTriangle size={13} />
                    <span>沙箱已关闭：shell 命令将以你的完整用户权限裸跑，请确认你信任正在运行的任务。</span>
                  </div>
                {/if}
              {/if}
            {/each}
          </div>

          <div class="cap-default-card">
            <span class="cap-default-badge on"><CheckCircle2 size={11} /> 常驻 · 默认运行</span>
            <strong class="cap-default-title">权限洋葱与即时授权 (On-demand Permission Pack)</strong>
            <p class="cap-default-text">
              这是始终在线的安全机制，不是可关的开关：Master Token 绝不持久化保存在客户端存储中；
              特权工具被拒绝产生待批请求时，在「工具管理」页面输入 Token 即时完成时效性签发。
            </p>
          </div>

          <div class="cap-default-card">
            <span class="cap-default-badge on"><CheckCircle2 size={11} /> 常驻 · 默认运行</span>
            <strong class="cap-default-title">宪法评审 (MiniMaxConstitutionLlm)</strong>
            <p class="cap-default-text">
              高危工具执行前自动按 E 层进行安全判案——默认运行、不可关闭，
              杜绝越权或有害操作。
            </p>
          </div>
        </div>

      {:else if activeSection === 'runtime'}
        <div class="setting-block">
          <h3 class="block-title">运行时诊断</h3>
          <p class="block-desc">实时探测后端网关、模型服务、会话账本与记忆流。</p>

          <button class="quiet-button" onclick={checkDiagnostics} disabled={checkingRuntime}>
            <RotateCcw size={13} class={checkingRuntime ? 'spin' : ''} />
            <span>{checkingRuntime ? '正在诊断…' : '立即执行深度诊断'}</span>
          </button>

          {#if runtimeReport}
            <div class="diag-results">
              <div class="diag-summary">
                <span>总体状态: <b>{runtimeReport.overall}</b></span>
                <span>总延迟: <b>{runtimeReport.latencyMs}ms</b></span>
              </div>
              <div class="diag-list">
                {#each runtimeReport.subsystems as sub}
                  <div class="diag-item">
                    <span>{sub.name} (<code>{sub.endpoint}</code>)</span>
                    <StatusBadge
                      label={sub.status === 'ok' ? '正常' : sub.status === 'degraded' ? '降级' : '离线'}
                      variant={sub.status === 'ok' ? 'green' : 'danger'}
                      size="small"
                    />
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>

      {:else if activeSection === 'data'}
        <div class="setting-block">
          <h3 class="block-title">数据与本地缓存</h3>
          <p class="block-desc">管理客户端本地存储的会话与配置缓存。</p>

          <div class="info-card">
            <strong class="info-title">工作区目录</strong>
            <div class="workspace-row">
              <code class="workspace-path">{workspaceDir || '默认 (应用数据目录)'}</code>
              <button class="quiet-button" onclick={() => void openWorkspacePicker()} disabled={workspaceLoading}>
                <Folder size={13} />
                <span>更改</span>
              </button>
            </div>
            {#if workspaceError}
              <p class="field-hint error-hint">{workspaceError}</p>
            {/if}
            <p class="field-hint">新路径在下次启动 sidecar 时生效。</p>
          </div>

          <div class="danger-zone-box">
            <div class="danger-head">
              <AlertTriangle size={16} class="danger-icon" />
              <strong>危险区域 (Danger Zone)</strong>
            </div>
            <p class="danger-desc">清空本地数据将删除浏览器/客户端中存储的会话历史。后端数据库中的长期记忆不会受影响。</p>
            <button class="danger-button" onclick={() => showClearConfirm = true}>
              <Trash2 size={13} />
              <span>清空本地会话数据</span>
            </button>
          </div>
        </div>

      {:else}
        <div class="setting-block">
          <h3 class="block-title">开发者选项</h3>
          <p class="block-desc">
            Beta 功能试验场与运行时契约。Beta 件默认关、随时可撤，验证稳定后晋升正式设置页；
            改动同样走「保存设置」注入侧车环境。
          </p>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Sparkles size={13} /> Beta 功能</span>
              <span class="cap-card-count">{enabledCount(BETA_DEFS)}/{BETA_DEFS.length}</span>
            </div>
            {#each BETA_DEFS as def (def.key)}
              <button
                class="cap-row"
                class:dim={capDisabled(def)}
                onclick={() => toggleCap(def)}
                role="switch"
                aria-checked={isCapOn(def)}
                aria-label={def.label}
              >
                <span class="cap-icon"><def.icon size={15} /></span>
                <span class="cap-text">
                  <strong>{def.label}<code class="cap-env">{def.env}</code></strong>
                  <small>{def.desc}</small>
                </span>
                <span class="cap-switch" class:on={isCapOn(def)}><span class="cap-knob"></span></span>
              </button>
            {/each}

            {#if capabilities.reasoningEnabled}
              <div class="cap-row cap-row-static">
                <span class="cap-icon"><Filter size={15} /></span>
                <span class="cap-text">
                  <strong>生效模型过滤器<code class="cap-env">APEIRETH_REASONING_MODEL_FILTERS</code></strong>
                  <small>逗号分隔子串白名单（如 deepseek,o3）；留空 = 全部模型生效。</small>
                </span>
                <span class="cap-input-wrap">
                  <input
                    type="text"
                    value={capabilities.reasoningModelFilters}
                    placeholder="全部模型"
                    aria-label="思考模式模型过滤器"
                    oninput={(e) => setReasoningFilters((e.currentTarget as HTMLInputElement).value)}
                  />
                </span>
              </div>
              <div class="cap-row cap-row-static">
                <span class="cap-icon"><Tag size={15} /></span>
                <span class="cap-text">
                  <strong>reasoning 标签<code class="cap-env">APEIRETH_REASONING_TAG</code></strong>
                  <small>reasoning_content 的字段标签名，默认 think。</small>
                </span>
                <span class="cap-input-wrap">
                  <input
                    type="text"
                    value={capabilities.reasoningTag}
                    aria-label="reasoning 标签"
                    oninput={(e) => setReasoningTag((e.currentTarget as HTMLInputElement).value)}
                  />
                </span>
              </div>
            {/if}
          </div>

          <div class="cap-default-card">
            <span class="cap-default-badge note"><Info size={11} /> 契约 · 恒成立</span>
            <strong class="cap-default-title">Agent Runtime Contract (§15)</strong>
            <p class="cap-default-text">
              UI 仅面对标准事件流 (run-start, text-delta, reasoning-delta, tool-call, tool-result,
              message-end)，不裸碰底层 HTTP/SSE 协议。
            </p>
          </div>

          <div class="form-group">
            <label for="raw-config-json">客户端配置 (JSON)</label>
            <div class="code-box-wrap">
              <button class="quiet-button copy-btn" onclick={() => void copyConfigJson()}>
                {#if configJsonCopied}
                  <Check size={12} />
                  <span>已复制</span>
                {:else}
                  <Copy size={12} />
                  <span>复制</span>
                {/if}
              </button>
              <pre class="code-box">{configJsonPreview}</pre>
            </div>
          </div>
        </div>
      {/if}

      <!-- 2026-09-23 主人指示：保存栏移入内容流末尾（每个分页最下面），
           不再做通栏固定黑带（2549 宽屏上比例失调且不优雅）。
           保存会重启本地网关以应用配置。
           2026-10-10 主人反馈：纯探测页（运行时诊断）不需要保存栏。 -->
      {#if activeSection !== 'runtime'}
        <div class="settings-save-bar">
          <span class="save-bar-hint">
            {saveSuccess ? '✓ 已保存，本地网关已应用新配置' : '填好配置后点"保存设置"（无重启热应用，不支持时自动重启网关）'}
          </span>
          <button class="primary-button save-bar-btn" onclick={handleSaveSettings}>
            <Check size={14} />
            <span>{saveSuccess ? '已保存！' : '保存设置'}</span>
          </button>
        </div>
      {/if}
    </div>
  </div>
</section>

<!-- API Key Edit Modal for Gateway -->
{#if showApiKeyModal}
  <div class="modal-backdrop" onclick={() => showApiKeyModal = false} role="presentation">
    <div
      class="modal-dialog"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="api-key-dialog-title"
    >
      <div class="modal-header">
        <h3 id="api-key-dialog-title">配置模型 API 密钥</h3>
      </div>
      <div class="modal-body">
        <p class="modal-desc">
          这是**模型提供商**的密钥（如 DeepSeek）。密钥存入系统钥匙串，不落盘明文；保存后热更新到本地网关，无需重启。
        </p>

        <div class="keychain-status">
          {#if keychainLoading}
            <span class="keychain-muted">正在读取系统钥匙串…</span>
          {:else if storedKeyExists}
            <span class="keychain-stored">{storedKeyMasked} (已存钥匙串)</span>
          {:else}
            <span class="keychain-muted">系统钥匙串中暂无该提供商的密钥</span>
          {/if}
        </div>

        <div class="form-group">
          <input
            type="password"
            placeholder="输入新的 API Key（例如 sk-…）"
            bind:value={tempApiKey}
            autocomplete="off"
          />
        </div>

        {#if keychainActionError}
          <p class="modal-error">{keychainActionError}</p>
        {/if}
      </div>
      <div class="modal-footer">
        {#if storedKeyExists}
          <button
            class="quiet-button danger-text delete-key-btn"
            onclick={deleteStoredKey}
            disabled={keychainDeleting}
          >
            <Trash2 size={13} />
            <span>{keychainDeleting ? '删除中…' : '删除已存密钥'}</span>
          </button>
        {/if}
        <button class="quiet-button" onclick={() => showApiKeyModal = false}>取消</button>
        <button class="primary-button" onclick={saveNewApiKey} disabled={keychainSaving}>
          {keychainSaving ? '保存中…' : '保存并应用'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Workspace Directory Picker -->
<WorkspacePickerModal
  open={showWorkspacePicker}
  current={workspaceDir}
  suggestions={workspaceSuggestions}
  onPick={(dir) => void handleWorkspacePick(dir)}
  onCancel={() => (showWorkspacePicker = false)}
/>

<!-- Clear Data Confirmation -->
<ConfirmDialog
  open={showClearConfirm}
  title="清空本地所有会话"
  message="确定要清空本地保存的所有会话记录吗？此操作无法撤销。"
  confirmText="确认清空"
  danger={true}
  onConfirm={() => {
    showClearConfirm = false;
    if (onClearLocalData) onClearLocalData();
  }}
  onCancel={() => showClearConfirm = false}
/>

<style>
  .settings-save-bar {
    /* 2026-09-23 主人指示：从 .settings-layout 通栏带改为内容流末尾的保存卡片
       （每个分页最下面）。不再跨 grid 列，比例随 900px 内容列收敛。 */
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    margin-top: 28px;
    padding: 12px 18px;
    border: 1px solid var(--line, #2a323c);
    border-radius: 10px;
    /* --surface-2：根（深）/essence（浅）双模式都有定义——曾用 --surface-1
       （无任何主题定义），浅色模式下跌回深色硬编码，浅页面上出现深色保存栏（主人
       2026-09-23 真机指出「浅色模式 + 深色保存设置很违和」）。 */
    background: var(--surface-2, #1b1a20);
  }
  .save-bar-hint {
    font-size: 12px;
    color: var(--faint, #8b97a5);
  }
  .save-bar-btn {
    font-weight: 600;
  }

  .settings-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .settings-layout {
    flex: 1;
    display: grid;
    grid-template-columns: 200px 1fr;
    min-height: 0;
  }
  .settings-subnav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 16px 12px;
    border-right: 1px solid var(--line);
    background: var(--surface);
  }
  .subnav-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border-radius: 6px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .subnav-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .subnav-btn.active {
    background: var(--amber-wash);
    color: var(--amber);
    font-weight: 500;
  }

  .settings-content {
    overflow-y: auto;
    padding: 24px 36px 48px;
    max-width: 900px;
  }
  .setting-block {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .block-title {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
  }
  .block-desc {
    margin: -10px 0 6px;
    font-size: 13px;
    color: var(--muted);
  }

  /* Provider summary */
  .provider-summary {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
    border-radius: 10px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    flex-wrap: wrap;
  }
  .summary-main {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .summary-badge {
    align-self: flex-start;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--amber-wash);
    color: var(--amber);
    font-weight: 600;
  }
  .summary-main strong {
    font-size: 15px;
    color: var(--text);
  }
  .summary-model {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--muted);
  }
  .summary-side {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    min-width: 0;
  }
  .summary-url {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status-pill {
    font-size: 11px;
    padding: 3px 10px;
    border-radius: 999px;
    border: 1px solid var(--line);
    color: var(--muted);
    background: var(--surface);
  }
  .status-pill.ok {
    border-color: rgba(61, 122, 92, 0.35);
    color: var(--green);
    background: var(--green-wash);
  }
  .status-pill.warn {
    border-color: rgba(168, 72, 64, 0.35);
    color: var(--danger);
    background: rgba(168, 72, 64, 0.08);
  }
  .status-pill.idle {
    color: var(--faint);
  }

  /* Config steps */
  .config-step {
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }
  .step-num {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 12px;
    font-weight: 600;
    background: var(--amber-wash);
    color: var(--amber);
    border: 1px solid var(--amber-line);
    margin-top: 2px;
  }
  .step-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .step-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .form-row-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  @media (max-width: 720px) {
    .form-row-2 {
      grid-template-columns: 1fr;
    }
  }
  .model-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .disc-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 8px;
  }
  .disc-search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    border-radius: 7px;
    border: 1px solid var(--line);
    background: var(--surface);
    min-width: 180px;
  }
  .disc-search input {
    border: 0;
    outline: 0;
    background: transparent;
    font-size: 12px;
    color: var(--text);
    width: 140px;
  }
  .model-catalog {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
  }
  .catalog-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border: 0;
    border-bottom: 1px solid var(--line);
    background: transparent;
    color: var(--text);
    font-family: var(--mono);
    font-size: 11.5px;
    text-align: left;
    cursor: pointer;
  }
  .catalog-item:last-child {
    border-bottom: 0;
  }
  .catalog-item:hover {
    background: var(--surface-2);
  }
  .catalog-item.selected {
    background: var(--amber-wash);
    color: var(--amber);
  }
  .catalog-empty {
    margin: 0;
    padding: 16px;
    text-align: center;
    font-size: 12px;
    color: var(--faint);
  }

  /* Protocol Tabs */
  .protocol-tabs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-bottom: 6px;
  }
  .protocol-tab {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 14px;
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-radius: 9px;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
    color: var(--muted);
  }
  .protocol-tab:hover {
    border-color: var(--amber-line);
    color: var(--text);
  }
  .protocol-tab.selected {
    background: var(--amber-wash);
    border-color: var(--amber-line);
    color: var(--amber);
  }
  .proto-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .proto-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .protocol-tab.selected .proto-title {
    color: var(--amber);
  }
  .proto-sub {
    font-size: 11px;
    color: var(--muted);
    line-height: 1.3;
  }

  /* Presets Row */
  .presets-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .preset-chip {
    padding: 6px 12px;
    border-radius: 7px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .preset-chip:hover {
    border-color: var(--amber-line);
    color: var(--text);
  }
  .preset-chip.selected {
    background: var(--amber-wash);
    border-color: var(--amber-line);
    color: var(--amber);
    font-weight: 600;
  }

  /* Form controls */
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form-group label,
  .form-group .group-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
  }
  .form-group input {
    padding: 8px 12px;
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    color: var(--text);
    font-size: 13px;
    outline: 0;
  }
  .form-group input:focus {
    border-color: var(--amber-line);
  }
  .field-hint {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.4;
  }

  /* Key input with show/hide toggle */
  .key-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }
  .key-input-wrapper input {
    width: 100%;
    padding-right: 36px;
  }
  .key-toggle-btn {
    position: absolute;
    right: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--muted);
    cursor: pointer;
    padding: 4px;
  }
  .key-toggle-btn:hover {
    color: var(--text);
  }

  .credential-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 7px;
  }
  .cred-status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text);
  }
  .chip-label {
    font-size: 11px;
    color: var(--muted);
  }
  .model-chip {
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    color: var(--muted);
    font-size: 11px;
    font-family: var(--mono);
    cursor: pointer;
  }
  .model-chip:hover {
    border-color: var(--amber-line);
    color: var(--amber);
  }
  .model-chip.selected {
    background: var(--amber-wash);
    border-color: var(--amber-line);
    color: var(--amber);
  }

  /* Test Connection Banner */
  .test-result-box {
    padding: 12px 14px;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
  }
  .test-result-box.success {
    background: rgba(46, 204, 113, 0.08);
    border: 1px solid rgba(46, 204, 113, 0.3);
  }
  .test-result-box.failed {
    background: rgba(231, 76, 60, 0.08);
    border: 1px solid rgba(231, 76, 60, 0.3);
  }
  .test-result-head {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  :global(.success-icon) {
    color: #2ecc71;
  }
  :global(.failed-icon) {
    color: #e74c3c;
  }
  .latency-badge {
    margin-left: auto;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
  }
  .test-result-msg {
    margin: 0;
    color: var(--muted);
    line-height: 1.4;
  }
  .discovered-models {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 4px;
    border-top: 1px solid var(--line);
    padding-top: 6px;
  }
  .disc-label {
    font-size: 11px;
    color: var(--muted);
  }
  .more-models {
    font-size: 11px;
    color: var(--faint);
    align-self: center;
  }

  /* Advanced Gateway Box */
  .advanced-box {
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
    margin-top: 6px;
  }
  .advanced-toggle {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--surface-2);
    border: none;
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
  }
  .advanced-toggle:hover {
    color: var(--text);
  }
  .adv-left {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
  }
  .advanced-body {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background: var(--surface);
    border-top: 1px solid var(--line);
  }

  .info-card {
    padding: 12px 14px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  /* ===== 能力中心 · 微信/QQ 式分组卡片（2026-10-10 W2/W3 收官批）===== */
  .cap-summary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 14px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--surface);
    font-size: 12px;
    color: var(--muted);
  }
  .cap-summary b {
    color: var(--amber);
    font-family: var(--mono);
    font-weight: 600;
  }

  /* 「推荐配置」一键预设：按钮 + 说明 + 确认面板（能力中心）。 */
  .preset-bar {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .preset-hint {
    font-size: 12px;
    color: var(--muted);
  }
  .preset-confirm {
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }
  .preset-actions {
    display: flex;
    gap: 8px;
  }

  .cap-card {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .cap-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
  }
  .cap-card-title {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .cap-card-count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--faint);
  }

  /* 行：图标 + 标题/副文案 + 右侧控制。整行可点（微信设置项语义）。 */
  .cap-row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 11px 14px;
    border: 0;
    border-bottom: 1px solid var(--line);
    background: transparent;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s ease;
    color: var(--text);
  }
  .cap-row:last-child {
    border-bottom: 0;
  }
  .cap-row:hover {
    background: var(--surface);
  }
  .cap-row.dim {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .cap-row-nested {
    padding-left: 40px;
    background: color-mix(in srgb, var(--surface) 40%, transparent);
  }
  .cap-row.sandbox-off {
    background: rgba(168, 72, 64, 0.07);
  }
  .cap-row-static {
    cursor: default;
  }
  .cap-row-static:hover {
    background: transparent;
  }

  .cap-icon {
    flex: none;
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--muted);
  }
  .cap-row-nested .cap-icon {
    width: 26px;
    height: 26px;
  }

  .cap-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .cap-text strong {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .cap-text small {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
  }
  /* 工程诚实：每个旋钮如实标注后端 env 名，等宽弱色芯片，不抢视觉。 */
  .cap-env {
    font-family: var(--mono);
    font-size: 9.5px;
    font-weight: 400;
    color: var(--faint);
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 1px 5px;
    letter-spacing: 0.02em;
  }

  /* 开关（微信式圆钮） */
  .cap-switch {
    position: relative;
    flex: none;
    width: 36px;
    height: 20px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    transition: background 0.18s ease, border-color 0.18s ease;
  }
  .cap-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--muted);
    transition: left 0.18s ease, background 0.18s ease;
  }
  .cap-switch.on {
    background: var(--accent, #6ea8fe);
    border-color: transparent;
  }
  .cap-switch.on .cap-knob {
    left: 18px;
    background: #fff;
  }

  /* 数值行（滑杆 / 数字输入） */
  .cap-slider-wrap {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .cap-slider-wrap input[type='range'] {
    width: 110px;
    accent-color: var(--accent, #6ea8fe);
  }
  .cap-slider-wrap input[type='number'] {
    width: 90px;
    padding: 6px 8px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    color: var(--text);
    font-size: 12px;
    font-family: var(--mono);
    outline: 0;
  }
  .cap-slider-wrap input[type='number']:focus {
    border-color: var(--amber-line);
  }
  .cap-slider-wrap b {
    font-family: var(--mono);
    font-size: 12px;
    min-width: 30px;
    text-align: right;
    color: var(--text);
  }
  .cap-select-wrap select {
    padding: 6px 10px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    color: var(--text);
    font-size: 12px;
    outline: 0;
  }

  .cap-warning {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 14px 12px 40px;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid rgba(168, 72, 64, 0.35);
    background: rgba(168, 72, 64, 0.08);
    color: var(--danger);
    font-size: 11.5px;
    line-height: 1.5;
  }

  /* 默认能力卡：常驻/显式触发机制的说明卡——明确「这不是开关」。
     主人 2026-10-10 反馈：静态 info-card 与可操控开关行同形，易误以为坏了；
     改为带徽章的说明卡 + 页脚注，与开关卡在视觉上明确分层。 */
  .cap-default-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
    border: 1px dashed var(--line-strong);
    border-radius: 10px;
    background: transparent;
  }
  .cap-default-badge {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--line);
    color: var(--faint);
  }
  .cap-default-badge.on {
    border-color: rgba(61, 122, 92, 0.35);
    color: var(--green);
    background: var(--green-wash);
  }
  .cap-default-badge.note {
    border-color: var(--line-strong);
    color: var(--muted);
  }
  .cap-default-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .cap-default-text {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.6;
  }
  .cap-default-text code {
    font-family: var(--mono);
    font-size: 11px;
  }

  .cap-footnote {
    margin: -6px 2px 0;
    font-size: 11px;
    color: var(--faint);
    line-height: 1.6;
  }

  .cap-input-wrap input {
    width: 170px;
    padding: 6px 10px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    color: var(--text);
    font-size: 12px;
    font-family: var(--mono);
    outline: 0;
  }
  .cap-input-wrap input:focus {
    border-color: var(--amber-line);
  }

  .code-box-wrap {
    position: relative;
  }
  .code-box-wrap .copy-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 1;
  }
  /* 多 Agent 人设卡片 */
  .persona-card {
    padding: 14px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 9px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .persona-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .persona-index {
    font-size: 11px;
    letter-spacing: 0.12em;
    color: var(--faint);
  }
  .persona-card-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .quiet-button.selected {
    background: var(--amber-wash);
    border-color: var(--amber-line);
    color: var(--amber);
    font-weight: 600;
  }
  .danger-text { color: var(--danger); }
  .form-group textarea {
    padding: 8px 12px;
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    color: var(--text);
    font-size: 12px;
    line-height: 1.6;
    font-family: inherit;
    resize: vertical;
    outline: 0;
  }
  .form-group textarea:focus { border-color: var(--amber-line); }
  .info-title {
    font-size: 13px;
    color: var(--text);
  }
  .notice-box {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: rgba(231, 162, 59, 0.08);
    border: 1px solid var(--amber-line);
    border-radius: 7px;
    font-size: 12px;
    color: var(--muted);
  }

  .diag-results {
    padding: 14px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .diag-summary {
    display: flex;
    gap: 20px;
    font-size: 12px;
    color: var(--muted);
    border-bottom: 1px solid var(--line);
    padding-bottom: 8px;
  }
  .diag-summary b {
    color: var(--amber);
    font-family: var(--mono);
  }
  .diag-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .diag-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text);
  }
  .diag-item code {
    font-family: var(--mono);
    color: var(--faint);
  }

  .danger-zone-box {
    padding: 16px;
    background: rgba(224, 91, 80, 0.08);
    border: 1px solid rgba(224, 91, 80, 0.35);
    border-radius: 9px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .danger-head {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--danger);
  }
  .danger-desc {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.5;
  }
  .danger-button {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    border-radius: 6px;
    background: var(--danger);
    border: 1px solid var(--danger);
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .code-box {
    margin: 0;
    padding: 10px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 7px;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
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
    width: 100%;
    max-width: 420px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 12px;
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .modal-header {
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
    background: var(--surface-2);
  }
  .modal-header h3 {
    margin: 0;
    font-size: 14px;
    color: var(--text);
  }
  .modal-body {
    padding: 16px 18px;
  }
  .modal-desc {
    margin: 0 0 12px;
    font-size: 12px;
    color: var(--muted);
  }
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--line);
    background: var(--surface-2);
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  /* ---- wave-2 设置页新增样式 ---- */
  .keychain-status {
    padding: 8px 10px;
    border-radius: 7px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    margin-bottom: 12px;
    font-size: 12px;
  }
  .keychain-stored {
    color: var(--green);
    font-family: var(--mono);
  }
  .keychain-muted {
    color: var(--faint);
  }
  .modal-error {
    margin: 10px 0 0;
    font-size: 12px;
    color: var(--danger);
    line-height: 1.5;
  }
  .delete-key-btn {
    margin-right: auto;
  }
  .apply-status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 7px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    font-size: 12px;
    color: var(--muted);
  }
  .warnings-box {
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--amber-line);
    background: var(--amber-wash);
    font-size: 12px;
    color: var(--muted);
  }
  .warnings-box strong {
    color: var(--amber);
    display: block;
    margin-bottom: 6px;
  }
  .warnings-box ul {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .warnings-box li {
    line-height: 1.5;
  }
  .effective-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 16px;
    margin-top: 8px;
  }
  .effective-grid span {
    font-size: 12px;
    color: var(--muted);
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .effective-grid code {
    font-family: var(--mono);
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .model-picker-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .workspace-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 6px;
  }
  .workspace-path {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .error-hint {
    color: var(--danger);
  }
  .form-group select {
    padding: 8px 12px;
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    color: var(--text);
    font-size: 13px;
    outline: 0;
  }
  .form-group select:focus {
    border-color: var(--amber-line);
  }
</style>
