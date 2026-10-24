// 参考侧的挂载：生产组件（**只读引入**）+ 同一份固定数据。
import { mount } from 'svelte';
import AuditApp from './AuditApp.svelte';

export default mount(AuditApp, { target: document.getElementById('app') });
