<script lang="ts">
  // T0 聊天壳骨骼 — 待签文书 dock（审批卡对话内化，00-PHILOSOPHY §3.2/§6.4）
  //
  // 审批 = 对话内浮起的「待签文书」：金色不是因为审批重要，是因为
  // 「他停下了」（§7 金色纪律）。在发生处完成批准/拒绝
  // （POST /v1/approvals/resolve，解析后原回合继续），不跳窗。
  //
  // 收起语义：X / Esc = 收起为一条 slim 金线（不做业务决策，区别于「拒绝」）；
  // 同一文书保持收起，新文书（approval_id 变化）自动重新浮起。
  import {ChevronUp} from 'lucide-svelte';
  import ApprovalRequestCard from '../components/ApprovalRequestCard.svelte';

  let {
    approvalId,
    item,
    busy,
    collapsed = false,
    onExpand,
    onAllow,
    onReject,
    onDismiss,
  }: {
    /** 当前文书 id（调用方据此在新文书到达时重置收起态）。 */
    approvalId: string;
    item: {
      title?: string;
      commandText?: string;
      argumentsSummary?: string;
      reason?: string;
      createdAt?: number;
      /** W1 沙箱徽标（后端 display_invocation.sandbox；批准前看见墙）。 */
      sandbox?: string;
      /** 冻结工作目录（卷宗 cwd）。 */
      cwd?: string;
      /** 隔离态摘要（文件/网络）。 */
      isolation?: string;
    };
    busy: boolean;
    /** 收起态由调用方持有（Esc/会话切换统一走调用方语义）。 */
    collapsed?: boolean;
    onExpand: () => void;
    onAllow: () => void;
    onReject: () => void;
    /** 收起（不做业务决策）。 */
    onDismiss: () => void;
  } = $props();
</script>

{#if collapsed}
  <!-- 收起态：一条 slim 金线，文书仍在等 —— 点击重新展开 -->
  <button class="doc-slim" onclick={onExpand} aria-label="他停下了，一份文书等你签字——点击展开">
    <span class="doc-ember" aria-hidden="true"></span>
    <span class="doc-slim-text">他停下了 · 一份文书等你签字</span>
    <span class="doc-slim-cta">
      展开
      <ChevronUp size={12} />
    </span>
  </button>
{:else}
  <div class="doc-dock" role="region" aria-label="待签文书" data-approval-id={approvalId}>
    <span class="doc-stamp" aria-hidden="true">待签文书 · 他停下了</span>
    <ApprovalRequestCard {item} {busy} {onAllow} {onReject} {onDismiss} />
  </div>
{/if}

<style>
  /* 浮起的文书：金边 + 上浮阴影（§6.4 判断即仪式：金色、浮起、被等待） */
  .doc-dock {
    position: relative;
    margin-top: 4px;
    padding: 22px 6px 4px;
    border: 1px solid rgba(255, 210, 122, 0.38);
    border-radius: 12px;
    background: rgba(11, 13, 18, 0.6);
    box-shadow:
      0 10px 44px -12px rgba(255, 210, 122, 0.22),
      0 2px 14px rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(14px);
  }
  /* 文书戳：mono 小字（§6.3 编号/状态戳语言），绝对定位不占排版 */
  .doc-stamp {
    position: absolute;
    top: 6px;
    left: 16px;
    font-family: var(--ap-font-mono);
    font-size: 9px;
    letter-spacing: 0.3em;
    color: var(--ap-gold);
    opacity: 0.75;
    pointer-events: none;
  }
  .doc-dock :global(.approval-card) {
    max-width: none;
    background: transparent;
    border: 0;
    box-shadow: none;
  }

  /* 收起态 slim 金线 */
  .doc-slim {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    margin-top: 4px;
    padding: 9px 14px;
    border: 1px solid rgba(255, 210, 122, 0.3);
    border-radius: 999px;
    background: rgba(11, 13, 18, 0.6);
    backdrop-filter: blur(10px);
    cursor: pointer;
    text-align: left;
    transition: border-color 0.25s ease;
  }
  .doc-slim:hover {
    border-color: rgba(255, 210, 122, 0.55);
  }
  /* 被等待的余烬：2.8s 呼吸（motion.breathe ✅，与「正在输入」同一节律） */
  .doc-ember {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ap-gold);
    box-shadow: 0 0 9px rgba(255, 210, 122, 0.6);
    animation: ap-doc-breathe 2.8s ease-in-out infinite;
  }
  @keyframes ap-doc-breathe {
    0%,
    100% {
      opacity: 0.22;
      transform: scale(1);
    }
    50% {
      opacity: 0.65;
      transform: scale(1.09);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .doc-ember {
      animation: none;
      opacity: 0.45;
    }
  }
  .doc-slim-text {
    font-size: 11.5px;
    letter-spacing: 0.1em;
    color: var(--ap-gold-ui);
  }
  .doc-slim-cta {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.2em;
    color: var(--ap-bone-42);
  }
</style>
