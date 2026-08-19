<script lang="ts">
  import {Activity, Check, Key, RefreshCw, Save, Server, Shield, Sparkles, User} from 'lucide-svelte';
  import PageHeader from '../../components/PageHeader.svelte';
  import StatusDot from '../../components/StatusDot.svelte';
  import type {ApeirethConfig, HealthState} from '../../lib/types';

  let {
    config,
    healthState,
    healthLabel,
    editBaseUrl = $bindable(''),
    editApiKey = $bindable(''),
    editModel = $bindable(''),
    modelsList,
    error = '',
    onSave,
    onRefreshModels,
  }: {
    config: ApeirethConfig;
    healthState: HealthState;
    healthLabel: Record<HealthState, string>;
    editBaseUrl: string;
    editApiKey: string;
    editModel: string;
    modelsList: string[];
    error: string;
    onSave: () => void;
    onRefreshModels: () => void;
  } = $props();

  type SettingsTab = 'connection' | 'security' | 'persona' | 'diagnostics';
  let activeTab = $state<SettingsTab>('connection');

  let masterToken = $state(localStorage.getItem('apeireth-master-token') || '');
  let subjectId = $state(localStorage.getItem('apeireth-subject-id') || 'companion-main');
  let savedHint = $state(false);
  let pingMs = $state<number | null>(null);
  let pinging = $state(false);

  function handleSaveAll(): void {
    localStorage.setItem('apeireth-master-token', masterToken.trim());
    localStorage.setItem('apeireth-subject-id', subjectId.trim());
    onSave();
    savedHint = true;
    setTimeout(() => { savedHint = false; }, 2500);
  }

  async function testPing(): Promise<void> {
    pinging = true;
    const t0 = performance.now();
    try {
      const res = await fetch(`${editBaseUrl.replace(/\/+$/, '')}/health`);
      if (res.ok) {
        pingMs = Math.round(performance.now() - t0);
      } else {
        pingMs = -1;
      }
    } catch {
      pingMs = -1;
    } finally {
      pinging = false;
    }
  }
</script>

<section class="view">
  <PageHeader
    eyebrow="系统配置"
    title="设置与首选项"
    subtitle="配置 Apeireth 后端连接、安全凭据、人格基线与开发者诊断。"
  >
    <button class="primary-button" onclick={handleSaveAll}>
      <Save size={14} />保存配置
    </button>
  </PageHeader>

  <div class="settings-tabs">
    <button
      class:active={activeTab === 'connection'}
      onclick={() => activeTab = 'connection'}
    >
      <Server size={14} />后端连接
    </button>
    <button
      class:active={activeTab === 'security'}
      onclick={() => activeTab = 'security'}
    >
      <Shield size={14} />权限洋葱凭据
    </button>
    <button
      class:active={activeTab === 'persona'}
      onclick={() => activeTab = 'persona'}
    >
      <User size={14} />主体与人格
    </button>
    <button
      class:active={activeTab === 'diagnostics'}
      onclick={() => activeTab = 'diagnostics'}
    >
      <Activity size={14} />系统诊断
    </button>
  </div>

  {#if savedHint}
    <div class="save-success-banner" role="status">
      <Check size={14} />
      <span>配置已保存，运行时已完成重载</span>
    </div>
  {/if}

  {#if error}
    <p class="error-banner" role="alert">{error}</p>
  {/if}

  <div class="settings-body">
    {#if activeTab === 'connection'}
      <div class="settings-card">
        <h3>Apeireth 服务端点</h3>
        <p class="section-desc">连接本地或远程 companion_serve 守护进程或 Apeireth Gateway。</p>
        
        <label class="field-label">
          <span>端点 URL</span>
          <input bind:value={editBaseUrl} placeholder="http://127.0.0.1:8090" />
        </label>

        <label class="field-label">
          <span>API Key</span>
          <input bind:value={editApiKey} type="password" placeholder="后端持有的鉴权 Key (任意非空串)" />
        </label>

        <label class="field-label">
          <span>模型名称 (Model)</span>
          <div class="model-row">
            <input bind:value={editModel} placeholder="MiniMax-M3" />
            <button class="quiet-button" onclick={onRefreshModels} title="拉取模型列表">
              <RefreshCw size={13} />拉取列表
            </button>
          </div>
        </label>

        {#if modelsList.length}
          <div class="model-chips-box">
            <span class="chips-label">可用模型:</span>
            <div class="model-list">
              {#each modelsList as model}
                <button
                  type="button"
                  class="model-chip"
                  class:selected={editModel === model}
                  onclick={() => editModel = model}
                >
                  {model}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {:else if activeTab === 'security'}
      <div class="settings-card">
        <h3>Master Token (主人授权权)</h3>
        <p class="section-desc">
          Apeireth 权限洋葱机制要求高危工具 (Tier 3) 由主人显式放行。
          Master Token 仅保存在主人桌面客户端，AI 自身无法获取。
        </p>

        <label class="field-label">
          <span>Master Token</span>
          <input
            bind:value={masterToken}
            type="password"
            placeholder="APEIRETH_MASTER_TOKEN 环境变量对应值"
          />
        </label>

        <div class="security-info-box">
          <Shield size={16} class="ok" />
          <div>
            <strong>权限洋葱防护已激活</strong>
            <p>低危只读工具自动放行；写操作与执行类工具需主人凭 Master Token 授权放行。</p>
          </div>
        </div>
      </div>
    {:else if activeTab === 'persona'}
      <div class="settings-card">
        <h3>主体会话标识 (Continuity Anchor)</h3>
        <p class="section-desc">
          指定与后端记忆库关联的主体会话 (Subject ID)。
          Apeireth 依赖此 ID 区分主人的核心记忆与特定项目记忆。
        </p>

        <label class="field-label">
          <span>Subject ID (X-Apeireth-Continuity)</span>
          <input bind:value={subjectId} placeholder="companion-main" />
        </label>

        <div class="persona-preview-box">
          <Sparkles size={14} class="exec-icon-accent" />
          <div>
            <strong>L0 常驻人格基线</strong>
            <p>“阿佩瑞斯 (Apeireth) — 一个诚实、有记忆、不断自成长的桌面伙伴。不说空话，不伪造状态。”</p>
          </div>
        </div>
      </div>
    {:else}
      <div class="settings-card">
        <h3>服务健康度与网络诊断</h3>
        <p class="section-desc">检测当前客户端与 Apeireth 后端的网络连通度与响应延迟。</p>

        <div class="diagnostics-status-row">
          <div class="conn-status-item">
            <StatusDot
              size="normal"
              off={healthState !== 'ready' && healthState !== 'generating'}
              active={healthState === 'generating'}
            />
            <strong>{healthLabel[healthState]}</strong>
            <small>{config.baseUrl}</small>
          </div>

          <button class="primary-button ping-btn" onclick={testPing} disabled={pinging}>
            {pinging ? '测速中…' : '测试端点延迟'}
          </button>
        </div>

        {#if pingMs !== null}
          <div class="ping-result-box" class:bad={pingMs < 0}>
            {#if pingMs >= 0}
              <span>端点响应延迟: <strong>{pingMs} ms</strong> (健康)</span>
            {:else}
              <span>端点连通失败，请检查 companion_serve 是否正在运行。</span>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</section>
