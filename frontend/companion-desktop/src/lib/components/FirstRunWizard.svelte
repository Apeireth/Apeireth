<script lang="ts">
  import {Sparkles} from 'lucide-svelte';
  import type {ApeirethConfig, ProviderConfig} from '../types';

  let {
    onComplete,
    onSkip,
  }: {
    onComplete: (config: ApeirethConfig) => void;
    onSkip: () => void;
  } = $props();

  const PRESETS: Array<{
    id: string;
    name: string;
    baseUrl: string;
    model: string;
    hint: string;
  }> = [
    {
      id: 'deepseek',
      name: 'DeepSeek',
      baseUrl: 'https://api.deepseek.com/v1',
      model: 'deepseek-v4-flash',
      hint: '推荐 · 本项目实测路径',
    },
    {
      id: 'openai',
      name: 'OpenAI',
      baseUrl: 'https://api.openai.com/v1',
      model: 'gpt-4o',
      hint: '',
    },
    {
      id: 'minimax',
      name: 'MiniMax',
      baseUrl: 'https://api.minimax.chat/v1',
      model: 'MiniMax-M3',
      hint: '',
    },
    {
      id: 'ollama',
      name: 'Ollama 本地',
      baseUrl: 'http://localhost:11434/v1',
      model: 'llama3.3',
      hint: '无需 key',
    },
    {
      id: 'custom',
      name: '自定义端点',
      baseUrl: '',
      model: '',
      hint: '任意 OpenAI 兼容服务',
    },
  ];

  let presetId = $state('deepseek');
  let apiKey = $state('');
  let baseUrl = $state(PRESETS[0].baseUrl);
  let model = $state(PRESETS[0].model);
  let applying = $state(false);
  let error = $state('');

  function selectPreset(id: string): void {
    presetId = id;
    const preset = PRESETS.find((p) => p.id === id);
    if (preset) {
      baseUrl = preset.baseUrl;
      model = preset.model;
    }
    error = '';
  }

  function finish(): void {
    const endpoint = baseUrl.trim();
    const modelName = model.trim();
    const key = presetId === 'ollama' ? '' : apiKey.trim();
    if (!endpoint) {
      error = '请填写 API 端点';
      return;
    }
    if (!modelName) {
      error = '请填写模型名';
      return;
    }
    if (presetId !== 'ollama' && !key) {
      error = '请填写 API Key（Ollama 本地可留空）';
      return;
    }
    applying = true;
    const provider: ProviderConfig = {
      protocol: 'openai',
      preset: presetId,
      baseUrl: endpoint,
      apiKey: key,
      model: modelName,
    };
    onComplete({
      // 桌面端会由 BackendSupervisor 重解析真实端口
      baseUrl: 'http://127.0.0.1:8080',
      apiKey: '',
      model: modelName,
      provider,
      openaiConfig: {preset: presetId, baseUrl: endpoint, apiKey: key, model: modelName},
    });
  }
</script>

<div class="wizard-backdrop">
  <div class="wizard-card">
    <header class="wizard-head">
      <span class="wizard-icon"><Sparkles size={16} /></span>
      <div>
        <h2>欢迎使用 Apeireth 伙伴</h2>
        <p>三步开始：选服务商 → 填密钥 → 开聊。配置直接注入本地网关（密钥只进内存与侧车环境，不落盘）。</p>
      </div>
    </header>

    <div class="preset-row">
      {#each PRESETS as preset (preset.id)}
        <button
          class="preset-chip"
          class:selected={presetId === preset.id}
          onclick={() => selectPreset(preset.id)}
        >
          <span>{preset.name}</span>
          {#if preset.hint}<small>{preset.hint}</small>{/if}
        </button>
      {/each}
    </div>

    <div class="field">
      <label for="wizard-url">API 端点</label>
      <input id="wizard-url" type="text" bind:value={baseUrl} placeholder="https://api.deepseek.com/v1" />
    </div>
    <div class="field">
      <label for="wizard-model">模型</label>
      <input id="wizard-model" type="text" bind:value={model} placeholder="deepseek-v4-flash" />
    </div>
    {#if presetId !== 'ollama'}
      <div class="field">
        <label for="wizard-key">API Key</label>
        <input id="wizard-key" type="password" bind:value={apiKey} placeholder="sk-…（仅内存，不落盘）" />
      </div>
    {/if}

    {#if error}<p class="wizard-error">{error}</p>{/if}

    <footer class="wizard-foot">
      <button class="skip-btn" onclick={onSkip} disabled={applying}>以后再说</button>
      <button class="start-btn" onclick={finish} disabled={applying}>
        {applying ? '正在应用（网关重启中）…' : '开始使用'}
      </button>
    </footer>
  </div>
</div>

<style>
  .wizard-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.72);
    backdrop-filter: blur(4px);
    display: grid;
    place-items: center;
    z-index: 1200;
  }
  .wizard-card {
    width: min(560px, calc(100vw - 40px));
    max-height: calc(100vh - 60px);
    overflow-y: auto;
    background: var(--surface-1, #101418);
    border: 1px solid var(--line, #2a323c);
    border-radius: 14px;
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .wizard-head {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .wizard-icon {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: var(--accent-wash, rgba(110, 168, 254, 0.15));
    color: var(--accent, #6ea8fe);
    flex-shrink: 0;
  }
  .wizard-head h2 {
    margin: 0 0 4px;
    font-size: 16px;
  }
  .wizard-head p {
    margin: 0;
    font-size: 12px;
    color: var(--faint, #8b97a5);
    line-height: 1.6;
  }
  .preset-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .preset-chip {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid var(--line, #2a323c);
    background: var(--surface-2, #161b21);
    color: var(--text, #e6edf3);
    cursor: pointer;
    text-align: left;
  }
  .preset-chip.selected {
    border-color: var(--accent, #6ea8fe);
    background: var(--accent-wash, rgba(110, 168, 254, 0.12));
  }
  .preset-chip small {
    font-size: 10px;
    color: var(--faint, #8b97a5);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field label {
    font-size: 12px;
    color: var(--faint, #8b97a5);
  }
  .field input {
    padding: 9px 12px;
    border-radius: 8px;
    border: 1px solid var(--line, #2a323c);
    background: var(--surface-2, #161b21);
    color: var(--text, #e6edf3);
    font-size: 13px;
  }
  .wizard-error {
    margin: 0;
    font-size: 12px;
    color: var(--danger, #e5484d);
  }
  .wizard-foot {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .skip-btn,
  .start-btn {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    cursor: pointer;
  }
  .skip-btn {
    background: transparent;
    border: 1px solid var(--line, #2a323c);
    color: var(--faint, #8b97a5);
  }
  .start-btn {
    background: var(--accent, #6ea8fe);
    border: 1px solid transparent;
    color: #0b1014;
    font-weight: 600;
  }
  .skip-btn:disabled,
  .start-btn:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
