<script lang="ts">
  import {Settings} from 'lucide-svelte';
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
</script>

<section class="view">
  <PageHeader
    eyebrow="配置"
    title="后端连接"
    subtitle="连接 Apeireth 的 OpenAI 兼容端点 (companion_serve :8090 或 apeireth-api :8080)。"
  >
    <button class="primary-button" onclick={onSave}><Settings size={14}/>保存</button>
  </PageHeader>
  <div class="settings-form">
    <label>
      <span>端点地址</span>
      <input bind:value={editBaseUrl} placeholder="http://127.0.0.1:8090" />
    </label>
    <label>
      <span>API Key</span>
      <input bind:value={editApiKey} type="password" placeholder="后端持有的 key (任意非空串)" />
    </label>
    <label>
      <span>模型</span>
      <div class="model-row">
        <input bind:value={editModel} placeholder="MiniMax-M3" />
        <button class="quiet-button" onclick={onRefreshModels} title="拉取模型列表">刷新</button>
      </div>
    </label>
    {#if modelsList.length}
      <div class="model-list">
        {#each modelsList as model}
          <button class="model-chip" onclick={() => editModel = model}>{model}</button>
        {/each}
      </div>
    {/if}
    <div class="conn-status">
      <StatusDot
        size="small"
        off={healthState !== 'ready' && healthState !== 'generating'}
        active={healthState === 'generating'}
      />
      <span>{healthLabel[healthState]} · {config.baseUrl}</span>
    </div>
    {#if error}
      <p class="error-banner" role="alert">{error}</p>
    {/if}
  </div>
</section>
