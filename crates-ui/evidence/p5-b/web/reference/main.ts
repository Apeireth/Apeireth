/**
 * P5-b 参考页 —— **生产版场景**（只读引入，不改生产文件一行）。
 *
 * - 引入方式：按**路径** import 生产版的场景类（`frontend/companion-desktop/src/lib/scene/blackhole.ts`），
 *   生产文件全程**只读**（vite 只做 TS → JS 的转译与打包）；
 * - 构造选项：`interactive: false`（关掉鼠标轨道视差）、`reduceMotion: false`（走 rAF 动画路径）；
 * - 时钟冻结：URL 带 `?t0=<秒>` ⇒ 生产版把 `tSec` 钉在 `t0`、`rot` 钉在 0；
 * - uniform 取值由 **CDP 注入的录制器**在 `drawArrays` 时刻抓取（生产版实际用的那一帧），
 *   本页只负责把场景实例暴露给驱动（`window.__P5B_SCENE__`）以便冻结渲染循环。
 */

import { BlackholeScene } from '../../../../../frontend/companion-desktop/src/lib/scene/blackhole.ts';

const canvas = document.getElementById('scene') as HTMLCanvasElement;
const scene = new BlackholeScene(canvas, { interactive: false, reduceMotion: false });

declare global {
  interface Window {
    __P5B_SCENE__: BlackholeScene;
    __P5B_ENV__: Record<string, unknown>;
  }
}

const rect = canvas.getBoundingClientRect();
window.__P5B_SCENE__ = scene;
window.__P5B_ENV__ = {
  cssWidth: rect.width,
  cssHeight: rect.height,
  canvasWidth: canvas.width,
  canvasHeight: canvas.height,
  devicePixelRatio: window.devicePixelRatio,
  visibilityState: document.visibilityState,
  fallback: scene.fallback,
  cameraIndex: scene.getCameraIndex(),
};
