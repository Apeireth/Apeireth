<!--
  参考侧：治理卷宗面「守卫」的**生产组件直接消费者**。

  - 组件本身**只读引入**生产前端的文件（构建期由 vite 编译，不复制、不改）。
  - `runtime` 被 vite 插件拦到对照用替身（喂固定 fixture）⇒ 生产文件零改动。
  - 能力清单 / 数据 / 用例都来自**同一份 fixture**（与引擎侧同源）。
  - 对外暴露 `window.__GOVERNANCE__`（与引擎侧同名同义，API 实现在共享文件里）。
-->
<script>
  import GuardTab from '../../../../../../frontend/companion-desktop/src/lib/views/governance/GuardTab.svelte';
  import fixture from '../../fixtures/governance-cases.json';
  import { createGovernanceApi } from '../../shared/governance-api.js';

  const tab = 'guard';
  const cases = fixture.cases[tab];
  // 对照 harness 在**导航前**注入当前用例下标与基准时刻（两侧同一份）。
  const index =
    (typeof window !== 'undefined' && Number(window.__GOV_CASE_INDEX__)) || 0;
  const item = cases[index] || cases[0];
  if (typeof window !== 'undefined') {
    window.__GOV_CASE__ = item;
  }
  const capabilities =
    item.manifest === null ? null : fixture.manifests[item.manifest] ?? null;
  const config = { baseUrl: 'http://127.0.0.1:0' };

  if (typeof window !== 'undefined') {
    window.__GOVERNANCE__ = createGovernanceApi({ cases });
    // 「数据面已就位」标志：生产组件在挂载后的微任务里发起取数（mock 立即
    // resolve）⇒ 120ms 后画面已落到数据态 ⇒ 置位供对照 harness 判定。
    // ⚠️ 能力清单未到的用例**不会发起取数** ⇒ 也要置位（否则对照会等到超时）。
    setTimeout(() => {
      window.__GOV_DATA_READY__ = true;
    }, 120);
  }
</script>

<div class="governance-stage">
  <GuardTab
    {config}
    {capabilities}
  />
</div>
