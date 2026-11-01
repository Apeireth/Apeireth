<!--
  参考侧：工具卡面两面的**生产组件直接消费者**。

  - 组件本身**只读引入**生产前端的文件（构建期由 vite 编译，不复制、不改）。
  - 数据 = 固定数据（与引擎侧**同一份** JSON）。
  - 对外暴露 `window.__TOOLCARD__`（与引擎侧同名同义，API 实现在共享文件里）。
-->
<script>
  import ToolCallCard from '../../../../../frontend/companion-desktop/src/lib/components/ToolCallCard.svelte';
  import ToolCallLifecycleCard from '../../../../../frontend/companion-desktop/src/lib/components/ToolCallLifecycleCard.svelte';
  import fixture from '../../fixtures/toolcard-cases.json';
  import { createToolCardApi } from '../../shared/toolcard-api.js';

  let index = $state(0);
  const current = $derived(fixture.cases[index]);

  if (typeof window !== 'undefined') {
    // 固定数据挂到 window ⇒ 浏览器差分测试能用**同一批值**跑脚本侧序列化。
    window.__TOOLCARD_FIXTURE__ = fixture;
    window.__TOOLCARD__ = createToolCardApi({
      caseCount: () => fixture.cases.length,
      selectCase: (value) => {
        index = value;
      },
    });
  }
</script>

<div class="toolcard-stage">
  <ToolCallCard toolCall={current.toolCall} />
  <ToolCallLifecycleCard call={current.lifecycleCall} />
</div>
