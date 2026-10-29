<!--
  参考侧：工具卡面的**生产视图直接消费者**。

  - 视图本身**只读引入**生产前端的文件（构建期由 vite 编译，不复制、不改）。
  - `../runtime` 由 vite 插件拦到**对照用替身**（喂固定 fixture）⇒ 生产文件
    零改动。
  - 数据 = 固定数据（与引擎侧**同一份** JSON）。
  - 对外暴露 `window.__TOOLGRID__`（与引擎侧同名同义，API 实现在共享文件里）。
-->
<script>
  import ToolsView from '../../../../../frontend/companion-desktop/src/lib/views/ToolsView.svelte';
  import fixture from '../../fixtures/toolgrid-cases.json';
  import { createToolGridApi } from '../../shared/toolgrid-api.js';

  // 生产组件要一个配置对象（真实实现里用来拼请求地址 ⇒ 本批被替身拦掉 ⇒
  // 这里只需要一个形状正确的占位）。
  const config = {
    baseUrl: 'http://127.0.0.1:0',
    apiKey: '',
    model: 'fixture',
  };

  if (typeof window !== 'undefined') {
    // 固定数据挂到 window ⇒ 浏览器差分测试能用**同一批值**。
    window.__TOOLGRID_FIXTURE__ = fixture;
    window.__TOOLGRID__ = createToolGridApi({
      caseCount: () => fixture.cases.length,
      // 参考侧的搜索词由**生产组件自己的绑定**驱动（`bind:value` ⇒ `input`
      // 事件）⇒ 这里不需要再写一次状态。
      applyQuery: () => {},
    });
  }
</script>

<div class="toolgrid-stage">
  <ToolsView {config} capabilities={fixture.capabilities} />
</div>
