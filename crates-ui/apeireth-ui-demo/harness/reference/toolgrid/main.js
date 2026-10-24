// 参考侧的挂载：生产视图（**只读引入**）+ 同一份固定数据。
import { mount } from 'svelte';
import ToolGridApp from './ToolGridApp.svelte';

export default mount(ToolGridApp, { target: document.getElementById('app') });
