<!--
  参考侧：生产组件的**直接消费者**。

  - 组件本身**只读引入**生产前端的文件（构建期由 vite 编译，不复制、不改）。
  - 数据 = 试点固定数据（与引擎侧**同一份** JSON）。
  - 对外暴露 `window.__PILOT__`（与引擎侧同名同义）⇒ 对照 harness 用**同一套**
    触发序列驱动两侧。
-->
<script>
  import ApprovalRequestCard from '../../../../frontend/companion-desktop/src/lib/components/ApprovalRequestCard.svelte';
  import fixture from '../fixtures/pilot-request.json';

  let busy = $state(false);
  let visible = $state(true);
  let decision = $state(null);

  function onAllow() {
    decision = true;
    busy = false;
  }

  function onReject() {
    decision = false;
    busy = false;
  }

  /** 关闭：**只关卡片**，不做业务决策（与引擎侧语义一致）。 */
  function onDismiss() {
    visible = false;
  }

  if (typeof window !== 'undefined') {
    window.__PILOT__ = {
      ready: () => true,
      setBusy: (value) => {
        busy = value;
      },
      setCopied: () => {
        /* 生产组件的复制态是组件内部状态 ⇒ 参考侧不提供外部写入口。 */
      },
      allow: () => onAllow(),
      reject: () => onReject(),
      dismiss: () => onDismiss(),
      decision: () => decision,
      cardPresent: () => visible,
    };
  }
</script>

{#if visible}
  <ApprovalRequestCard
    item={fixture}
    {busy}
    onAllow={onAllow}
    onReject={onReject}
    onDismiss={onDismiss}
  />
{/if}
