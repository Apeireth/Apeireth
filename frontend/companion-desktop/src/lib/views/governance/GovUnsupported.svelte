<script lang="ts">
  // 治理卷宗 — 能力不支持显式标注（0 装纪律：能力不可用就说不可用，
  // 不装死也不造假；capability id 等宽小字，用户问「这是真的吗」永远答得起）。
  import {ShieldOff} from 'lucide-svelte';

  let {
    capabilityId,
    reason = null,
    hint = '',
  }: {
    capabilityId: string;
    /** capabilityUnavailableReason 的 machine-readable 原因（not_implemented 等）。 */
    reason?: string | null;
    /** 一句人话补充（可选）。 */
    hint?: string;
  } = $props();
</script>

<div class="gov-unsupported" role="note">
  <span class="u-icon"><ShieldOff size={14} /></span>
  <div class="u-body">
    <p class="u-title">当前运行时不支持此卷宗面</p>
    <p class="u-detail">
      能力 <code>{capabilityId}</code> 未在 runtime capability manifest 中声明为可用{#if reason}（原因 <code>{reason}</code>）{/if}。
      {#if hint}{hint}{/if}
    </p>
  </div>
</div>

<style>
  .gov-unsupported {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 14px;
    border: 1px dashed var(--ap-bone-30);
    border-radius: 8px;
    background: rgba(5, 10, 15, 0.35);
    color: var(--ap-bone-68);
  }
  .u-icon {
    flex: none;
    margin-top: 1px;
    color: var(--ap-bone-42);
  }
  .u-body {
    min-width: 0;
  }
  .u-title {
    margin: 0 0 4px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ap-bone);
  }
  .u-detail {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.7;
    color: var(--ap-bone-42);
  }
  .u-detail code {
    font-family: var(--ap-font-mono);
    font-size: 10.5px;
    color: var(--ap-bone-68);
    background: rgba(0, 0, 0, 0.3);
    padding: 1px 5px;
    border-radius: 3px;
  }
</style>
