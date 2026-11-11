// 参考侧的挂载：生产组件（**只读引入**）+ 同一份固定数据。
//
// ## 两种形态
//
// | 页面 | 挂什么 |
// |---|---|
// | `/reference/<面>/index.html` | 生产组件（本面对应的那个） |
// | `/reference/index.html` | **总览页**（列出全部面；不渲染画面） |
//
// ## 纪律
//
// - 生产组件**不复制、不改**（构建期由 vite 编译）；
// - 生产的数据面模块由 vite 插件拦到**对照用替身**（喂固定 fixture）
//   ⇒ 生产文件**零改动**。

import { mount } from 'svelte';
import { PANELS } from '../shared/panels.js';

// 总览页：只列面（不挂载生产组件）⇒ 避免「总览页也要喂数据」的额外依赖。
const isOverview = location.pathname.endsWith("/reference/") || location.pathname.endsWith("/reference");

if (!isOverview) {
  const { default: ReferenceSettingsApp } = await import('./ReferenceSettingsApp.svelte');
  mount(ReferenceSettingsApp, { target: document.getElementById('app') });
} else {
  const list = document.getElementById('app');
  if (list) {
    list.innerHTML = PANELS.map(
      (panel) =>
        `<li><a href="./${panel.id}/index.html">${panel.title}</a>` +
        `<small>${panel.id} · ${panel.scope} · 可信度 ${panel.trust.level}</small></li>`,
    ).join('');
  }
}
