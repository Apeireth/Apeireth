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
    FilePenLine,
    FileCheck2,
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
    Gauge,
  } from 'lucide-svelte';
  import PageHeader from '../../components/PageHeader.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import ThemeSettingsPanel from '../components/ThemeSettingsPanel.svelte';
  import SessionModelPicker from '../components/SessionModelPicker.svelte';
  import WorkspacePickerModal from '../components/WorkspacePickerModal.svelte';
  import ErrorSolutionBanner from '../components/ErrorSolutionBanner.svelte';
  import GovernanceView, {type GovernanceTabId} from './GovernanceView.svelte';
  import type {ApeirethConfig, CapabilityManifest, RuntimeHealthReport, ProviderProtocol, ProviderConfig, PersonaProfile, CapabilityToggles, ModelInfo, AdminConfigPatch} from '../types';
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
  import {configWithCapabilities, toggledCapability} from '../capability-apply';
  import type {CapabilityToggleKey} from '../capability-apply';
  import {
    DANGER_ACTION_CONFIRMATIONS,
    configWithGatewayUrl,
    configWithPersonas,
    configWithProviderGroup,
    focusLeavesGroup,
    isSameAsCommitted,
    isTextCommitKey,
    personasSnapshot,
    providerGroupSnapshot,
    textCommitFlag,
  } from '../settings-live-apply';
  import type {DangerActionKey, ProviderGroupDraft, TextCommitFlag} from '../settings-live-apply';
  import {
    BUDGET_EXHAUSTION,
    BUDGET_KNOB_SPECS,
    QUOTA_DIMENSIONS,
    QUOTA_DIMENSION_STATUS_LABEL,
    budgetRemainingRows,
    effectiveBudget,
    resolveBudgetLimitInput,
  } from '../budget';
  import type {BudgetKnobKey} from '../budget';
  import {emptySessionTotals, formatSessionTotals} from '../chat-shell/turn-telemetry';
  import type {SessionUsageTotals} from '../chat-shell/turn-telemetry';

  let {
    config,
    onSave,
    onClearLocalData,
    capabilityManifest = null,
    initialSection = 'appearance',
    initialGovernanceTab = 'approvals',
    governanceKey = 0,
    onGovernanceOpenChat,
    sessionUsage = null,
  }: {
    config: ApeirethConfig;
    /** 返回 apply 的 Promise：推送失败时拒绝，供开关即点即生效做失败回滚。 */
    onSave: (newConfig: ApeirethConfig) => void | Promise<void>;
    onClearLocalData?: () => void;
    /** 运行时能力清单（「安全与治理」面板取数门用；治理面板原样搬入，不重写）。 */
    capabilityManifest?: CapabilityManifest | null;
    /** 壳层落区：命令面板 / 旧深链重定向指定的初始分区。 */
    initialSection?: SettingsSection;
    /** 「安全与治理」初始 tab（状态条守卫计数入口指令式落 tab）。 */
    initialGovernanceTab?: GovernanceTabId;
    /** 治理面板重挂载钥匙：外部指令式换 tab 时递增。 */
    governanceKey?: number;
    /** 治理空态引导卡的「回到对话」动作。 */
    onGovernanceOpenChat?: () => void;
    /** 会话累计消耗（App 的 running totals；预算仪表取数面）。 */
    sessionUsage?: SessionUsageTotals | null;
  } = $props();

  type SettingsSection =
    | 'appearance'
    | 'models'
    | 'personality'
    | 'cognition'
    | 'disposition'
    | 'governance'
    | 'security'
    | 'tools'
    | 'budget'
    | 'runtime'
    | 'data'
    | 'developer';

  let activeSection = $state<SettingsSection>('appearance');

  // 壳层导航落区（侧栏收纳批）：外部改 initialSection 即换区（不猜、不静默）。
  $effect(() => {
    const next = initialSection;
    if (next) activeSection = next;
  });

  // Gateway backend fields
  let editBaseUrl = $state('');
  let showAdvancedGateway = $state(false);

  // Backend advanced-capability toggles (fail-closed defaults, applied live)
  // 草稿语义：只取 config 初值快照，untrack 显式声明不跟踪；改动即点/失焦
  // 即走 apply 路径回写，没有「稍后一起提交」的隐藏草稿。
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
    let next = capabilities;
    if (cognitiveDepth === 'balanced') {
      next = {...capabilities, judge: true, council: false};
    } else if (cognitiveDepth === 'deep') {
      next = {...capabilities, judge: true, council: true};
    } else if (cognitiveDepth === 'light') {
      next = {...capabilities, judge: false, council: false};
    } else {
      // custom: 用户手动改 judge/council，保持现状。
      return;
    }
    capabilities = next;
    // 档位即预设（第 1 级）：点选即生效，失败回填 judge/council 旧值 + 横幅。
    void applyKnobNow(['judge', 'council'], next);
  }

  // ---- 三级即效语义：改动即点/失焦即生效，页面没有「保存」按钮 ----
  // 设置页是「即点即生效」的视觉语言：用户不会想到还要点远处的确认按钮。
  // 每类控件只有一条生效时机，全部走同一条 apply 路径（onSave → 配置持久化
  // + env 注入 + 侧车重启/热应用）：
  //   1. 开关 / 滑杆 / 预设：点（或松手）即生效；
  //   2. 文本输入：失焦或回车提交（服务商组整组提交，避免半截配置）；
  //   3. 危险动作：即点 + 二次确认弹层，确认后即效。
  // 乐观更新 UI + 本项 pending 态（侧车重启时行内「正在应用到运行时…」）；
  // 推送失败把该项拨回原值并亮错误横幅。
  let liveApplyPendingKey = $state<string | null>(null);
  let liveApplyError = $state<{code?: string; message: string; solution?: string} | null>(null);
  // 改动序号：失败回滚只对「最后一次在途改动」生效，后改的项不被先改的
  // 失败回滚覆盖。
  let liveApplySeq = 0;
  // 上次成功应用的能力集快照：滑杆/预设失败回填的旧值来源。
  let appliedCapabilities = $state<CapabilityToggles>(
    untrack(() => ({
      ...DEFAULT_CAPABILITY_TOGGLES,
      ...(config.capabilities ?? {}),
    })),
  );

  /** 即效 apply 的统一外壳：pending 转圈 + 失败回填 + 错误横幅 + 序号守卫。 */
  async function runLiveApply(key: string, push: () => Promise<void>, rollback: () => void): Promise<void> {
    liveApplySeq += 1;
    const seq = liveApplySeq;
    liveApplyPendingKey = key;
    liveApplyError = null;
    try {
      await push();
    } catch (err) {
      if (seq !== liveApplySeq) return;
      rollback();
      liveApplyError = errorBannerFrom(err);
    } finally {
      if (seq === liveApplySeq) liveApplyPendingKey = null;
    }
  }

  /** 能力集里可即效提交的键：布尔开关 + 数值/文本旋钮（都进 env 注入面）。 */
  type CapabilityDraftKey = keyof CapabilityToggles;

  async function applyCapabilityNow(
    previous: CapabilityToggles,
    key: CapabilityToggleKey,
    attempted: CapabilityToggles,
  ): Promise<void> {
    liveApplySeq += 1;
    const seq = liveApplySeq;
    liveApplyPendingKey = key;
    liveApplyError = null;
    try {
      await onSave(configWithCapabilities(config, attempted));
      if (seq === liveApplySeq) appliedCapabilities = attempted;
    } catch (err) {
      if (seq !== liveApplySeq) return;
      // 失败回滚：只把本次拨动的开关拨回原值，不覆盖其它并发编辑。
      capabilities = {...capabilities, [key]: previous[key]};
      if (key === 'judge' || key === 'council') {
        cognitiveDepth = deriveCognitiveDepth({
          ...DEFAULT_CAPABILITY_TOGGLES,
          ...(config.capabilities ?? {}),
        });
      }
      liveApplyError = errorBannerFrom(err);
    } finally {
      if (seq === liveApplySeq) liveApplyPendingKey = null;
    }
  }

  function handleCapabilityToggle(key: CapabilityToggleKey, checked: boolean): void {
    const previous = capabilities;
    const next = toggledCapability(previous, key, checked);
    capabilities = next;
    if (key === 'judge' || key === 'council') {
      cognitiveDepth = 'custom';
    }
    void applyCapabilityNow(previous, key, next);
  }

  /** 旋钮（滑杆/数值/文本）松手或提交即生效：失败逐键回填旧值 + 横幅。 */
  async function applyKnobNow(keys: CapabilityDraftKey[], attempted: CapabilityToggles): Promise<void> {
    const previous = appliedCapabilities;
    await runLiveApply(
      keys.join('+'),
      async () => {
        await onSave(configWithCapabilities(config, attempted));
        appliedCapabilities = attempted;
      },
      () => {
        // 失败回填：只把本次改动的键拨回上次已应用的值。
        let restored = capabilities;
        for (const key of keys) restored = {...restored, [key]: previous[key]};
        capabilities = restored;
        cognitiveDepth = deriveCognitiveDepth({
          ...DEFAULT_CAPABILITY_TOGGLES,
          ...(config.capabilities ?? {}),
        });
      },
    );
  }

  /** 能力旋钮里的文本项（第 2 级）：失焦/回车提交，失败逐键回填旧值 + 横幅。 */
  const COUNCIL_TEXT_KEYS = ['council.timeoutMs'];
  const REASONING_TEXT_KEYS = ['reasoning.filters', 'reasoning.tag'];
  let reasoningGroupEl = $state<HTMLDivElement | undefined>();

  async function submitCapabilityText(keys: CapabilityDraftKey[], flagKeys: string[]): Promise<void> {
    const attempted = capabilities;
    const previous = appliedCapabilities;
    if (keys.every((key) => attempted[key] === previous[key])) {
      // 空提交：值没变——只清「未保存」标记，不打扰运行时。
      clearTextDirty(flagKeys);
      return;
    }
    await runLiveApply(
      keys.join('+'),
      async () => {
        await onSave(configWithCapabilities(config, attempted));
        appliedCapabilities = attempted;
        ackTextKeys(flagKeys);
      },
      () => {
        let restored = capabilities;
        for (const key of keys) restored = {...restored, [key]: previous[key]};
        capabilities = restored;
        clearTextDirty(flagKeys);
      },
    );
  }

  // ---- 能力中心（2026-10-10 W2/W3 收官批）：分组卡片式旋钮注册表 ----
  // 设计语言：同类桌面 IM 设置页——图标 + 标题 + 副文案 + 右侧开关的分组列表。
  // 每个旋钮如实标注后端 env 名（工程诚实），env 芯片用等宽弱色，不抢视觉。

  type CapDef = {
    key: CapabilityToggleKey;
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
    {key: 'consolidation', icon: Archive, label: '记忆固化', env: 'APEIRETH_ENABLE_CONSOLIDATION', desc: '后台把散碎对话提炼成长期记忆（consolidation）。挂在记忆写入模块上：记忆写入关闭时此项不生效（自报照实）。'},
    {key: 'reflexion', icon: RefreshCcw, label: '反思沉淀', env: 'APEIRETH_ENABLE_REFLEXION', desc: 'reflexion 文件回流：从错误与复盘里沉淀经验。'},
    {key: 'proactiveRecall', icon: Radar, label: '前瞻召回', env: 'APEIRETH_ENABLE_PROACTIVE_RECALL', desc: '闲置时主动浮现可能相关的记忆，不打断但在场。'},
    {key: 'typedRecall', icon: Layers3, label: '类型化召回', env: 'APEIRETH_DISABLE_TYPED_RECALL', invert: true, desc: '按事件/偏好/事实等类型分别召回（默认开）。'},
  ];

  /** 认知增强：W2 接线批落地的五件 + 器官链/偏好学习。 */
  const COGNITION_DEFS: CapDef[] = [
    {key: 'organs', icon: Network, label: '器官链（9 organs）', env: 'APEIRETH_ENABLE_ORGANS', desc: '回合后跑反事实推演/好奇心/情绪记忆等（AfterTurn，不阻塞回复）。'},
    {key: 'preferenceLearning', icon: HeartHandshake, label: '偏好学习', env: 'APEIRETH_ENABLE_PREFERENCE_LEARNING', desc: '把主人偏好写成双索引记忆，后续召回按主题展开。'},
    {key: 'partnerBond', icon: Users, label: '伙伴羁绊', env: 'APEIRETH_ENABLE_PARTNER_BOND', desc: '关系阶段/深度注入语气校准，回合后确定性演化（W2 §4.2）。'},
    {key: 'morphologyRecall', icon: SlidersHorizontal, label: '检索深度自适应', env: 'APEIRETH_ENABLE_MORPHOLOGY_RECALL', desc: '按形态学读数收紧检索条数——只收紧不放大（W2 §4.3）。挂在记忆召回模块上：记忆召回关闭时此项不生效（自报照实）。'},
    {key: 'education', icon: BookOpen, label: 'Dx-Check 教育工具', env: 'APEIRETH_ENABLE_EDUCATION', desc: '模型可调用自查式教学工具，回答前先自检（W2 §4.3）。'},
    {key: 'absorptionInsight', icon: Dumbbell, label: '认知体操', env: 'APEIRETH_ENABLE_ABSORPTION_INSIGHT', desc: '四算法洞察注入（betti/残差金字塔/river/kuramoto，实验性，W2 §4.4）。'},
  ];

  /** 社区与账本：W3 批次落地的检索路由与可溯源记账。 */
  const COMMUNITY_DEFS: CapDef[] = [
    {key: 'communityTriage', icon: GitBranch, label: '图社区分诊', env: 'APEIRETH_ENABLE_COMMUNITY_TRIAGE', desc: '检索前置双路路由：命中实体走实体链，否则给社区摘要（W3 §1）。需记忆召回模块与图谱后端同时在场才生效（自报照实）。'},
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

  /** 工具：四类可授予的工具权限（文件写入带依赖子开关，见 TOOL_SUB_DEFS）。 */
  const TOOL_DEFS: CapDef[] = [
    {key: 'shell', icon: Terminal, label: 'Shell 命令工具', env: 'APEIRETH_ENABLE_SHELL', desc: '模型可提议本地命令——每次执行前仍需你在审批卡点头；写命令（重定向/删除类）另受文件写入总闸管辖。'},
    {key: 'fetch', icon: Globe, label: '网络读取工具', env: 'APEIRETH_ENABLE_FETCH', desc: '公网 GET 只读请求，无凭据转发。'},
    {key: 'localReadTools', icon: FolderSearch, label: '本地只读工具', env: 'APEIRETH_ENABLE_LOCAL_READ_TOOLS', desc: '文件/搜索读侧工具随开关（file / search）；仓库只读探查（repo）恒授、无开关（名实相符对齐，见开关权力审计表）。'},
    {key: 'fileWrite', icon: FilePenLine, label: '文件写入（apply_patch · shell 写命令）', env: 'APEIRETH_ENABLE_FILE_WRITE', desc: '文件写入总闸：apply_patch 与 shell 写命令同受此闸——补丁式受控写文件（创建/修改/删除须在补丁里声明）+ shell 重定向/删除类写命令（关闸即拒绝即帧），每次写入默认要人工审批；工作区外路径与凭据/密钥面拒绝；git 提交等写操作不提供工具（设计边界）。'},
  ];

  /** 工具子开关：依赖 fileWrite 主开关（requires/capDisabled 语义，同沙箱嵌套行），
   *  嵌套展示、不计入工具类数。 */
  const TOOL_SUB_DEFS: CapDef[] = [
    {
      key: 'fileWriteAutoPass',
      icon: FileCheck2,
      label: '自动放行已读文件修改',
      env: 'APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS',
      requires: 'fileWrite',
      desc: '仅修改类补丁免审批；未读文件仍被读前门禁拒绝（先读后写）；删除/新建永不自动放行（仍走人工审批）。',
    },
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
  // 绝不包含 shell/fetch 等危险工具；其余开关保持现状。预设 = 第 1 级即效：
  // 点击立即走同一条 apply 路径（onSave → src-tauri env 注入 + 侧车重启/热应用），
  // 失败整组回填旧值 + 错误横幅。
  function applyRecommendedPreset(): void {
    const next: CapabilityToggles = {...capabilities};
    for (const key of RECOMMENDED_CAPABILITY_PRESET) {
      next[key] = true;
    }
    capabilities = next;
    void applyKnobNow([...RECOMMENDED_CAPABILITY_PRESET], next);
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
    // 「从使用中学习」也是能力开关：同走开关即点即生效语义。
    handleCapabilityToggle('selfTuning', on === true);
  }

  // 预设/恢复基线（第 1 级）：点击即生效——立即走同一条 apply 路径（onSave →
  // 配置持久化 + env 注入 + 网关热应用/重启），失败把拨过的键回填旧值 + 横幅。
  function applyDispositionPreset(name: '省心' | '均衡' | '深度记忆' | '恢复基线'): void {
    let next: CapabilityToggles;
    let keys: CapabilityDraftKey[];
    if (name === '恢复基线') {
      // 恢复基线：四旋钮回基线 + 「从使用中学习」关回默认关（fail-closed）。
      next = {...capabilities, ...DISPOSITION_BASELINE_RESET};
      keys = ['memoryFade', 'curiosityStrength', 'toneSaturation', 'consolidationCadence', 'selfTuning'];
    } else {
      const preset = DISPOSITION_PRESETS[name];
      next = {
        ...capabilities,
        memoryFade: preset.memoryFade,
        curiosityStrength: preset.curiosityStrength,
        toneSaturation: preset.toneSaturation,
        consolidationCadence: preset.consolidationCadence,
      };
      keys = ['memoryFade', 'curiosityStrength', 'toneSaturation', 'consolidationCadence'];
    }
    capabilities = next;
    void applyKnobNow(keys, next);
  }

  // ---- 「预算与配额」：回合预算/上下文预算旋钮 + 会话消耗仪表 ----
  // 数字旋钮 = 失焦/回车提交（三级即效第 2 级），走同一条 apply 缝（onSave →
  // 配置持久化 + env 注入 + 侧车重启/热应用）。越界钳制 1..=64、非法回默认的
  // 语义与后端解析同源（budget.ts 单一来源）；提交后旁注人话反馈，旋钮旁挂
  // 实际生效值徽标（source=configured/constant，自述预算节同款诚实语法）。

  /** 旋钮行注册表（标签/env 芯片/取值域随 spec 走，不在模板里手抄第二份）。 */
  const BUDGET_KNOB_ROWS: ReadonlyArray<{key: BudgetKnobKey; icon: typeof Gauge; hint: string}> = [
    {key: 'maxTurnRounds', icon: Timer, hint: '1..=64；越界钳制，非法回默认 8。'},
    {key: 'maxToolCalls', icon: Wrench, hint: '1..=64；越界钳制，非法回默认 16。'},
    {key: 'contextBudgetChars', icon: Layers3, hint: '正整数（字符）；越界/非法回默认 24000。'},
  ];

  /** 旋钮草稿文本（number 输入的原始字面量；'' = 未配置 = 回默认）。 */
  let budgetDraft = $state<Record<BudgetKnobKey, string>>({
    maxTurnRounds: '',
    maxToolCalls: '',
    contextBudgetChars: '',
  });
  /** 提交反馈（钳制/非法回默认的人话提示；空 = 无）。 */
  let budgetFeedback = $state<Record<BudgetKnobKey, string>>({
    maxTurnRounds: '',
    maxToolCalls: '',
    contextBudgetChars: '',
  });

  const BUDGET_FLAG_KEYS: Record<BudgetKnobKey, string> = {
    maxTurnRounds: 'budget.maxTurnRounds',
    maxToolCalls: 'budget.maxToolCalls',
    contextBudgetChars: 'budget.contextBudgetChars',
  };

  function budgetDraftFromConfig(cfg: ApeirethConfig): Record<BudgetKnobKey, string> {
    const knobs: CapabilityToggles | undefined = cfg.capabilities;
    return {
      maxTurnRounds: knobs?.maxTurnRounds == null ? '' : String(knobs.maxTurnRounds),
      maxToolCalls: knobs?.maxToolCalls == null ? '' : String(knobs.maxToolCalls),
      contextBudgetChars: knobs?.contextBudgetChars == null ? '' : String(knobs.contextBudgetChars),
    };
  }

  // 外部 config 变化时对齐草稿（只跟踪 config，不跟踪页内草稿，打字不被回写）。
  $effect(() => {
    const draft = budgetDraftFromConfig(config);
    untrack(() => {
      budgetDraft = draft;
    });
  });

  /** 数字旋钮提交（失焦/回车，第 2 级即效）：先亮越界/非法反馈，归一值经
   *  同一条 apply 缝即效写 config→env 注入；失败回填旧值 + 横幅。 */
  async function submitBudgetKnob(key: BudgetKnobKey): Promise<void> {
    const spec = BUDGET_KNOB_SPECS[key];
    const resolution = resolveBudgetLimitInput(budgetDraft[key], spec);
    budgetFeedback = {...budgetFeedback, [key]: resolution.feedback};
    budgetDraft = {
      ...budgetDraft,
      [key]: resolution.value === null ? '' : String(resolution.value),
    };
    const previous = appliedCapabilities;
    const attempted = {...capabilities, [key]: resolution.value};
    if (attempted[key] === previous[key]) {
      // 空提交：值没变——只清「未保存」标记，不打扰运行时。
      clearTextDirty([BUDGET_FLAG_KEYS[key]]);
      return;
    }
    capabilities = attempted;
    await runLiveApply(
      key,
      async () => {
        await onSave(configWithCapabilities(config, attempted));
        appliedCapabilities = attempted;
        ackTextKeys([BUDGET_FLAG_KEYS[key]]);
      },
      () => {
        // 失败回填：只把本次改动的键拨回上次已应用的值。
        const restore = previous[key];
        capabilities = {...capabilities, [key]: restore};
        budgetDraft = {
          ...budgetDraft,
          [key]: restore == null ? '' : String(restore),
        };
        clearTextDirty([BUDGET_FLAG_KEYS[key]]);
      },
    );
  }

  /** 三枚旋钮的实际生效值徽标（source=configured/constant + 口径注记）。 */
  const budgetBadges = $derived(effectiveBudget(capabilities));

  /** 会话消耗卡 + 预算余量条（真值渲染，无数据位诚实「—」）。 */
  const sessionTotalsView = $derived(formatSessionTotals(sessionUsage ?? emptySessionTotals()));
  const budgetRemaining = $derived(
    budgetRemainingRows(sessionUsage ?? emptySessionTotals(), budgetBadges),
  );

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
  // 范围），然后立即生效（第 1 级）——写回值走同一条 apply 路径（onSave：
  // 配置持久化 + env 注入 + 网关热应用/重启），失败回填旧值 + 横幅。之所以
  // 不另立 revert-request 文件：现有 apply 路径是既定架构，独立的撤销请求
  // 需要后端新增一条摄取通路，超出本次「性格养成第一铲」范围。
  function undoTuningEntry(entry: TuningLogEntry): void {
    let key: CapabilityDraftKey;
    switch (entry.param) {
      case 'memory_fade':
        key = 'memoryFade';
        setMemoryFade(entry.previous);
        break;
      case 'curiosity_strength':
        key = 'curiosityStrength';
        setCuriosityStrength(entry.previous);
        break;
      case 'tone_saturation':
        key = 'toneSaturation';
        setToneSaturation(entry.previous);
        break;
      case 'consolidation_cadence':
        key = 'consolidationCadence';
        setConsolidationCadence(entry.previous);
        break;
      default:
        return;
    }
    void applyKnobNow([key], capabilities);
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

  // ---- 文本输入（第 2 级）：失焦或回车提交，字段旁 ✓ / 「未保存」微提示 ----
  let textDirty = $state<Record<string, boolean>>({});
  let textAck = $state<Record<string, boolean>>({});
  const textAckTimers = new Map<string, ReturnType<typeof setTimeout>>();

  function textFlag(key: string): TextCommitFlag {
    return textCommitFlag(textDirty[key] === true, textAck[key] === true);
  }

  /** 输入即标「未提交」（旁注「未保存」），提交成功后清掉并闪 ✓。 */
  function markTextDirty(key: string): void {
    textDirty = {...textDirty, [key]: true};
  }

  /** 提交成功：清「未保存」标记 + 闪 ✓（1.6s 后收起）。 */
  function ackTextKeys(keys: string[]): void {
    const nextDirty = {...textDirty};
    const nextAck = {...textAck};
    for (const key of keys) {
      nextDirty[key] = false;
      nextAck[key] = true;
      const timer = textAckTimers.get(key);
      if (timer !== undefined) clearTimeout(timer);
      textAckTimers.set(
        key,
        setTimeout(() => {
          textAck = {...textAck, [key]: false};
          textAckTimers.delete(key);
        }, 1600),
      );
    }
    textDirty = nextDirty;
    textAck = nextAck;
  }

  /** 字段回到与已提交一致（提交成功或回填旧值后）：只清「未保存」标记。 */
  function clearTextDirty(keys: string[]): void {
    const nextDirty = {...textDirty};
    for (const key of keys) nextDirty[key] = false;
    textDirty = nextDirty;
  }

  /** 整组失焦提交：焦点移出整组才提交，组内移动焦点（Tab/点组内按钮）不提交。 */
  function groupFocusOut(e: FocusEvent, groupEl: HTMLElement | undefined, submit: () => void): void {
    const next = e.relatedTarget as Node | null;
    if (!focusLeavesGroup(next !== null && groupEl !== undefined && groupEl.contains(next))) return;
    submit();
  }

  /** 整组回车提交：只认单行输入框（textarea 回车 = 换行，走失焦提交）。 */
  function groupKeydown(e: KeyboardEvent, submit: () => void): void {
    const target = e.target as HTMLElement | null;
    if (target?.tagName !== 'INPUT' || !isTextCommitKey(e.key)) return;
    e.preventDefault();
    submit();
  }

  // ---- 服务商组（端点 + 模型 + 密钥 + 协议头）整组失焦/回车提交 ----
  // 整组一个提交原子，避免半截配置：端点/模型/密钥任一改动都等整组提交，
  // 失败整组回填旧值 + 横幅。
  let providerGroupEl = $state<HTMLDivElement | undefined>();
  let providerCommitted: ProviderGroupDraft | null = null;

  const PROVIDER_TEXT_KEYS = [
    'provider.baseUrl',
    'provider.model',
    'provider.apiKey',
    'provider.anthropicVersion',
  ];

  function providerGroupDraft(): ProviderGroupDraft {
    return {
      protocol: activeProtocol,
      preset: activePreset,
      baseUrl: providerBaseUrl,
      apiKey: providerApiKey,
      model: providerModel,
      anthropicVersion,
    };
  }

  function loadProviderGroup(draft: ProviderGroupDraft): void {
    activeProtocol = draft.protocol;
    activePreset = draft.preset;
    providerBaseUrl = draft.baseUrl;
    providerApiKey = draft.apiKey;
    providerModel = draft.model;
    anthropicVersion = draft.anthropicVersion;
  }

  /** 组内改动（含点选类）都标「未提交」：整组失焦/回车时一次性提交。 */
  function markProviderDirty(keys: string[] = PROVIDER_TEXT_KEYS): void {
    for (const key of keys) markTextDirty(key);
  }

  function providerGroupDirty(): boolean {
    if (providerCommitted === null) return true;
    return !isSameAsCommitted(
      providerGroupSnapshot(providerGroupDraft()),
      providerGroupSnapshot(providerCommitted),
    );
  }

  async function submitProviderGroup(): Promise<void> {
    const draft = providerGroupDraft();
    if (!providerGroupDirty()) {
      // 空提交：值没变——只清「未保存」标记，不打扰运行时。
      clearTextDirty(PROVIDER_TEXT_KEYS);
      return;
    }
    await runLiveApply(
      'provider-group',
      async () => {
        await handleSaveSettings();
        providerCommitted = {...draft};
        ackTextKeys(PROVIDER_TEXT_KEYS);
      },
      () => {
        // 失败整组回填旧值（半截配置不留运行时），横幅由外壳统一亮起。
        if (providerCommitted !== null) loadProviderGroup(providerCommitted);
        clearTextDirty(PROVIDER_TEXT_KEYS);
      },
    );
  }

  // ---- 多 Agent 人设 (数据驱动, 整组失焦/回车提交, 点选类动作即点即生效) ----
  let personas = $state<PersonaProfile[]>([]);
  let activePersonaId = $state<string>('');
  let personasGroupEl = $state<HTMLDivElement | undefined>();
  let personasCommitted: {personas: PersonaProfile[]; activePersonaId: string} | null = null;

  function personaTextKeys(): string[] {
    return personas.flatMap((p) => [`persona.name:${p.id}`, `persona.model:${p.id}`, `persona.text:${p.id}`]);
  }

  function personasDirty(): boolean {
    if (personasCommitted === null) return true;
    return !isSameAsCommitted(
      personasSnapshot(personas, activePersonaId),
      personasSnapshot(personasCommitted.personas, personasCommitted.activePersonaId),
    );
  }

  /** 人设整组提交：人设列表 + 当前伙伴 id 一次落位，失败整组回填旧值。 */
  async function submitPersonas(): Promise<void> {
    const attempted = personas;
    const attemptedActive = activePersonaId;
    if (!personasDirty()) {
      clearTextDirty(personaTextKeys());
      return;
    }
    await runLiveApply(
      'personas',
      async () => {
        await onSave(configWithPersonas(config, attempted, attemptedActive));
        personasCommitted = {
          personas: attempted.map((p) => ({...p})),
          activePersonaId: attemptedActive,
        };
        ackTextKeys(personaTextKeys());
      },
      () => {
        if (personasCommitted !== null) {
          personas = personasCommitted.personas.map((p) => ({...p}));
          activePersonaId = personasCommitted.activePersonaId;
        }
        clearTextDirty(personaTextKeys());
      },
    );
  }

  function addPersona(): void {
    personas = [...personas, {id: crypto.randomUUID(), name: '新伙伴', persona: ''}];
    // 新增 = 即点即生效（第 1 级），失败回填旧列表 + 横幅。
    void submitPersonas();
  }

  function removePersona(id: string): void {
    if (personas.length <= 1) return;
    personas = personas.filter((p) => p.id !== id);
    if (activePersonaId === id) activePersonaId = personas[0]?.id || '';
    void submitPersonas();
  }

  // Sync config from props（只跟外部 config 变化同步；本地草稿一律 untrack
  // 读取，有未提交文本时不覆盖在途编辑）
  $effect(() => {
    const nextBaseUrl = config.baseUrl;
    const nextOpenai = config.openaiConfig;
    const nextAnthropic = config.anthropicConfig;
    const nextProvider = config.provider;
    const nextModel = config.model;
    untrack(() => {
      if (!textFlagDirty(GATEWAY_TEXT_KEYS)) {
        editBaseUrl = nextBaseUrl;
        gatewayCommitted = nextBaseUrl;
      }

      if (nextOpenai) {
        openaiBuffer = {
          preset: nextOpenai.preset || 'openai',
          baseUrl: nextOpenai.baseUrl || 'https://api.openai.com/v1',
          apiKey: nextOpenai.apiKey || '',
          model: nextOpenai.model || 'gpt-4o',
        };
      }

      if (nextAnthropic) {
        anthropicBuffer = {
          preset: nextAnthropic.preset || 'anthropic',
          baseUrl: nextAnthropic.baseUrl || 'https://api.anthropic.com',
          apiKey: nextAnthropic.apiKey || '',
          model: nextAnthropic.model || 'claude-3-7-sonnet-20250219',
          anthropicVersion: nextAnthropic.anthropicVersion || '2023-06-01',
        };
      }

      if (!textFlagDirty(PROVIDER_TEXT_KEYS)) {
        if (nextProvider) {
          activeProtocol = nextProvider.protocol;
          activePreset = nextProvider.preset || 'openai';
          providerBaseUrl = nextProvider.baseUrl;
          providerApiKey = nextProvider.apiKey || '';
          providerModel = nextProvider.model || nextModel;
          anthropicVersion = nextProvider.anthropicVersion || '2023-06-01';
        } else {
          activeProtocol = 'openai';
          activePreset = 'openai';
          providerBaseUrl = 'https://api.openai.com/v1';
          providerApiKey = '';
          providerModel = nextModel || 'gpt-4o';
        }
        providerCommitted = providerGroupDraft();
      }
    });
  });

  $effect(() => {
    const source = config.personas && config.personas.length > 0 ? config.personas : DEFAULT_PERSONAS;
    const nextActive = config.activePersonaId || source[0]?.id || '';
    untrack(() => {
      if (personaTextDirty()) return;
      personas = source.map((p) => ({...p}));
      activePersonaId = nextActive;
      personasCommitted = {
        personas: personas.map((p) => ({...p})),
        activePersonaId: nextActive,
      };
    });
  });

  /** 这些字段有在途编辑（未提交）时不许被外部同步冲掉。 */
  function textFlagDirty(keys: string[]): boolean {
    return keys.some((key) => textDirty[key] === true);
  }

  /** 人设字段按前缀判定：同步效果里不读 personas 草稿，避免自触发。 */
  function personaTextDirty(): boolean {
    return Object.keys(textDirty).some((key) => key.startsWith('persona.') && textDirty[key] === true);
  }

  // ---- 网关服务地址（第 2 级）：失焦/回车提交，失败回填旧值 ----
  const GATEWAY_TEXT_KEYS = ['gateway.baseUrl'];
  let gatewayCommitted = '';

  async function submitGatewayUrl(): Promise<void> {
    const attempted = editBaseUrl;
    if (isSameAsCommitted(attempted.trim(), gatewayCommitted.trim())) {
      clearTextDirty(GATEWAY_TEXT_KEYS);
      return;
    }
    await runLiveApply(
      'gateway-url',
      async () => {
        await onSave(configWithGatewayUrl(config, attempted));
        gatewayCommitted = attempted;
        ackTextKeys(GATEWAY_TEXT_KEYS);
      },
      () => {
        editBaseUrl = gatewayCommitted;
        clearTextDirty(GATEWAY_TEXT_KEYS);
      },
    );
  }

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

  // P1-1: 无重启热应用回显（失败横幅统一走顶部 liveApplyError，pending 走行内态）
  let applyResult = $state<{ok: boolean; warnings: string[]} | null>(null);
  let effectiveConfig = $state<{provider?: string; base_url?: string; api_key?: string; model?: string} | null>(null);

  // P0-2: 从 /v1/models 拉取的模型列表
  let discoveredModels = $state<ModelInfo[]>([]);
  let modelsLoading = $state(false);
  let modelsLoadedOnce = false;
  let modelsError = $state('');

  // P1-2: 全局权限预设默认值。admin config 契约无 permission_preset 字段
  // (它是会话级 session settings)，这里仅作 UI 状态 + 本地记录，不进配置 patch。
  const PERMISSION_PRESET_KEY = 'apeireth-permission-preset-default';
  let permissionPreset = $state<'read_only' | 'standard' | 'full'>(initialPermissionPreset());

  // P1-4: 工作区目录
  let workspaceDir = $state('');
  let workspaceLoading = $state(false);
  let workspaceLoadedOnce = false;
  let workspaceError = $state('');
  let showWorkspacePicker = $state(false);
  let workspaceSuggestions = $state<string[]>([]);

  // ---- 危险动作（第 3 级）：即点 + 二次确认弹层，确认后即效 ----
  // 删数据 / 清记忆 / 断开连接类动作一律先开确认弹层；确认文案来自
  // DANGER_ACTION_CONFIRMATIONS（唯一文案源，测试镜像校验覆盖面）。
  let dangerPending = $state<DangerActionKey | null>(null);
  let dangerPayload = $state('');

  const dangerConfirm = $derived(dangerPending === null ? null : DANGER_ACTION_CONFIRMATIONS[dangerPending]);

  function requestDanger(action: DangerActionKey, payload = ''): void {
    dangerPending = action;
    dangerPayload = payload;
  }

  async function runDangerAction(): Promise<void> {
    const action = dangerPending;
    const payload = dangerPayload;
    dangerPending = null;
    dangerPayload = '';
    if (action === null) return;
    switch (action) {
      case 'clearLocalData':
        if (onClearLocalData) onClearLocalData();
        break;
      case 'deleteStoredKey':
        // 删除即效（第 3 级确认后）：钥匙串删除 + 组清空提交，自身走即效外壳。
        await deleteStoredKey();
        break;
      case 'removePersona':
        removePersona(payload);
        break;
      case 'clearCustomBg':
        // 背景图清理由主题面板自己收口（图本体在浏览器数据库里）。
        break;
    }
  }

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
    {id: 'security', label: '安全与治理', icon: ShieldCheck},
    {id: 'tools', label: '工具与安全', icon: Wrench},
    {id: 'budget', label: '预算与配额', icon: Gauge},
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

  /** 危险动作（断开连接）：二次确认后立即删钥匙串密钥并清空组内密钥。 */
  async function deleteStoredKey(): Promise<void> {
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

    // 同步清掉组内密钥，避免删除后测试连接仍带上旧 key；清空后整组立即提交。
    providerApiKey = '';
    await runLiveApply(
      'delete-key',
      async () => {
        await onSave({
          ...configWithProviderGroup(config, providerGroupDraft(), DEFAULT_MODEL_ID),
          apiKey: '',
        });
        providerCommitted = {...providerGroupDraft()};
        ackTextKeys(PROVIDER_TEXT_KEYS);
      },
      () => {
        // 密钥已从钥匙串删除：不回填旧密钥（它指向的钥匙已不存在），
        // 只把组快照对齐当前值，横幅由外壳统一亮起。
        providerCommitted = {...providerGroupDraft()};
      },
    );
  }

  async function loadWorkspace() {
    workspaceLoading = true;
    workspaceError = '';
    const dir = await getWorkspaceDir();
    workspaceDir = dir ?? '';
    workspaceCommitted = workspaceDir;
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
      workspaceCommitted = applied;
      ackTextKeys(WORKSPACE_TEXT_KEYS);
    } else {
      workspaceError = '设置工作区目录失败，请检查路径权限后重试。';
    }
    showWorkspacePicker = false;
  }

  // ---- 工作区路径（第 2 级）：失焦/回车提交，失败回填旧值 + 横幅 ----
  const WORKSPACE_TEXT_KEYS = ['workspace.dir'];
  let workspaceCommitted = '';

  async function submitWorkspaceDir(): Promise<void> {
    const attempted = workspaceDir;
    if (isSameAsCommitted(attempted.trim(), workspaceCommitted.trim())) {
      clearTextDirty(WORKSPACE_TEXT_KEYS);
      return;
    }
    await runLiveApply(
      'workspace',
      async () => {
        const applied = await setWorkspaceDir(attempted);
        if (applied === null) throw new Error('设置工作区目录失败，请检查路径权限后重试。');
        workspaceDir = applied;
        workspaceCommitted = applied;
        ackTextKeys(WORKSPACE_TEXT_KEYS);
      },
      () => {
        workspaceDir = workspaceCommitted;
        clearTextDirty(WORKSPACE_TEXT_KEYS);
      },
    );
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

  /** 服务商组的 apply 缝：端点/模型/密钥整组写回 + 密钥入钥匙串 + 网关热应用
   *  回显。页面「保存」按钮已删除——本缝由服务商组失焦/回车提交与密钥动作复用，
   *  一次只提交一个切面（叠加不覆盖其余字段）；失败向上抛，由调用侧回填 + 横幅。 */
  async function handleSaveSettings(): Promise<void> {
    const updated = configWithProviderGroup(config, providerGroupDraft(), DEFAULT_MODEL_ID);
    applyResult = null;
    effectiveConfig = null;

    // 主 apply 路径：onSave = 配置持久化 + env 注入 + 侧车重启/热应用。
    await onSave(updated);

    // 密钥随改动写入系统钥匙串（不落盘明文）——配置落盘会清洗密钥，钥匙串
    // 是密钥唯一真源：只进内存的 key 重启后会被旧值覆盖（表现为「发旧 key」）。
    // 仅在输入了新密钥时写入，空值不覆盖钥匙串；写入失败记警告。
    const warnings: string[] = [];
    if (providerApiKey.trim()) {
      const stored = await setProviderKey(providerFamily(), providerApiKey.trim());
      if (!stored) {
        warnings.push('系统钥匙串写入未成功：密钥只在本次运行生效，重启后可能回退旧值。');
      }
    }

    // P1-1: 无重启热应用 + 生效配置回显。这是尽力而为的加速路径：失败只记
    // 警告，主 apply 已生效（配置变化时侧车会带着新环境重启）。
    try {
      const patch: AdminConfigPatch = {
        provider: providerFamily(),
        base_url: normalizeBaseUrl(providerBaseUrl),
        model: providerModel.trim() || DEFAULT_MODEL_ID,
        ...(providerApiKey.trim() ? {api_key: providerApiKey.trim()} : {}),
      };
      applyResult = await applyAdminConfig(updated, patch);
      effectiveConfig = await getAdminConfig(updated);
    } catch (err) {
      warnings.push(`网关热应用未确认（${describeCaught(err)}）：配置已注入侧车，网关重启后自动拾取。`);
    }
    if (warnings.length > 0) {
      applyResult = {
        ok: applyResult?.ok ?? true,
        warnings: [...warnings, ...(applyResult?.warnings ?? [])],
      };
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
      liveApplyError = errorBannerFrom(err);
    }

    // 密钥必须落到 provider 组配置（与组提交同一条 apply 路径）：只进网关
    // apiKey 字段根本到不了侧车——填完仍会以 "missing API key" 失败。
    providerApiKey = key;
    const updated: ApeirethConfig = {
      ...configWithProviderGroup(config, providerGroupDraft(), DEFAULT_MODEL_ID),
      apiKey: key,
    };
    void Promise.resolve(onSave(updated)).catch((err) => (liveApplyError = errorBannerFrom(err)));
    providerCommitted = {...providerGroupDraft()};
    ackTextKeys(PROVIDER_TEXT_KEYS);
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
    subtitle="配置模型提供商、记忆与认知、决策治理、工具安全与数据存储。改动即点/失焦即生效。"
  />

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
      <!-- 三级即效语义：pending 行内态（侧车重启/热应用期间）+ 失败横幅
           （回填旧值后告知用户）。全程行内提示，不弹模态。 -->
      {#snippet fieldFlag(key: string)}
        {#if textFlag(key) === 'dirty'}
          <span class="field-flag field-dirty" title="改动尚未提交：失焦或回车即提交">未保存</span>
        {:else if textFlag(key) === 'ack'}
          <span class="field-flag field-ack" title="已提交并生效"><Check size={12} /></span>
        {/if}
      {/snippet}
      {#if liveApplyPendingKey}
        <div class="apply-status"><RotateCcw size={13} class="spin" /><span>正在应用到运行时…</span></div>
      {/if}
      {#if liveApplyError}
        <ErrorSolutionBanner
          code={liveApplyError.code}
          message={liveApplyError.message}
          solution={liveApplyError.solution}
          onClose={() => (liveApplyError = null)}
        />
      {/if}
      {#if activeSection === 'appearance'}
        <div class="setting-block">
          <h3 class="block-title">外观与主题</h3>
          <p class="block-desc">选择界面照明档位与背景风格，立即生效并写入本地配置。</p>
          <ThemeSettingsPanel
            {config}
            onSave={(cfg) =>
              runLiveApply(
                'appearance',
                async () => {
                  await onSave(cfg);
                },
                () => {
                  /* 主题/强调色/背景的回退由 App 的 apply 缝统一收口（回退
                     config + 重新套用改动前的文档主题），这里只负责行内态 */
                },
              )}
          />
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

          <!-- 服务商组（端点 + 模型 + 密钥 + 协议头）：整组失焦/回车提交 -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            class="text-group"
            role="group"
            aria-label="服务商端点、密钥与模型"
            bind:this={providerGroupEl}
            onfocusout={(e) => groupFocusOut(e, providerGroupEl, () => void submitProviderGroup())}
            onkeydown={(e) => groupKeydown(e, () => void submitProviderGroup())}
          >
          <div class="config-step">
            <span class="step-num">1</span>
            <div class="step-body">
              <h4 class="step-title">选择协议</h4>
              <div class="protocol-tabs">
                <button
                  class="protocol-tab"
                  class:selected={activeProtocol === 'openai'}
                  onclick={() => {
                    switchProtocol('openai');
                    markProviderDirty();
                  }}
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
                  onclick={() => {
                    switchProtocol('anthropic');
                    markProviderDirty();
                  }}
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
                    onclick={() => {
                      selectPreset(p.id);
                      markProviderDirty(['provider.baseUrl', 'provider.model']);
                    }}
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
                  <label for="provider-url-input">API 端点 (Base URL) {@render fieldFlag('provider.baseUrl')}</label>
                  <input
                    id="provider-url-input"
                    type="text"
                    bind:value={providerBaseUrl}
                    oninput={() => markTextDirty('provider.baseUrl')}
                    placeholder={activeProtocol === 'openai' ? 'https://api.openai.com/v1' : 'https://api.anthropic.com'}
                  />
                </div>
                <div class="form-group">
                  <label for="provider-model-input">活动模型 ID {@render fieldFlag('provider.model')}</label>
                  <input
                    id="provider-model-input"
                    type="text"
                    bind:value={providerModel}
                    oninput={() => markTextDirty('provider.model')}
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
                    onSelect={(id) => {
                      providerModel = id;
                      markTextDirty('provider.model');
                    }}
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
                  {@render fieldFlag('provider.apiKey')}
                </label>
                <div class="key-input-wrapper">
                  <input
                    id="provider-key-input"
                    type={showApiKey ? 'text' : 'password'}
                    bind:value={providerApiKey}
                    oninput={() => markTextDirty('provider.apiKey')}
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
                  <label for="anthropic-ver-input">anthropic-version {@render fieldFlag('provider.anthropicVersion')}</label>
                  <input
                    id="anthropic-ver-input"
                    type="text"
                    bind:value={anthropicVersion}
                    oninput={() => markTextDirty('provider.anthropicVersion')}
                    placeholder="2023-06-01"
                  />
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
                      onclick={() => {
                        providerModel = m;
                        markTextDirty('provider.model');
                      }}
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
                        onclick={() => {
                          providerModel = dm;
                          markTextDirty('provider.model');
                        }}
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
          </div>

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
                  <label for="endpoint-input">网关服务地址 (Gateway URL) {@render fieldFlag('gateway.baseUrl')}</label>
                  <input
                    id="endpoint-input"
                    type="text"
                    bind:value={editBaseUrl}
                    oninput={() => markTextDirty('gateway.baseUrl')}
                    onfocusout={() => void submitGatewayUrl()}
                    onkeydown={(e) => {
                      if (isTextCommitKey(e.key)) {
                        e.preventDefault();
                        void submitGatewayUrl();
                      }
                    }}
                    placeholder="http://127.0.0.1:8080"
                  />
                  <small class="field-hint">默认为 Apeireth 核心网关端口 (:8080)；失焦或回车提交，失败回填旧值。</small>
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
            数据驱动的多 Agent 身份：可随时增删改。文本改动失焦或回车整组提交（避免半截配置）；
            「设为当前 / 新增伙伴」即点即生效，删除需二次确认（人设作为 system 消息注入每次对话），无需重编译。
          </p>

          <!-- 人设组：整组失焦/回车提交（人设列表 + 当前伙伴 id 一次落位） -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            class="text-group"
            role="group"
            aria-label="伙伴人设"
            bind:this={personasGroupEl}
            onfocusout={(e) => groupFocusOut(e, personasGroupEl, () => void submitPersonas())}
            onkeydown={(e) => groupKeydown(e, () => void submitPersonas())}
          >
          {#each personas as p, i (p.id)}
            <div class="persona-card">
              <div class="persona-card-head">
                <span class="persona-index">伙伴 {i + 1}</span>
                <div class="persona-card-actions">
                  <button
                    class="quiet-button"
                    class:selected={activePersonaId === p.id}
                    onclick={() => {
                      activePersonaId = p.id;
                      void submitPersonas();
                    }}
                    title="设为当前伙伴"
                  >
                    {activePersonaId === p.id ? '✓ 当前伙伴' : '设为当前'}
                  </button>
                  <button
                    class="quiet-button danger-text"
                    onclick={() => requestDanger('removePersona', p.id)}
                    disabled={personas.length <= 1}
                    title="删除该伙伴（需二次确认）"
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              </div>
              <div class="form-group">
                <label for="persona-name-{p.id}">名称 {@render fieldFlag(`persona.name:${p.id}`)}</label>
                <input
                  id="persona-name-{p.id}"
                  type="text"
                  bind:value={p.name}
                  oninput={() => markTextDirty(`persona.name:${p.id}`)}
                  placeholder="伙伴名称"
                />
              </div>
              <div class="form-group">
                <label for="persona-model-{p.id}">固定模型（可选，留空跟随全局设置） {@render fieldFlag(`persona.model:${p.id}`)}</label>
                <input
                  id="persona-model-{p.id}"
                  type="text"
                  value={p.model || ''}
                  oninput={(e) => {
                    p.model = (e.currentTarget as HTMLInputElement).value.trim() || undefined;
                    markTextDirty(`persona.model:${p.id}`);
                  }}
                  placeholder={config.model || 'deepseek-chat'}
                />
              </div>
              <div class="form-group">
                <label for="persona-text-{p.id}">人设文本（system 消息；留空 = 该伙伴不注入人设） {@render fieldFlag(`persona.text:${p.id}`)}</label>
                <textarea
                  id="persona-text-{p.id}"
                  rows={5}
                  bind:value={p.persona}
                  oninput={() => markTextDirty(`persona.text:${p.id}`)}
                  placeholder="你是「阿佩瑞斯」——Apeireth 基地的主管…"
                ></textarea>
              </div>
            </div>
          {/each}
          </div>

          <button class="quiet-button" onclick={addPersona}>
            <Plus size={14} />
            <span>新增伙伴</span>
          </button>

          <div class="notice-box">
            <StatusBadge label="实时生效" variant="green" size="small" />
            <span>人设由客户端作为 system 消息注入每次请求；失焦或回车提交后立即生效，重启应用仍保留（不含任何密钥）。</span>
          </div>
        </div>

      {:else if activeSection === 'cognition'}
        <div class="setting-block">
          <h3 class="block-title">记忆与认知</h3>
          <p class="block-desc">
            记忆流运行态 + W2/W3 收官批落地的认知增强件。记忆核心族（记忆注入/前瞻召回/偏好学习）
            默认开启、可随时关；其余默认关闭、逐项显式开启。拨动即注入侧车环境并生效（配置没变不会重启网关）。
          </p>

          <div class="cap-summary">
            <Brain size={14} />
            <span>本页已启用 <b>{enabledCount(MEMORY_FLOW_DEFS) + enabledCount(COGNITION_DEFS) + enabledCount(COMMUNITY_DEFS)}</b> / {MEMORY_FLOW_DEFS.length + COGNITION_DEFS.length + COMMUNITY_DEFS.length} 项认知能力</span>
          </div>

          <div class="preset-bar">
            <button class="quiet-button" onclick={applyRecommendedPreset}>
              <Sparkles size={14} />
              <span>应用推荐配置</span>
            </button>
            <span class="preset-hint">一键开启记忆核心族 + 记忆固化/反思沉淀/器官链，不含 shell/fetch；点击即生效（失败回填 + 横幅）。</span>
          </div>
          <div class="notice-box">
            <span>
              将开启 6 项：前瞻召回、偏好学习、记忆注入、记忆固化、反思沉淀、器官链。
              <strong>不包含</strong> Shell / 网络读取等危险工具（它们保持现状、仍需逐项开启与审批），其余开关不动。
            </span>
          </div>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Layers3 size={13} /> 记忆流</span>
              <span class="cap-card-count">{enabledCount(MEMORY_FLOW_DEFS)}/{MEMORY_FLOW_DEFS.length}</span>
            </div>
            {#each MEMORY_FLOW_DEFS as def (def.key)}
              <button
                class="cap-row"
                class:dim={capDisabled(def)}
                class:pending={liveApplyPendingKey === def.key}
                onclick={() => toggleCap(def)}
                disabled={liveApplyPendingKey !== null}
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
                class:pending={liveApplyPendingKey === def.key}
                onclick={() => toggleCap(def)}
                disabled={liveApplyPendingKey !== null}
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
                      onchange={() => applyKnobNow(['morphologyTemperature'], capabilities)}
                    />
                    <b>{capabilities.morphologyTemperature.toFixed(1)}</b>
                  </span>
                </div>
              {/if}
            {/each}
          </div>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><GitBranch size={13} /> 社区与账本 · W3 落地批</span>
              <span class="cap-card-count">{enabledCount(COMMUNITY_DEFS)}/{COMMUNITY_DEFS.length}</span>
            </div>
            {#each COMMUNITY_DEFS as def (def.key)}
              <button
                class="cap-row"
                class:dim={capDisabled(def)}
                class:pending={liveApplyPendingKey === def.key}
                onclick={() => toggleCap(def)}
                disabled={liveApplyPendingKey !== null}
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
                  onchange={() => applyKnobNow(['memoryFade'], capabilities)}
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
                  onchange={() => applyKnobNow(['curiosityStrength'], capabilities)}
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
                  onchange={() => applyKnobNow(['toneSaturation'], capabilities)}
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
                  onchange={() => applyKnobNow(['consolidationCadence'], capabilities)}
                />
                <span class="preset-hint">基线 1（每回合）</span>
                <b>每 {capabilities.consolidationCadence} 回合</b>
              </span>
            </div>
          </div>

          <div class="preset-bar">
            <button class="quiet-button" onclick={() => applyDispositionPreset('省心')}>
              <Sparkles size={14} />
              <span>省心</span>
            </button>
            <button class="quiet-button" onclick={() => applyDispositionPreset('均衡')}>
              <Sparkles size={14} />
              <span>均衡</span>
            </button>
            <button class="quiet-button" onclick={() => applyDispositionPreset('深度记忆')}>
              <Sparkles size={14} />
              <span>深度记忆</span>
            </button>
            <button class="quiet-button" onclick={() => applyDispositionPreset('恢复基线')}>
              <RotateCcw size={14} />
              <span>恢复基线</span>
            </button>
            <span class="preset-hint">预设只动这 4 个体验参数（「恢复基线」另把「从使用中学习」关回默认关），点击即生效；失败回填旧值 + 横幅。</span>
          </div>

          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Sparkles size={13} /> 从使用中学习</span>
              <span class="cap-card-count">默认关</span>
            </div>
            <button
              class="cap-row"
              class:pending={liveApplyPendingKey === 'selfTuning'}
              onclick={() => setSelfTuning(!capabilities.selfTuning)}
              disabled={liveApplyPendingKey !== null}
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
                  挂在记忆检索信号链上：记忆召回关闭时此项不生效（自报照实）。
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
                  <small>「撤销」= 把该参数拨回记录原值并立即生效（写回值走同一条 apply 路径，不做独立撤销请求）。</small>
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
                class:pending={liveApplyPendingKey === def.key}
                onclick={() => toggleCap(def)}
                disabled={liveApplyPendingKey !== null}
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
                    onchange={() => applyKnobNow(['councilAdvisors'], capabilities)}
                  />
                  <b>{capabilities.councilAdvisors}</b>
                </span>
              </div>
              <div class="cap-row cap-row-static">
                <span class="cap-icon"><Timer size={15} /></span>
                <span class="cap-text">
                  <strong>单顾问超时 (ms)<code class="cap-env">APEIRETH_COUNCIL_TIMEOUT_MS</code> {@render fieldFlag('council.timeoutMs')}</strong>
                  <small>默认 30000；思考型模型延迟高时可放宽。失焦或回车提交，失败回填旧值。</small>
                </span>
                <span class="cap-slider-wrap">
                  <input
                    type="number"
                    min="1000"
                    step="5000"
                    value={capabilities.councilTimeoutMs}
                    aria-label="单顾问超时毫秒"
                    oninput={(e) => {
                      setCouncilTimeout(Number((e.currentTarget as HTMLInputElement).value));
                      markTextDirty('council.timeoutMs');
                    }}
                    onfocusout={() => void submitCapabilityText(['councilTimeoutMs'], COUNCIL_TEXT_KEYS)}
                    onkeydown={(e) => {
                      if (isTextCommitKey(e.key)) {
                        e.preventDefault();
                        void submitCapabilityText(['councilTimeoutMs'], COUNCIL_TEXT_KEYS);
                      }
                    }}
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
                class:pending={liveApplyPendingKey === def.key}
                onclick={() => toggleCap(def)}
                disabled={liveApplyPendingKey !== null}
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

      {:else if activeSection === 'security'}
        <!-- 「安全与治理」（侧栏收纳批）：原治理卷宗面板原样搬入，不重写 -->
        <div class="setting-block">
          <h3 class="block-title">安全与治理</h3>
          <p class="block-desc">
            审批的账、授权的账、守卫的账、执行的账——对话内完成的判断，在这里成卷。
          </p>
          {#key governanceKey}
            <GovernanceView
              config={config}
              capabilities={capabilityManifest}
              initialTab={initialGovernanceTab}
              onOpenChat={() => onGovernanceOpenChat?.()}
            />
          {/key}
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
                class:pending={liveApplyPendingKey === def.key}
                onclick={() => toggleCap(def)}
                disabled={liveApplyPendingKey !== null}
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
                  class:pending={liveApplyPendingKey === 'shellSandbox'}
                  onclick={() => handleCapabilityToggle('shellSandbox', !capabilities.shellSandbox)}
                  disabled={liveApplyPendingKey !== null}
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

              {#if def.key === 'fileWrite'}
                {#each TOOL_SUB_DEFS as subDef (subDef.key)}
                  <button
                    class="cap-row cap-row-nested"
                    class:dim={capDisabled(subDef)}
                    class:pending={liveApplyPendingKey === subDef.key}
                    onclick={() => toggleCap(subDef)}
                    disabled={capDisabled(subDef) || liveApplyPendingKey !== null}
                    role="switch"
                    aria-checked={isCapOn(subDef)}
                    aria-label={subDef.label}
                  >
                    <span class="cap-icon"><subDef.icon size={15} /></span>
                    <span class="cap-text">
                      <strong>{subDef.label}<code class="cap-env">{subDef.env}</code></strong>
                      <small>{subDef.desc}</small>
                    </span>
                    <span class="cap-switch" class:on={isCapOn(subDef)}><span class="cap-knob"></span></span>
                  </button>
                {/each}
              {/if}
            {/each}
            <!-- 外部工具桥: 无开关（需另行配置服务器列表才真正生效）——不硬造开关,
                 如实登记本页无此开关面, 服务端 CLI 旋钮见 env 芯片。 -->
            <div class="cap-row cap-row-static">
              <span class="cap-icon"><Server size={15} /></span>
              <span class="cap-text">
                <strong>外部工具桥（MCP）<code class="cap-env">APEIRETH_ENABLE_MCP</code></strong>
                <small>
                  无开关（本页不设开关，服务端 CLI 旋钮）——需另行配置服务器列表
                  （APEIRETH_MCP_SERVERS 或数据目录 mcp-servers.json）才真正有外部工具；
                  已启用但无服务器配置 = 无外部工具（自报照实）。
                </small>
              </span>
            </div>
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

      {:else if activeSection === 'budget'}
        <div class="setting-block">
          <h3 class="block-title">预算与配额</h3>
          <p class="block-desc">
            预算旋钮面 + 会话消耗仪表。数字旋钮失焦或回车提交，即效写配置并注入运行时环境；
            越界钳制到 1..=64，非法值回默认。多维配额没接口的维度如实标注「暂无接口」，不造假旋钮。
          </p>

          <!-- 会话消耗卡（预算仪表）：真值渲染，无数据位诚实「—」 -->
          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Activity size={13} /> 会话消耗（当前对话）</span>
              <span class="cap-card-count">回合 {sessionTotalsView.turns}</span>
            </div>
            <div class="meter-grid">
              <div class="meter-cell">
                <small>本会话 token（入 / 出）</small>
                <b>{sessionTotalsView.tokens}</b>
              </div>
              <div class="meter-cell">
                <small>提示缓存命中（数 · 率）</small>
                <b>{sessionTotalsView.cache}</b>
              </div>
              <div class="meter-cell">
                <small>回合数</small>
                <b>{sessionTotalsView.turns}</b>
              </div>
              <div class="meter-cell">
                <small>累计耗时</small>
                <b>{sessionTotalsView.duration}</b>
              </div>
            </div>
            <div class="budget-remaining">
              <div class="remaining-title">预算余量（对生效上限求余量；无上限维度显「—」）</div>
              {#each budgetRemaining as row (row.key)}
                <div class="remaining-row">
                  <span class="remaining-label">{row.label}<small>{row.scope}</small></span>
                  <span class="remaining-num">上限 {row.cap}</span>
                  <span class="remaining-num">已耗 {row.used}</span>
                  <span class="remaining-num remaining-strong">余 {row.remaining}</span>
                  {#if row.ratio !== null}
                    <span class="remaining-bar" aria-hidden="true">
                      <span class="remaining-fill" style="width: {Math.round(row.ratio * 100)}%"></span>
                    </span>
                  {/if}
                </div>
              {/each}
            </div>
          </div>

          <!-- 预算旋钮（数字步进器）：回合预算两枚 + 上下文预算一枚，同一条即效缝 -->
          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><Timer size={13} /> 预算旋钮</span>
              <span class="cap-card-count">失焦 / 回车即生效</span>
            </div>
            {#each BUDGET_KNOB_ROWS as row (row.key)}
              {@const spec = BUDGET_KNOB_SPECS[row.key]}
              {@const badgeView = budgetBadges[row.key]}
              <div class="cap-row cap-row-static" class:pending={liveApplyPendingKey === row.key}>
                <span class="cap-icon"><row.icon size={15} /></span>
                <span class="cap-text">
                  <strong>
                    {spec.label}<code class="cap-env">{spec.env}</code>
                    {@render fieldFlag(BUDGET_FLAG_KEYS[row.key])}
                    <span
                      class="cap-default-badge budget-badge"
                      class:on={badgeView.source === 'configured'}
                      class:note={badgeView.source === 'constant'}
                      title={badgeView.note}>{badgeView.label}</span>
                  </strong>
                  <small>{spec.desc} {row.hint}</small>
                </span>
                <span class="cap-slider-wrap">
                  <input
                    type="number"
                    min={spec.min}
                    max={spec.max ?? undefined}
                    step="1"
                    value={budgetDraft[row.key]}
                    aria-label={spec.label}
                    oninput={(e) => {
                      budgetDraft = {
                        ...budgetDraft,
                        [row.key]: (e.currentTarget as HTMLInputElement).value,
                      };
                      markTextDirty(BUDGET_FLAG_KEYS[row.key]);
                    }}
                    onfocusout={() => void submitBudgetKnob(row.key)}
                    onkeydown={(e) => {
                      if (isTextCommitKey(e.key)) {
                        e.preventDefault();
                        void submitBudgetKnob(row.key);
                      }
                    }}
                  />
                </span>
              </div>
              {#if budgetFeedback[row.key]}
                <div class="budget-feedback">
                  <Info size={12} />
                  <span>{budgetFeedback[row.key]}</span>
                </div>
              {/if}
            {/each}
          </div>

          <!-- 多维配额真实可配面（读码结论）：没接口的维度如实「暂无接口」 -->
          <div class="cap-card">
            <div class="cap-card-head">
              <span class="cap-card-title"><SlidersHorizontal size={13} /> 多维配额（Token / 步数 / 花费 / 深度）</span>
              <span class="cap-card-count">0/4 可配</span>
            </div>
            {#each QUOTA_DIMENSIONS as dim (dim.key)}
              <div class="cap-row cap-row-static">
                <span class="cap-icon"><Gauge size={15} /></span>
                <span class="cap-text">
                  <strong>{dim.label}</strong>
                  <small>{dim.reason}</small>
                </span>
                <span class="quota-none" title={dim.evidence}>{QUOTA_DIMENSION_STATUS_LABEL}</span>
              </div>
            {/each}
          </div>

          <!-- 预算耗尽行为：后端无可配置语义 → 如实不出选择器，只出固定语义说明 -->
          <div class="cap-default-card">
            <span class="cap-default-badge note"><Info size={11} /> 固定语义 · 无选择器</span>
            <strong class="cap-default-title">预算耗尽行为</strong>
            <p class="cap-default-text">
              后端没有可配置的耗尽行为语义，这里不出选择器；实际行为是固定语义：
            </p>
            <ul class="budget-behaviors">
              {#each BUDGET_EXHAUSTION.behaviors as behavior}
                <li>{behavior}</li>
              {/each}
            </ul>
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
            <strong class="info-title">工作区目录 {@render fieldFlag('workspace.dir')}</strong>
            <div class="workspace-row">
              <input
                class="workspace-path workspace-input"
                type="text"
                bind:value={workspaceDir}
                oninput={() => markTextDirty('workspace.dir')}
                onfocusout={() => void submitWorkspaceDir()}
                onkeydown={(e) => {
                  if (isTextCommitKey(e.key)) {
                    e.preventDefault();
                    void submitWorkspaceDir();
                  }
                }}
                placeholder="默认 (应用数据目录)"
                aria-label="工作区目录"
              />
              <button class="quiet-button" onclick={() => void openWorkspacePicker()} disabled={workspaceLoading}>
                <Folder size={13} />
                <span>更改</span>
              </button>
            </div>
            {#if workspaceError}
              <p class="field-hint error-hint">{workspaceError}</p>
            {/if}
            <p class="field-hint">路径可直接编辑，失焦或回车提交（失败回填旧值）；新路径在下次启动 sidecar 时生效。</p>
          </div>

          <div class="danger-zone-box">
            <div class="danger-head">
              <AlertTriangle size={16} class="danger-icon" />
              <strong>危险区域 (Danger Zone)</strong>
            </div>
            <p class="danger-desc">清空本地数据将删除浏览器/客户端中存储的会话历史。后端数据库中的长期记忆不会受影响。</p>
            <button class="danger-button" onclick={() => requestDanger('clearLocalData')}>
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
            改动即点/失焦即注入侧车环境生效。
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
                class:pending={liveApplyPendingKey === def.key}
                onclick={() => toggleCap(def)}
                disabled={liveApplyPendingKey !== null}
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
              <!-- 思考模式文本旋钮：整组失焦/回车提交 -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <div
                class="text-group"
                role="group"
                aria-label="思考模式文本旋钮"
                bind:this={reasoningGroupEl}
                onfocusout={(e) =>
                  groupFocusOut(e, reasoningGroupEl, () =>
                    void submitCapabilityText(['reasoningModelFilters', 'reasoningTag'], REASONING_TEXT_KEYS))}
                onkeydown={(e) =>
                  groupKeydown(e, () =>
                    void submitCapabilityText(['reasoningModelFilters', 'reasoningTag'], REASONING_TEXT_KEYS))}
              >
              <div class="cap-row cap-row-static">
                <span class="cap-icon"><Filter size={15} /></span>
                <span class="cap-text">
                  <strong>生效模型过滤器<code class="cap-env">APEIRETH_REASONING_MODEL_FILTERS</code> {@render fieldFlag('reasoning.filters')}</strong>
                  <small>逗号分隔子串白名单（如 deepseek,o3）；留空 = 全部模型生效。</small>
                </span>
                <span class="cap-input-wrap">
                  <input
                    type="text"
                    value={capabilities.reasoningModelFilters}
                    placeholder="全部模型"
                    aria-label="思考模式模型过滤器"
                    oninput={(e) => {
                      setReasoningFilters((e.currentTarget as HTMLInputElement).value);
                      markTextDirty('reasoning.filters');
                    }}
                  />
                </span>
              </div>
              <div class="cap-row cap-row-static">
                <span class="cap-icon"><Tag size={15} /></span>
                <span class="cap-text">
                  <strong>reasoning 标签<code class="cap-env">APEIRETH_REASONING_TAG</code> {@render fieldFlag('reasoning.tag')}</strong>
                  <small>reasoning_content 的字段标签名，默认 think。</small>
                </span>
                <span class="cap-input-wrap">
                  <input
                    type="text"
                    value={capabilities.reasoningTag}
                    aria-label="reasoning 标签"
                    oninput={(e) => {
                      setReasoningTag((e.currentTarget as HTMLInputElement).value);
                      markTextDirty('reasoning.tag');
                    }}
                  />
                </span>
              </div>
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
          这是**模型提供商**的密钥（如 DeepSeek）。密钥存入系统钥匙串，不落盘明文；写入后热更新，无需重启。
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
            onclick={() => requestDanger('deleteStoredKey')}
            disabled={keychainDeleting}
            title="删除已存密钥（需二次确认）"
          >
            <Trash2 size={13} />
            <span>{keychainDeleting ? '删除中…' : '删除已存密钥'}</span>
          </button>
        {/if}
        <button class="quiet-button" onclick={() => showApiKeyModal = false}>取消</button>
        <button class="primary-button" onclick={saveNewApiKey} disabled={keychainSaving}>
          {keychainSaving ? '写入中…' : '写入并应用'}
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

<!-- 危险动作二次确认（第 3 级）：确认后即效（弹层文案来自登记表） -->
{#if dangerConfirm}
  <ConfirmDialog
    open={dangerPending !== null}
    title={dangerConfirm.title}
    message={dangerConfirm.message}
    confirmText={dangerConfirm.confirmText}
    danger={true}
    onConfirm={() => void runDangerAction()}
    onCancel={() => {
      dangerPending = null;
      dangerPayload = '';
    }}
  />
{/if}

<style>
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

  /* ===== 能力中心 · 同类 IM 式分组卡片（2026-10-10 W2/W3 收官批）===== */
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

  /* 「推荐配置」一键预设：按钮 + 说明（能力中心）。 */
  .preset-bar {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .preset-hint {
    font-size: 12px;
    color: var(--muted);
  }
  .preset-actions {
    display: flex;
    gap: 8px;
  }

  /* 文本提交组（第 2 级）：整组失焦/回车提交的容器。display:contents
     不改既有排版，只负责承载组事件与组语义。 */
  .text-group {
    display: contents;
  }

  /* 字段旁即效确认：未提交显「未保存」，提交成功闪 ✓。 */
  .field-flag {
    display: inline-flex;
    align-items: center;
    margin-left: 6px;
    vertical-align: middle;
  }
  .field-dirty {
    font-size: 10px;
    font-weight: 500;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--amber-wash);
    color: var(--amber);
  }
  .field-ack {
    color: #2ecc71;
  }

  .workspace-input {
    flex: 1;
    min-width: 0;
    padding: 7px 10px;
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    color: var(--text);
    font-family: var(--mono);
    font-size: 12px;
    outline: 0;
  }
  .workspace-input:focus {
    border-color: var(--amber-line);
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

  /* 行：图标 + 标题/副文案 + 右侧控制。整行可点（同类设置项语义）。 */
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
  /* 即点即生效 pending：目标行开关脉动、行禁用防连点；失败回滚后由横幅收场。 */
  .cap-row:disabled {
    cursor: default;
    opacity: 0.7;
  }
  .cap-row.pending .cap-switch {
    animation: cap-pending 0.9s ease-in-out infinite;
  }
  @keyframes cap-pending {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
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

  /* 开关（圆钮样式） */
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

  /* ---- 「预算与配额」：会话消耗卡 / 预算余量条 / 旋钮反馈 / 「暂无接口」徽标 ---- */
  .meter-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--line);
  }
  .meter-cell {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .meter-cell small {
    font-size: 11px;
    color: var(--faint);
  }
  .meter-cell b {
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .budget-remaining {
    padding: 12px 14px;
  }
  .remaining-title {
    font-size: 11px;
    letter-spacing: 0.04em;
    color: var(--muted);
    margin-bottom: 8px;
  }
  .remaining-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 5px 0;
  }
  .remaining-label {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 12px;
    color: var(--text);
  }
  .remaining-label small {
    font-size: 10px;
    color: var(--faint);
  }
  .remaining-num {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
  }
  .remaining-strong {
    color: var(--text);
  }
  .remaining-bar {
    flex: none;
    width: 96px;
    height: 5px;
    border-radius: 999px;
    background: var(--surface);
    overflow: hidden;
  }
  .remaining-fill {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--amber);
  }
  .budget-badge {
    margin-left: 8px;
    vertical-align: middle;
  }
  .budget-feedback {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px 10px 40px;
    font-size: 11px;
    color: var(--amber);
  }
  .quota-none {
    flex: none;
    font-family: var(--mono);
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px dashed var(--line-strong);
    color: var(--faint);
  }
  .budget-behaviors {
    margin: 8px 0 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .budget-behaviors li {
    font-size: 12px;
    line-height: 1.6;
    color: var(--muted);
  }
</style>
