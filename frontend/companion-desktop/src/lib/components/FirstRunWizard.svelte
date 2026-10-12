<script lang="ts">
  // 首次运行引导（OOBE）—— 应用内弹窗、分步依次提示。
  //
  // 分流：普通用户 = 预设值 + 使用中自动调参（推荐配置六旋钮 + selfTuning）；
  //      专业用户 = 只做介绍（参考卡），不写任何预设 / 配置 / 印象。
  // 流程（依次提示）：欢迎/介绍 → 分流 →（普通：连接模型服务 → 自我描述词 →
  // 预设确认；专业：参考卡）。纯逻辑与文案在 ../onboarding.ts（可单测）。
  //
  // 对外接口与 v1 单步向导完全一致（onComplete / onSkip）——App.svelte 零改动。
  import {Sparkles, ChevronLeft, ChevronRight, Check} from 'lucide-svelte';
  import type {ApeirethConfig} from '../types';
  import {setProviderKey} from '../tauri-bridge';
  import {
    INTRO_BULLETS,
    INTRO_PLAN,
    PRO_GUIDE_LINES,
    buildWizardConfig,
    stepsFor,
    writeSelfDescription,
    type OnboardingStep,
    type OnboardingTier,
  } from '../onboarding';

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

  let tier = $state<OnboardingTier | null>(null);
  let stepIndex = $state(0);
  let presetId = $state('deepseek');
  let apiKey = $state('');
  let baseUrl = $state(PRESETS[0].baseUrl);
  let model = $state(PRESETS[0].model);
  let selfDescription = $state('');
  let autoTune = $state(true);
  let applying = $state(false);
  let error = $state('');

  const steps = $derived(stepsFor(tier));
  const step = $derived(steps[Math.min(stepIndex, steps.length - 1)] as OnboardingStep);

  const STEP_TITLES: Record<OnboardingStep, {title: string; sub: string}> = {
    welcome: {title: '欢迎使用 Apeireth 伙伴', sub: '先把「这是什么」讲清楚，约 2 分钟'},
    tier: {title: '你是哪类用户？', sub: '不同轨道，不同待遇——之后随时可改'},
    api: {title: '连接模型服务', sub: '密钥只进系统钥匙串，不落盘明文'},
    identity: {title: '自我描述词', sub: '他如何理解你——随每次对话带在身上'},
    preset: {title: '普通用户预设值', sub: '一次配好，使用中自动调整参数'},
    'pro-guide': {title: '专业用户参考', sub: '只做介绍——一切由你自己掌控'},
  };

  function selectPreset(id: string): void {
    presetId = id;
    const preset = PRESETS.find((p) => p.id === id);
    if (preset) {
      baseUrl = preset.baseUrl;
      model = preset.model;
    }
    error = '';
  }

  function pickTier(next: OnboardingTier): void {
    tier = next;
    error = '';
  }

  function validateConnection(): boolean {
    const endpoint = baseUrl.trim();
    const modelName = model.trim();
    const key = presetId === 'ollama' ? '' : apiKey.trim();
    if (!endpoint) {
      error = '请填写 API 端点';
      return false;
    }
    if (!modelName) {
      error = '请填写模型名';
      return false;
    }
    if (presetId !== 'ollama' && !key) {
      error = '请填写 API Key（Ollama 本地可留空）';
      return false;
    }
    return true;
  }

  function next(): void {
    error = '';
    if (step === 'tier' && !tier) {
      error = '请选择一种使用方式';
      return;
    }
    if (step === 'api' && !validateConnection()) {
      return;
    }
    stepIndex = Math.min(stepIndex + 1, steps.length - 1);
  }

  function back(): void {
    error = '';
    stepIndex = Math.max(0, stepIndex - 1);
  }

  async function finishCasual(): Promise<void> {
    error = '';
    if (!validateConnection()) {
      stepIndex = steps.indexOf('api');
      return;
    }
    applying = true;
    const key = presetId === 'ollama' ? '' : apiKey.trim();
    // 密钥入系统钥匙串（不落盘明文）：重启后由 BackendSupervisor 的
    // keychain 回填自动恢复，无需重输。非桌面环境返回 false，内存路径不变。
    if (key) {
      const family = presetId === 'minimax' ? 'minimax' : presetId === 'anthropic' ? 'anthropic' : 'openai';
      await setProviderKey(family, key);
    }
    // 自我描述词 → 用户印象（写侧如实抛：失败保输入并给可读理由，不静默丢）。
    if (selfDescription.trim()) {
      try {
        writeSelfDescription(selfDescription);
      } catch (caught) {
        applying = false;
        error = `自我描述词保存失败：${caught instanceof Error ? caught.message : String(caught)}（清空该栏可跳过）`;
        return;
      }
    }
    onComplete(
      buildWizardConfig(
        {presetId, baseUrl: baseUrl.trim(), model: model.trim(), apiKey: key},
        autoTune,
      ),
    );
  }

  function finishPro(): void {
    // 专业用户：只做介绍，不写任何预设 / 配置 / 印象。
    onSkip();
  }
