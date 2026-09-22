<script lang="ts">
  // T0 聊天壳骨骼 — 守卫通报卡（琥珀），00-PHILOSOPHY §3.2「动作收敛为卡片」。
  //
  // 对话内呈现，不跳窗：回合被治理面拦停时，在发生处（消息流末尾）浮出。
  // 琥珀 = 语义警示色（01-DESIGN-SYSTEM §2.4 warning，🟡 提案值），不是金——
  // 金色留给他；守卫是治理面，不是他的存在。
  //
  // 真实信号断点（0 装）：本卡唯一接线是回合错误帧 code === review_rejected
  // （认知评审驳回，error_codes.rs:21 核实存在）。canonical 错误帧目录没有
  // guard 专属 code，events 总线（契约 §8）也没有 guard_* 事件类型——
  // 守卫否决当前只经审批流（待签文书卡）或 tool_failed（工具卡红边）上浮。
  // 未来 guard 获得专属信号后在 gateway-events.ts classifyGovernanceNotice
  // 扩展分支即可，本卡形状不变。没有真实信号时本卡蛰伏，绝不演示假数据。
  import {ShieldAlert, X} from 'lucide-svelte';

  let {
    title,
    message,
    solution = '',
    onClose,
  }: {
    /** 通报标题（来自 classifyGovernanceNotice 的人话）。 */
    title: string;
    /** 后端上浮的原始原因。 */
    message: string;
    /** 可操作方案（ERROR_SOLUTIONS 真实文案）。 */
    solution?: string;
    onClose: () => void;
  } = $props();
</script>

<div class="guard-card" role="alert">
  <span class="guard-stamp" aria-hidden="true">守卫通报</span>
  <div class="guard-head">
    <ShieldAlert size={14} />
    <h3 class="guard-title">{title}</h3>
    <button class="guard-close" onclick={onClose} aria-label="关闭通报">
      <X size={13} />
    </button>
  </div>
  {#if message}
    <p class="guard-message">{message}</p>
  {/if}
  {#if solution}
    <p class="guard-solution">{solution}</p>
  {/if}
</div>

<style>
  .guard-card {
    position: relative;
    margin-top: 10px;
    padding: 14px 16px 12px;
    border: 1px solid var(--amber-line);
    border-radius: 10px;
    background: var(--amber-wash);
    backdrop-filter: blur(10px);
  }
  .guard-stamp {
    position: absolute;
    top: -7px;
    left: 14px;
    padding: 0 6px;
    background: var(--ap-space-void);
    font-family: var(--ap-font-mono);
    font-size: 9px;
    letter-spacing: 0.3em;
    color: var(--amber);
    opacity: 0.85;
  }
  .guard-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .guard-head > :global(svg) {
    flex: none;
    color: var(--amber);
  }
  .guard-title {
    margin: 0;
    font-size: 12.5px;
    font-weight: 500;
    letter-spacing: 0.06em;
    color: var(--ap-bone);
  }
  .guard-close {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--ap-bone-42);
    cursor: pointer;
    padding: 2px;
    border-radius: 6px;
  }
  .guard-close:hover {
    color: var(--ap-bone);
    background: rgba(255, 255, 255, 0.06);
  }
  .guard-message {
    margin: 8px 0 0;
    font-size: 12px;
    line-height: 1.7;
    color: var(--ap-bone-68);
    word-break: break-word;
  }
  .guard-solution {
    margin: 6px 0 0;
    font-size: 11.5px;
    line-height: 1.7;
    color: var(--amber-hi);
  }
</style>
