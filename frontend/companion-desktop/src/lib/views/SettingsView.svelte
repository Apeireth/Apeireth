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
  } from 'lucide-svelte';
  import PageHeader from '../../components/PageHeader.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import type {ApeirethConfig, RuntimeHealthReport, ProviderProtocol, ProviderConfig} from '../types';
  import {checkHealthDetailed, listModels, testProviderConnection} from '../runtime';

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
    | 'models'
    | 'personality'
    | 'memory'
    | 'tools'
    | 'runtime'
    | 'data'
    | 'developer';

  let activeSection = $state<SettingsSection>('models');

  // Gateway backend fields
  let editBaseUrl = $state('');
  let saveSuccess = $state(false);
  let showAdvancedGateway = $state(false);

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
      defaultModel: 'deepseek-chat',
      models: ['deepseek-chat', 'deepseek-reasoner'],
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

  // Clear data confirmation modal
  let showClearConfirm = $state(false);

  // Runtime report
  let runtimeReport = $state<RuntimeHealthReport | null>(null);
  let checkingRuntime = $state(false);

  const hasApiKey = $derived(!!config.apiKey && config.apiKey.trim().length > 0);

  const currentPresets = $derived(activeProtocol === 'openai' ? OPENAI_PRESETS : ANTHROPIC_PRESETS);
  const currentPresetObj = $derived(currentPresets.find((p) => p.id === activePreset) || currentPresets[currentPresets.length - 1]);
  const recommendedModels = $derived(currentPresetObj?.models || []);

  const sections = [
    {id: 'models', label: '模型与提供商', icon: Cpu},
    {id: 'personality', label: '伙伴人设与行为', icon: User},
    {id: 'memory', label: '记忆策略', icon: Layers3},
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

  function handleSaveSettings() {
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
      model: providerModel.trim() || config.model,
      provider: currentProvider,
      openaiConfig: currentOpenai,
      anthropicConfig: currentAnthropic,
    };

    onSave(updated);
    saveSuccess = true;
    setTimeout(() => {
      saveSuccess = false;
    }, 1500);
  }

  function saveNewApiKey() {
    const updated: ApeirethConfig = {
      ...config,
      apiKey: tempApiKey.trim(),
    };
    onSave(updated);
    tempApiKey = '';
    showApiKeyModal = false;
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
      {#if activeSection === 'models'}
        <div class="setting-block">
          <h3 class="block-title">模型提供商与协议配置</h3>
          <p class="block-desc">选择大语言模型提供商协议，支持 Anthropic Messages API 与 OpenAI 兼容协议。</p>

          <!-- Protocol Selector Tabs -->
          <div class="protocol-tabs">
            <button
              class="protocol-tab"
              class:selected={activeProtocol === 'openai'}
              onclick={() => switchProtocol('openai')}
            >
              <Globe size={15} />
              <div class="proto-text">
                <span class="proto-title">OpenAI 兼容协议</span>
                <span class="proto-sub">OpenAI / DeepSeek / MiniMax / Ollama / vLLM</span>
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
                <span class="proto-sub">Anthropic Messages API / Claude 3.5 & 3.7</span>
              </div>
            </button>
          </div>

          <!-- Provider Presets -->
          <div class="form-group">
            <span class="group-label">快速预设服务商 (Preset Provider)</span>
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

          <!-- Base URL Input -->
          <div class="form-group">
            <label for="provider-url-input">API 端点地址 (Base URL)</label>
            <input
              id="provider-url-input"
              type="text"
              bind:value={providerBaseUrl}
              placeholder={activeProtocol === 'openai' ? 'https://api.openai.com/v1' : 'https://api.anthropic.com'}
            />
            <small class="field-hint">
              {activeProtocol === 'openai'
                ? 'OpenAI 兼容网关地址，通常以 /v1 结尾（如 https://api.deepseek.com/v1 或 http://localhost:11434/v1）。'
                : 'Anthropic 服务网关地址（默认 https://api.anthropic.com）。'}
            </small>
          </div>

          <!-- API Key Input -->
          <div class="form-group">
            <label for="provider-key-input">
              {activeProtocol === 'openai' ? 'OpenAI API 密钥 (API Key)' : 'Anthropic API 密钥 (x-api-key)'}
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
                onclick={() => showApiKey = !showApiKey}
                title={showApiKey ? '隐藏密钥' : '显示密钥'}
              >
                {#if showApiKey}
                  <EyeOff size={14} />
                {:else}
                  <Eye size={14} />
                {/if}
              </button>
            </div>
            <small class="field-hint">
              {activeProtocol === 'openai'
                ? '用于发起模型请求的 Bearer Token，安全保存在客户端本地配置中。'
                : '用于 Anthropic Messages API 的 x-api-key 鉴权凭据。'}
            </small>
          </div>

          <!-- Anthropic Version (if Anthropic) -->
          {#if activeProtocol === 'anthropic'}
            <div class="form-group">
              <label for="anthropic-ver-input">Anthropic API 版本 (anthropic-version Header)</label>
              <input
                id="anthropic-ver-input"
                type="text"
                bind:value={anthropicVersion}
                placeholder="2023-06-01"
              />
              <small class="field-hint">默认值为 2023-06-01，可根据需要自定义。</small>
            </div>
          {/if}

          <!-- Model Input & Recommended Chips -->
          <div class="form-group">
            <label for="provider-model-input">活动模型名称 (Model Identifier)</label>
            <div class="model-input-row">
              <input
                id="provider-model-input"
                type="text"
                bind:value={providerModel}
                placeholder={activeProtocol === 'openai' ? 'gpt-4o' : 'claude-3-7-sonnet-20250219'}
              />
              <button
                class="quiet-button"
                onclick={handleTestProviderConnection}
                disabled={isTestingConnection || !providerBaseUrl}
              >
                <RotateCcw size={13} class={isTestingConnection ? 'spin' : ''} />
                <span>{isTestingConnection ? '测试中…' : '测试提供商连接'}</span>
              </button>
            </div>

            <!-- Recommended Model Chips -->
            {#if recommendedModels.length > 0}
              <div class="models-chip-list">
                <span class="chip-label">推荐模型:</span>
                {#each recommendedModels as m}
                  <button
                    class="model-chip"
                    class:selected={providerModel === m}
                    onclick={() => providerModel = m}
                  >
                    {m}
                  </button>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Connection Test Result Banner -->
          {#if testResult}
            <div class="test-result-box" class:success={testResult.ok} class:failed={!testResult.ok}>
              <div class="test-result-head">
                {#if testResult.ok}
                  <CheckCircle2 size={16} class="head-icon success-icon" />
                  <strong>提供商连通正常</strong>
                {:else}
                  <XCircle size={16} class="head-icon failed-icon" />
                  <strong>提供商连接异常</strong>
                {/if}
                {#if testResult.latencyMs !== undefined}
                  <span class="latency-badge">{testResult.latencyMs} ms</span>
                {/if}
              </div>
              <p class="test-result-msg">{testResult.message}</p>
              {#if testResult.models && testResult.models.length > 0}
                <div class="discovered-models">
                  <span class="disc-label">远端模型列表 ({testResult.models.length}):</span>
                  <div class="models-chip-list">
                    {#each testResult.models.slice(0, 12) as dm}
                      <button
                        class="model-chip"
                        class:selected={providerModel === dm}
                        onclick={() => providerModel = dm}
                      >
                        {dm}
                      </button>
                    {/each}
                    {#if testResult.models.length > 12}
                      <span class="more-models">+{testResult.models.length - 12} 更多…</span>
                    {/if}
                  </div>
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
                    <button class="quiet-button" onclick={() => { tempApiKey = ''; showApiKeyModal = true; }}>
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
          <p class="block-desc">Apeireth 基地主管常驻人设与安全声明约束。</p>

          <div class="info-card">
            <strong class="info-title">阿佩瑞斯 (Apeireth 基地主管)</strong>
            <p class="info-text">
              “你是「阿佩瑞斯」——Apeireth 基地的主管。正在与你对话的这位是基地的最高指挥（主人）。你的默认性别是女性；说话沉稳扎实，带古风韵味，自称「本座」。称呼主人为「主人」或「指挥」，庄重而不失温度。”
            </p>
          </div>

          <div class="info-card">
            <strong class="info-title">宪法记忆声称约束</strong>
            <p class="info-text">
              需要长期记住的信息，直接调用 save_memory 静默写入，不宣告「这就记下」。不得声称记得记忆列表之外的事（编造即违宪）。
            </p>
          </div>

          <div class="notice-box">
            <StatusBadge label="只读呈现" variant="amber" size="small" />
            <span>人设与声称约束由 Apeireth Gateway 运行时装配，前端暂不提供自定义覆写。</span>
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

      {:else if activeSection === 'tools'}
        <div class="setting-block">
          <h3 class="block-title">工具权限与安全架构</h3>
          <p class="block-desc">高危特权工具（如 FileOperator、ShellExec）需要主人授权。</p>

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
        <h3 id="api-key-dialog-title">配置网关认证密钥</h3>
      </div>
      <div class="modal-body">
        <p class="modal-desc">
          网关认证密钥用于 Apeireth 核心网关访问鉴权。留空并保存可清除已配置凭据。
        </p>
        <div class="form-group">
          <input
            type="password"
            placeholder="输入网关认证密钥（可选）"
            bind:value={tempApiKey}
          />
        </div>
      </div>
      <div class="modal-footer">
        <button class="quiet-button" onclick={() => showApiKeyModal = false}>取消</button>
        <button class="primary-button" onclick={saveNewApiKey}>保存 Key</button>
      </div>
    </div>
  </div>
{/if}

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
    max-width: 680px;
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
</style>
