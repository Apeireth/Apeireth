// 参考侧的挂载：生产组件（**只读引入**）+ 同一份固定数据。
import { mount } from 'svelte';
import GrantsApp from './GrantsApp.svelte';

export default mount(GrantsApp, { target: document.getElementById('app') });
