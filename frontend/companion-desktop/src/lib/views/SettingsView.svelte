<script lang="ts">
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
  } from 'lucide-svelte';
  import PageHeader from '../../components/PageHeader.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import ThemeSettingsPanel from '../components/ThemeSettingsPanel.svelte';
  import SessionModelPicker from '../components/SessionModelPicker.svelte';
  import WorkspacePickerModal from '../components/WorkspacePickerModal.svelte';
  import ErrorSolutionBanner from '../components/ErrorSolutionBanner.svelte';
  import type {ApeirethConfig, RuntimeHealthReport, ProviderProtocol, ProviderConfig, PersonaProfile, CapabilityToggles, ModelInfo, AdminConfigPatch} from '../types';
  import {DEFAULT_CAPABILITY_TOGGLES} from '../types';
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
    | 'memory'
    | 'capabilities'
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
  let capabilities = $state<CapabilityToggles>({
    ...DEFAULT_CAPABILITY_TOGGLES,
    ...(config.capabilities ?? {}),
  });

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

  const CAPABILITY_TOGGLE_DEFS: Array<{key: keyof CapabilityToggles; label: string; desc: string}> = [
    {key: 'shell', label: 'Shell 命令工具', desc: '模型可提议本地命令，每次执行前需你在审批面板确认（fail-closed 授权）。'},
    {key: 'fetch', label: '网络读取工具', desc: '模型可发起公网 GET 请求（只读，无凭据转发）。'},
    {key: 'organs', label: '器官链（9 organs）', desc: '回合后跑 9 器官反事实推演/好奇心/情绪记忆等（AfterTurn，fail-open，不阻塞回复）。'},
    {key: 'preferenceLearning', label: '偏好学习', desc: '回合后把主人偏好写成双索引记忆（原始 + 主题簇），后续召回按主题展开。'},
    {key: 'judge', label: '评审 (Judge)', desc: '模型回复后由评审模块判一次（每次评审最多一次 side-call）。'},
    {key: 'council', label: '议会 (Council)', desc: '模型回复后由 7 个 advisor 并行评审（整体 60s 超时）。'},
  ];

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
    {id: 'memory', label: '记忆策略', icon: Layers3},
    {id: 'capabilities', label: '高级能力', icon: Sparkles},
    {id: 'tools', label: '工具与权限策略', icon: Shield},
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
</script>

