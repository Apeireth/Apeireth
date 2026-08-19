<script lang="ts">
  import {Sparkles, X} from 'lucide-svelte';
  import type {CompanionPresentationState} from '../../lib/runtime';

  let {
    state = 'idle',
    proactiveText = '',
    onDismissProactive,
    onActivate,
  }: {
    state: CompanionPresentationState;
    proactiveText?: string;
    onDismissProactive?: () => void;
    onActivate?: () => void;
  } = $props();

  const stateTitle: Record<CompanionPresentationState, string> = {
    idle: '陪伴中 (安静守候)',
    thinking: '正在思考与推理…',
    speaking: '正在表达与生成…',
    working: '正在执行工具与任务…',
    reflecting: '正在深度反思与提炼…',
    concerned: '需要主人关注或审批',
    happy: '心情愉快',
  };
</script>

<div class="companion-widget" data-state={state}>
  <button
    type="button"
    class="companion-avatar-btn"
    onclick={onActivate}
    title={`阿佩瑞斯: ${stateTitle[state] || state}`}
    aria-label="伴随体状态"
  >
    <div
      class="companion-eye"
      class:thinking={state === 'thinking'}
      class:speaking={state === 'speaking'}
      class:working={state === 'working'}
      class:reflecting={state === 'reflecting'}
      class:concerned={state === 'concerned'}
    >
      <div class="eye-pupil"></div>
    </div>
  </button>

  {#if proactiveText}
    <div class="companion-speech-bubble" role="status">
      <div class="bubble-tail"></div>
      <div class="bubble-body">
        <Sparkles size={13} class="bubble-icon" />
        <p>{proactiveText}</p>
        {#if onDismissProactive}
          <button
            type="button"
            class="bubble-close"
            onclick={onDismissProactive}
            aria-label="关闭问候"
          >
            <X size={11} />
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>