</script>

<div class="wizard-backdrop">
  <div class="wizard-card">
    <header class="wizard-head">
      <span class="wizard-icon"><Sparkles size={16} /></span>
      <div>
        <h2>{STEP_TITLES[step].title}</h2>
        <p>{STEP_TITLES[step].sub}</p>
      </div>
    </header>

    <!-- 分步进度（依次提示） -->
    <ol class="step-dots" aria-label="引导进度">
      {#each steps as s, i (s)}
        <li class:active={i === stepIndex} class:done={i < stepIndex}>
          {#if i < stepIndex}
            <Check size={11} />
          {:else}
            {i + 1}
          {/if}
        </li>
      {/each}
    </ol>

    {#if step === 'welcome'}
      <section class="step-body">
        <p class="lead">Apeireth 是一个本地优先的 AI 伙伴运行时——所有对话、审批、审计、记忆都留在你自己的机器上。</p>
        <ul class="intro-list">
          {#each INTRO_BULLETS as line (line)}
            <li>{line}</li>
          {/each}
        </ul>
        <div class="plan-box">
          {#each INTRO_PLAN as line (line)}
            <p>{line}</p>
          {/each}
        </div>
      </section>
    {:else if step === 'tier'}
      <section class="step-body">
        <div class="tier-cards">
          <button
            class="tier-card"
            class:selected={tier === 'casual'}
            onclick={() => pickTier('casual')}
          >
            <h3>普通用户<span class="tier-tag">推荐</span></h3>
            <p>引导为你写好预设值（记忆核心族、预算族），使用中<strong>自动调整参数</strong>——上手就用，细节随时可改。</p>
          </button>
          <button class="tier-card" class:selected={tier === 'pro'} onclick={() => pickTier('pro')}>
            <h3>专业用户</h3>
            <p><strong>只做介绍</strong>：给你完整配置参考，不写任何预设——一切旋钮由你自己决定。</p>
          </button>
        </div>
      </section>
    {:else if step === 'api'}
      <section class="step-body">
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
            <input id="wizard-key" type="password" bind:value={apiKey} placeholder="sk-…（保存到系统钥匙串，不落盘明文）" />
          </div>
        {/if}
      </section>
    {:else if step === 'identity'}
      <section class="step-body">
        <div class="field">
          <label for="wizard-self">用一两句话描述你自己，以及希望他如何与你相处</label>
          <textarea
            id="wizard-self"
            rows="4"
            bind:value={selfDescription}
            placeholder="例：「叫我小明。我是学生，常用中文，回答请简洁可靠。」"
          ></textarea>
        </div>
        <p class="hint">
          它会成为「用户印象」的初始档案——阿佩瑞斯带着这份理解与你相处（可在设置里随时改或清空；留空 = 之后再补）。
        </p>
      </section>
    {:else if step === 'preset'}
      <section class="step-body">
        <p class="lead">将为普通用户写入以下预设（之后可在「能力与安全」里逐项改）：</p>
        <ul class="intro-list">
          <li>记忆核心族六件：主动回忆 / 偏好学习 / 记忆注入 / 记忆固化 / 反思沉淀 / 器官链</li>
          <li>预算族保持后端默认（上下文 24000 字符 / 回合 8 轮 / 工具 16 次），随使用自动伸缩</li>
          <li>危险能力（shell / fetch / 文件写）保持关闭——不进任何预设</li>
        </ul>
        <label class="tune-row">
          <input type="checkbox" bind:checked={autoTune} />
          <span>
            <strong>使用中自动调整参数</strong>
            <small>从使用中学习，自动微调体验参数（selfTuning）；关闭 = 预设值固定不变</small>
          </span>
        </label>
      </section>
    {:else if step === 'pro-guide'}
      <section class="step-body">
        <p class="lead">专业用户通道——只做介绍，不写任何预设 / 配置 / 印象：</p>
        <ul class="intro-list">
          {#each PRO_GUIDE_LINES as line (line)}
            <li>{line}</li>
          {/each}
        </ul>
        <p class="hint">想换到普通用户轨道拿到预设值：设置 ›「能力与安全」有「推荐配置」一键预设。</p>
      </section>
    {/if}

    {#if error}<p class="wizard-error">{error}</p>{/if}

    <footer class="wizard-foot">
      {#if step === 'welcome'}
        <button class="skip-btn" onclick={onSkip} disabled={applying}>以后再说</button>
        <button class="start-btn" onclick={next} disabled={applying}>开始引导</button>
      {:else}
        <button class="nav-btn" onclick={back} disabled={applying}>
          <ChevronLeft size={14} /> 上一步
        </button>
        {#if step === 'tier' && !tier}
          <button class="start-btn" disabled>下一步 <ChevronRight size={14} /></button>
        {:else if stepIndex < steps.length - 1}
          <button class="start-btn" onclick={next}>下一步 <ChevronRight size={14} /></button>
        {:else if step === 'pro-guide'}
          <button class="start-btn" onclick={finishPro}>完成（不写任何预设）</button>
        {:else}
          <button class="start-btn" onclick={finishCasual} disabled={applying}>
            {applying ? '正在应用（网关重启中）…' : '完成'}
          </button>
        {/if}
      {/if}
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
  /* 分步进度点（依次提示的导航脊） */
  .step-dots {
    display: flex;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .step-dots li {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 999px;
    border: 1px solid var(--line, #2a323c);
    font-size: 10px;
    color: var(--faint, #8b97a5);
  }
  .step-dots li.active {
    border-color: var(--accent, #6ea8fe);
    color: var(--accent, #6ea8fe);
    background: var(--accent-wash, rgba(110, 168, 254, 0.12));
  }
  .step-dots li.done {
    border-color: var(--accent, #6ea8fe);
    color: var(--accent, #6ea8fe);
  }
  .step-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .lead {
    margin: 0;
    font-size: 13px;
    line-height: 1.7;
    color: var(--text, #e6edf3);
  }
  .intro-list {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .intro-list li {
    font-size: 12px;
    line-height: 1.6;
    color: var(--faint, #8b97a5);
  }
  .plan-box {
    border: 1px dashed var(--line, #2a323c);
    border-radius: 10px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .plan-box p {
    margin: 0;
    font-size: 12px;
    color: var(--faint, #8b97a5);
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--faint, #8b97a5);
    line-height: 1.6;
  }
  /* 分流双卡 */
  .tier-cards {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .tier-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px;
    border-radius: 10px;
    border: 1px solid var(--line, #2a323c);
    background: var(--surface-2, #161b21);
    color: var(--text, #e6edf3);
    cursor: pointer;
    text-align: left;
  }
  .tier-card.selected {
    border-color: var(--accent, #6ea8fe);
    background: var(--accent-wash, rgba(110, 168, 254, 0.12));
  }
  .tier-card h3 {
    margin: 0;
    font-size: 14px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .tier-tag {
    font-size: 10px;
    font-weight: 400;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--accent-wash, rgba(110, 168, 254, 0.15));
    color: var(--accent, #6ea8fe);
  }
  .tier-card p {
    margin: 0;
    font-size: 11px;
    line-height: 1.6;
    color: var(--faint, #8b97a5);
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
  .field input,
  .field textarea {
    padding: 9px 12px;
    border-radius: 8px;
    border: 1px solid var(--line, #2a323c);
    background: var(--surface-2, #161b21);
    color: var(--text, #e6edf3);
    font-size: 13px;
    font-family: inherit;
    resize: vertical;
  }
  /* 自动调参开关行 */
  .tune-row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 10px 12px;
    border: 1px solid var(--line, #2a323c);
    border-radius: 10px;
    background: var(--surface-2, #161b21);
    cursor: pointer;
  }
  .tune-row input {
    margin-top: 2px;
  }
  .tune-row span {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
    color: var(--text, #e6edf3);
  }
  .tune-row small {
    font-size: 11px;
    color: var(--faint, #8b97a5);
    line-height: 1.5;
  }
  .wizard-error {
    margin: 0;
    font-size: 12px;
    color: var(--danger, #e5484d);
  }
  .wizard-foot {
    display: flex;
    justify-content: space-between;
    gap: 10px;
  }
  .skip-btn,
  .nav-btn,
  .start-btn {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .skip-btn,
  .nav-btn {
    background: transparent;
    border: 1px solid var(--line, #2a323c);
    color: var(--faint, #8b97a5);
  }
  .start-btn {
    background: var(--accent, #6ea8fe);
    border: 1px solid transparent;
    color: #0b1014;
    font-weight: 600;
    margin-left: auto;
  }
  .skip-btn:disabled,
  .nav-btn:disabled,
  .start-btn:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
