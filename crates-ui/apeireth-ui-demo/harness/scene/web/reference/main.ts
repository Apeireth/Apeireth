/**
 * 参考页 —— **生产版场景**（只读引入，不改生产文件一行）。
 *
 * - 按**路径** import 生产版的场景类，生产文件全程**只读**（vite 只做
 *   TS → JS 的转译与打包）；
 * - 用例参数全部走 URL 查询串（驱动脚本按用例切换）：
 *   - `path`：`static`（静态单帧）/ `animated`（动画路径）；
 *   - `mode`：`quiet` / `thinking` / `speaking`；
 *   - `hour`：舰内时刻（缺省 ⇒ 本地时钟）；
 *   - `camera`：机位下标；
 *   - `pad`：`p,a,d` 三轴（缺省 ⇒ 没有存在态帧）；
 *   - `intensity`：回合级强度（缺省 ⇒ 没有存在态帧）；
 * - 动画路径的时钟由生产版自带的 `?t0=<秒>` 钩子冻结（生产文件自己读 URL）；
 * - uniform 取值由 **CDP 注入的录制器**在 `drawArrays` 时刻抓取。
 */

import { BlackholeScene } from '../../../../../../frontend/companion-desktop/src/lib/scene/blackhole.ts';

const params = new URLSearchParams(location.search);
const canvas = document.getElementById('scene') as HTMLCanvasElement;

const path = params.get('path') === 'animated' ? 'animated' : 'static';
const mode = (params.get('mode') ?? 'quiet') as 'quiet' | 'thinking' | 'speaking';
const hourParam = params.get('hour');
const hour = hourParam === null || hourParam === '' ? null : Number(hourParam);
const camera = Number(params.get('camera') ?? '0');
const padParam = params.get('pad');
const intensityParam = params.get('intensity');

const scene = new BlackholeScene(canvas, {
  interactive: false,
  reduceMotion: path === 'static',
});

// ⚠️ 顺序与生产侧的调用序列一致：先档位、再时刻、再机位、最后存在态。
// 静态路径下每次调用都会立刻重绘 ⇒ 最后一帧就是本用例的最终状态。
scene.setMode(mode);
scene.setHour(hour);
scene.setCamera(camera);
if (padParam !== null && padParam !== '') {
  const [p, a, d] = padParam.split(',').map((value) => Number(value));
  const intensity = intensityParam === null || intensityParam === '' ? null : Number(intensityParam);
  scene.setPadBias({ p, a, d }, intensity);
}

declare global {
  interface Window {
    __SCENE_SCENE__: BlackholeScene;
    __SCENE_ENV__: Record<string, unknown>;
  }
}

const rect = canvas.getBoundingClientRect();
window.__SCENE_SCENE__ = scene;
window.__SCENE_ENV__ = {
  cssWidth: rect.width,
  cssHeight: rect.height,
  canvasWidth: canvas.width,
  canvasHeight: canvas.height,
  devicePixelRatio: window.devicePixelRatio,
  visibilityState: document.visibilityState,
  fallback: scene.fallback,
  cameraIndex: scene.getCameraIndex(),
  path,
  mode,
  hour,
  camera,
  pad: padParam,
  intensity: intensityParam,
};