<section class="settings-view">
  <PageHeader
    eyebrow="首选项"
    title="系统设置"
    subtitle="配置模型提供商（Anthropic / OpenAI 兼容协议）、后端连接、权限与数据。"
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

      {:else if activeSection === 'memory'}
        <div class="setting-block">
          <h3 class="block-title">记忆流与提取策略</h3>
          <p class="block-desc">伙伴常驻后台记忆提炼与做梦机制。</p>

          <div class="info-card">
            <strong class="info-title">6 历史流体系</strong>
            <p class="info-text">包含会话历史、偏好模型、事实抽取、反思沉淀、经验总结与图谱关联。</p>
          </div>

          <div class="info-card">
            <strong class="info-title">后台做梦与反思循环 (Dream & Reflection)</strong>
            <p class="info-text">伴随常驻 daemon 运行，安静期后自动触发做梦提炼与经验入库。</p>
          </div>
        </div>

      {:else if activeSection === 'capabilities'}
        <div class="setting-block">
          <h3 class="block-title">高级能力（后端旋钮）</h3>
          <p class="block-desc">
            保存后注入侧车环境并重启网关（配置没变不会重启）。全部默认关闭，
            逐项显式开启；shell/fetch 开启后每次调用仍走人工审批。
          </p>

          <div class="form-group">
            <label for="cognitive-depth">认知深度</label>
            <select id="cognitive-depth" bind:value={cognitiveDepth} onchange={() => applyCognitiveDepth()}>
              <option value="light">轻量 — judge 关 / council 关 (默认)</option>
              <option value="balanced">平衡 — judge 开 / council 关</option>
              <option value="deep">深度 — judge 开 / council 开</option>
              <option value="custom">自定义 — 手动设置 judge/council</option>
            </select>
            <small class="field-hint">judge/council 会在回复后追加评审，增加延迟与 token 消耗。</small>
          </div>

          {#each CAPABILITY_TOGGLE_DEFS as item (item.key)}
            <label class="toggle-row">
              <span class="toggle-text">
                <strong>{item.label}</strong>
                <small>{item.desc}</small>
              </span>
              <input
                type="checkbox"
                checked={capabilities[item.key]}
                onchange={(e) => {
                  handleCapabilityToggle(item.key, (e.target as HTMLInputElement).checked);
                }}
              />
            </label>
          {/each}

          <div class="info-card">
            <strong class="info-title">fail-closed 语义</strong>
            <p class="info-text">
              关闭（默认）时不注入任何环境变量，后端能力完全不存在；开启只注入
              "1"。shell/fetch 有 require_approval 治理标，开启 ≠ 无审批执行。
            </p>
          </div>
        </div>

      {:else if activeSection === 'tools'}
        <div class="setting-block">
          <h3 class="block-title">工具权限与安全架构</h3>
          <p class="block-desc">高危特权工具（如 FileOperator、ShellExec）需要主人授权。</p>

          <div class="form-group">
            <label for="permission-preset">全局权限预设</label>
            <select id="permission-preset" bind:value={permissionPreset}>
              <option value="read_only">read_only — 只读</option>
              <option value="standard">standard — 标准 (推荐)</option>
              <option value="full">full — 完全权限</option>
            </select>
            <small class="field-hint">作为新会话默认 (已生效)：仅对新会话生效，已有会话请在会话内改。</small>
          </div>

          <div class="info-card">
            <strong class="info-title">权限洋葱与即时授权 (On-demand Permission Pack)</strong>
            <p class="info-text">
              为保障安全性，Master Token 绝不持久化保存在客户端存储中。当特权工具被拒绝并产生待批授权请求时，主人在「工具管理」页面输入 Token 即时完成时效性签发。
            </p>
          </div>

          <div class="info-card">
            <strong class="info-title">宪法评审 (MiniMaxConstitutionLlm)</strong>
            <p class="info-text">高危工具执行前自动按 E 层进行安全判案，杜绝越权或有害操作。</p>
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
          <h3 class="block-title">开发者与协议信息</h3>
          <p class="block-desc">技术参数与运行时契约规范。</p>

          <div class="info-card">
            <strong class="info-title">Agent Runtime Contract (§15)</strong>
            <p class="info-text">
              UI 仅面对标准事件流 (run-start, text-delta, reasoning-delta, tool-call, tool-result, message-end)，不裸碰底层 HTTP/SSE 协议。
            </p>
          </div>

          <div class="form-group">
            <label for="raw-config-json">客户端配置 (JSON)</label>
            <pre class="code-box">{JSON.stringify({baseUrl: config.baseUrl, model: providerModel, provider: config.provider, hasApiKey}, null, 2)}</pre>
          </div>
        </div>
      {/if}

    </div>

    <!-- 常驻保存栏：填完配置点这里（保存会重启本地网关以应用配置） -->
    <div class="settings-save-bar">
      <span class="save-bar-hint">
        {saveSuccess ? '✓ 已保存，本地网关已应用新配置' : '填好配置后点"保存设置"（无重启热应用，不支持时自动重启网关）'}
      </span>
      <button class="primary-button save-bar-btn" onclick={handleSaveSettings}>
        <Check size={14} />
        <span>{saveSuccess ? '已保存！' : '保存设置'}</span>
      </button>
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
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 14px;
    padding: 10px 18px;
    border-top: 1px solid var(--line, #2a323c);
    background: var(--surface-1, #101418);
    flex-shrink: 0;
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
  .model-input-row {
    display: flex;
    gap: 8px;
  }
  .model-input-row input {
    flex: 1;
  }
  .models-chip-list {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
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
  /* 高级能力开关行 */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 8px;
    cursor: pointer;
  }
  .toggle-text {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .toggle-text strong {
    font-size: 13px;
  }
  .toggle-text small {
    font-size: 11px;
    color: var(--faint);
    line-height: 1.5;
  }
  .toggle-row input[type='checkbox'] {
    width: 16px;
    height: 16px;
    accent-color: var(--accent, #6ea8fe);
    flex-shrink: 0;
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
  .info-text {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.6;
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
