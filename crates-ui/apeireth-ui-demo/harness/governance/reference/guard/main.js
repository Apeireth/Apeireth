// 参考侧的挂载：生产组件（**只读引入**）+ 同一份固定数据。
import { mount } from 'svelte';
import GuardApp from './GuardApp.svelte';

export default mount(GuardApp, { target: document.getElementById('app') });
