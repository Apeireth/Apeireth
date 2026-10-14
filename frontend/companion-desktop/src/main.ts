import {mount} from 'svelte';
import App from './App.svelte';
import QuickWindowView from './features/quick/QuickWindowView.svelte';
import PetWindowView from './features/pet/PetWindowView.svelte';
import './app.css';

const params = new URLSearchParams(window.location.search);
const windowRole = params.get('window');

// 窗口角色路由：主窗（默认）/ 快捷窗 / 桌宠常驻窗（presence 第四个投影）。
const View =
  windowRole === 'quick' ? QuickWindowView : windowRole === 'pet' ? PetWindowView : App;

mount(View, {
  target: document.getElementById('app')!,
});
