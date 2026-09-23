<script lang="ts">
  // 治理卷宗共享门：capabilities === null（运行时能力清单未到）时的诚实等待态。
  //
  // 背景（0 装 + 走查修复 2026-09-23）：capabilities 仅在 /health 通时由 App
  // 拉取（App.svelte refreshConnection）——网关离线则永为 null，原实现四 tab
  // 各自无限转「正在读取运行时能力清单…」，无超时、无说明。
  // 本组件：12s 内转圈（正常启动节拍），超时后给出诚实说明——不是「不支持」
  // （GovUnsupported 语义），是「还没到」。健康轮询每 15s 自动重试，
  // 连接恢复后 capabilities 到位即自动换页，无需手动重试按钮。
  import {onMount, onDestroy} from 'svelte';
  import {Loader2, CloudOff} from 'lucide-svelte';
  import LoadingState from '../../components/LoadingState.svelte';

  let {message = '正在读取运行时能力清单…'}: {message?: string} = $props();

  const WAIT_BUDGET_MS = 12000;
  let waitedTooLong = $state(false);
  let timer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    timer = setTimeout(() => (waitedTooLong = true), WAIT_BUDGET_MS);
  });
  onDestroy(() => {
    if (timer) clearTimeout(timer);
  });
</script>

{#if !waitedTooLong}
  <LoadingState {message} />
{:else}
  <div class="manifest-pending" role="status">
    <div class="mp-icon"><CloudOff size={22} /></div>
    <h3 class="mp-title">运行时能力清单迟迟未到</h3>
    <p class="mp-desc">
      网关可能离线或仍在启动——这本账需要 gateway 的内省面才能翻开。
      连接恢复后会自动载入，无需手动重试。
    </p>
  </div>
{/if}

<style>
  .manifest-pending {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    text-align: center;
  }
  .mp-icon {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    margin-bottom: 14px;
    color: var(--ap-semantic-warning);
    background: color-mix(in srgb, var(--ap-semantic-warning) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--ap-semantic-warning) 30%, transparent);
  }
  .mp-title {
    margin: 0 0 6px;
    font-size: 15px;
    font-weight: 600;
    color: var(--ap-bone);
  }
  .mp-desc {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.7;
    color: var(--ap-bone-42);
    max-width: 420px;
  }
</style>
