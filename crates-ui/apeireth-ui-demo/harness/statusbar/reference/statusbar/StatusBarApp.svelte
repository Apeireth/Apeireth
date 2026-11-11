<!--
  参考侧：状态条面的**生产组件直接消费者**。

  - 组件本身**只读引入**生产前端的文件（构建期由 vite 编译，不复制、不改）。
  - 数据 = 固定数据（与引擎侧**同一份** JSON）。
  - 对外暴露 `window.__STATUSBAR__`（与引擎侧同名同义，API 实现在共享文件里）。
-->
<script>
  import StatusBar from '../../../../../../frontend/companion-desktop/src/lib/components/StatusBar.svelte';
  import StatusBadge from '../../../../../../frontend/companion-desktop/src/lib/components/StatusBadge.svelte';
  import fixture from '../../fixtures/statusbar-cases.json';
  import { createStatusBarApi } from '../../shared/statusbar-api.js';

  let index = $state(0);
  let clicks = $state(0);
  const current = $derived(fixture.cases[index]);

  // 四格各一个回调（**恒在**；不可点的格**不挂**监听器 ⇒ 与生产组件同构）。
  const bump = () => {
    clicks += 1;
  };

  if (typeof window !== 'undefined') {
    window.__STATUSBAR_FIXTURE__ = fixture;
    window.__STATUSBAR__ = createStatusBarApi({
      caseCount: () => fixture.cases.length,
      selectCase: (value) => {
        index = value;
      },
      clickCount: () => clicks,
    });
  }
</script>

<div class="statusbar-stage">
  <StatusBar
    sse={current.items[0]}
    turn={current.items[1]}
    guard={current.items[2]}
    memory={current.items[3]}
    onSseClick={bump}
    onTurnClick={bump}
    onGuardClick={bump}
    onMemoryClick={bump}
  />
  <StatusBadge
    variant={current.badge.variant}
    label={current.badge.label}
    size={current.badge.size}
  />
</div>
