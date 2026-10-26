/**
 * P5-b 片头腿 —— 生产 `intro/fire.ts` ↔ Rust 引擎（**逐值** + **逐像素**）。
 *
 * ## 这一页在测什么
 *
 * | 判据 | 含义 |
 * |---|---|
 * | **逐值** | 生产侧**实际用的那一帧**的 uniform 取值 vs Rust 场景层算出来的取值 ⇒ **逐个分量**比 |
 * | **逐像素** | 生产侧画布 vs 引擎侧画布（同一批取值）⇒ 逐像素差 |
 *
 * ## ⭐ 取值怎么抓（**不**改生产文件一行）
 *
 * 生产 `fire.ts` 用**原生 WebGL2** ⇒ ⇒ 在 `WebGL2RenderingContext.prototype`
 * 上装录制器（与 P5-b 的生产对照**同一口径**）：
 *
 * - `getUniformLocation` ⇒ 记「location → 名字」；
 * - `uniform1f/2f/3f/4f` ⇒ 记「location → 值」；
 * - `useProgram` ⇒ 记「当前 program」（生产侧在 `useProgram` **之后**才取
 *   各 uniform 的 location ⇒ 归属正确）；
 * - `drawArrays` ⇒ **在绘制时刻**快照 ⇒ 抓到的就是这一帧真正用的值。
 *
 * ## ⭐ 两张画布怎么区分（背景 pass vs 粒子 pass）
 *
 * 生产侧：`drawArrays(gl.TRIANGLES, 0, 3)` = 背景 matte；
 * `drawArrays(gl.POINTS, 0, 4096)` = 火焰粒子 ⇒ ⇒ **按 draw mode 分类**。
 *
 * ## 时钟冻结
 *
 * 生产 `fire.ts` 自带 `?it=<秒>` 调试钩子（把 `tSec` 钉在 `it`）⇒
 * ⇒ 本页用 `?it=` 冻结时钟 ⇒ ⇒ 抓到的取值与画的画面属于**同一帧**。
 */

import { IntroFire } from '../../../../../frontend/companion-desktop/src/lib/intro/fire.ts';

// ---------------------------------------------------------------------------
// 录制器（必须在**生产实例创建之前**装上）
// ---------------------------------------------------------------------------
type UniformSnapshot = Record<string, number[]>;

const captured: {
  background: UniformSnapshot | null;
  sprite: UniformSnapshot | null;
  draws: number;
} = { background: null, sprite: null, draws: 0 };

(function installRecorder() {
  const proto = WebGL2RenderingContext.prototype as unknown as Record<string, unknown>;
  const locationNames = new WeakMap<object, string>();
  const programOfLocation = new WeakMap<object, WebGLProgram>();
  // ⚠ **按 program 分桶**存取值：生产侧有两个 program（背景 matte / 火焰粒子）
  //   ⇒ 若用**一张全局表**，后写的会覆盖先写的 ⇒ ⇒ 快照会混着两个 pass 的
  //   uniform ⇒ ⇒ 必须按 program 归属。
  const valuesByProgram = new Map<WebGLProgram, Map<object, number[]>>();
  // ⚠ 当前 program（生产侧在 `useProgram` **之后**才取各 uniform 的 location
  //   ⇒ 归属正确 ⇒ 快照取的是「这一遍真正用的那个 program」的值）。
  let currentProgram: WebGLProgram | null = null;

  const originalUseProgram = proto['useProgram'] as (program: WebGLProgram | null) => void;
  proto['useProgram'] = function (this: WebGL2RenderingContext, program: WebGLProgram | null) {
    currentProgram = program;
    return originalUseProgram.call(this, program);
  };

  const originalGetUniformLocation = proto['getUniformLocation'] as (
    program: WebGLProgram,
    name: string,
  ) => WebGLUniformLocation | null;
  proto['getUniformLocation'] = function (
    this: WebGL2RenderingContext,
    program: WebGLProgram,
    name: string,
  ) {
    const location = originalGetUniformLocation.call(this, program, name);
    if (location) {
      locationNames.set(location as unknown as object, name);
      programOfLocation.set(location as unknown as object, program);
    }
    return location;
  };

  for (const fn of ['uniform1f', 'uniform2f', 'uniform3f', 'uniform4f'] as const) {
    const original = proto[fn] as (...args: unknown[]) => void;
    proto[fn] = function (this: WebGL2RenderingContext, ...args: unknown[]) {
      const location = args[0] as WebGLUniformLocation | null;
      if (location) {
        const program = programOfLocation.get(location as unknown as object);
        if (program) {
          let bucket = valuesByProgram.get(program);
          if (!bucket) {
            bucket = new Map<object, number[]>();
            valuesByProgram.set(program, bucket);
          }
          bucket.set(location as unknown as object, args.slice(1) as number[]);
        }
      }
      return original.apply(this, args);
    };
  }

  const originalDrawArrays = proto['drawArrays'] as (
    mode: number,
    first: number,
    count: number,
  ) => void;
  proto['drawArrays'] = function (
    this: WebGL2RenderingContext,
    mode: number,
    first: number,
    count: number,
  ) {
    // ⚠ 取「**当前 program**」的那一桶（`useProgram` 记下的）。
    const program = currentProgram;
    const bucket = program ? valuesByProgram.get(program) : undefined;
    const snapshot: UniformSnapshot = {};
    if (bucket) {
      for (const [location, value] of bucket) {
        const name = locationNames.get(location);
        if (name !== undefined) snapshot[name] = value;
      }
    }
    captured.draws += 1;
    // gl.TRIANGLES = 4 ⇒ 背景 matte；gl.POINTS = 0 ⇒ 火焰粒子。
    if (mode === 0) captured.sprite = snapshot;
    else captured.background = snapshot;
    return originalDrawArrays.call(this, mode, first, count);
  };
})();

// ---------------------------------------------------------------------------
// 生产侧（只读引入，一行不改）
// ---------------------------------------------------------------------------
const params = new URLSearchParams(location.search);
const frozenAt = Number.parseFloat(params.get('it') ?? '13.5');
const productionCanvas = document.getElementById('production') as HTMLCanvasElement;
const engineCanvases: Record<string, HTMLCanvasElement> = {
  webgl: document.getElementById('engine-webgl') as HTMLCanvasElement,
  webgpu: document.getElementById('engine-webgpu') as HTMLCanvasElement,
};

const fire = new IntroFire(productionCanvas);

// ---------------------------------------------------------------------------
// Rust 侧（wasm 宿主）
// ---------------------------------------------------------------------------

/** 逐值比较：两个分量列表是否**逐位**相同（f32 收口后比较）。 */
function compareComponents(left: number[] | undefined, right: number[] | undefined) {
  if (!left || !right) return { same: false, reason: 'missing' };
  if (left.length !== right.length) {
    return { same: false, reason: `分量数不同：${left.length} vs ${right.length}` };
  }
  const leftF32 = left.map((v) => Math.fround(v));
  const rightF32 = right.map((v) => Math.fround(v));
  for (let i = 0; i < leftF32.length; i += 1) {
    if (leftF32[i] !== rightF32[i]) {
      return {
        same: false,
        reason: `分量 ${i} 不同：生产 ${leftF32[i]} vs Rust ${rightF32[i]}（原始 ${left[i]} / ${right[i]}）`,
      };
    }
  }
  return { same: true, reason: '' };
}

async function main() {
  const result: Record<string, unknown> = { frozen_at: frozenAt };
  try {
    // ① 等生产侧画出第一帧（录制器会抓到取值）。
    const deadline = Date.now() + 30000;
    while (captured.draws < 2 && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    result.production_draws = captured.draws;
    if (!captured.background) {
      result.error = '生产侧没有抓到背景 uniform';
      window.__INTRO__ = result;
      return;
    }

    // ② Rust 侧算同一时刻的取值。
    const wasm = await import(/* @vite-ignore */ '/engine/pkg/p5b_harness.js');
    await wasm.default();
    window.__P5B_WASM__ = wasm;

    const rect = productionCanvas.getBoundingClientRect();
    const aspect = rect.width / rect.height;
    const rustRaw = wasm.intro_uniforms(
      frozenAt,
      productionCanvas.width,
      productionCanvas.height,
      aspect,
      window.devicePixelRatio || 1,
    );
    const rust = JSON.parse(rustRaw);
    result.rust = rust;

    // ③ **逐值**对照：背景 matte 的 27 项。
    const valueDiff: unknown[] = [];
    let compared = 0;
    for (const [name, value] of Object.entries(captured.background)) {
      const rustValue = (rust.backdrop ?? {})[name];
      const outcome = compareComponents(value, rustValue);
      compared += 1;
      if (!outcome.same) valueDiff.push({ name, production: value, rust: rustValue, reason: outcome.reason });
    }
    result.value_comparison = {
      compared,
      differing: valueDiff.length,
      differences: valueDiff.slice(0, 20),
    };

    // ④ 粒子 pass 的取值对照（生产侧在粒子 pass 存在时才有）。
    const spriteDiff: unknown[] = [];
    let spriteCompared = 0;
    if (captured.sprite && Array.isArray(rust.sprites) && rust.sprites.length > 0) {
      const rustSprite = rust.sprites[0] as Record<string, number[]>;
      for (const [name, value] of Object.entries(captured.sprite)) {
        const rustValue = rustSprite[name];
        const outcome = compareComponents(value, rustValue);
        spriteCompared += 1;
        if (!outcome.same) {
          spriteDiff.push({ name, production: value, rust: rustValue, reason: outcome.reason });
        }
      }
    }
    result.sprite_value_comparison = {
      compared: spriteCompared,
      differing: spriteDiff.length,
      differences: spriteDiff.slice(0, 20),
    };

    // ⑤ 种子对照：生产侧在 `initGL()` 里用 xorshift32 生成 4096×4 个种子
    //    （**确定性** ⇒ 刷新不重排）⇒ ⇒ Rust 侧必须**逐值相同**。
    //    ⚠ 生产种子在 GL 缓冲里 ⇒ JS 读不到 ⇒ ⇒ 这里按**生产同一段循环**复算
    //    （4 行 ⇒ 与生产**逐字**同源）⇒ ⇒ 再与 Rust 的逐值对照。
    const productionSeeds: number[] = [];
    {
      const MAX_PARTICLES = 4096;
      let s = 1234567;
      for (let i = 0; i < MAX_PARTICLES * 4; i += 1) {
        s ^= s << 13;
        s ^= s >>> 17;
        s ^= s << 5;
        productionSeeds.push(((s >>> 0) % 100000) / 100000);
      }
    }
    const rustSeeds = wasm.intro_sprite_seeds();
    let seedDiffering = 0;
    let seedFirstDifference = '';
    for (let i = 0; i < productionSeeds.length; i += 1) {
      if (Math.fround(productionSeeds[i]) !== Math.fround(rustSeeds[i])) {
        seedDiffering += 1;
        if (!seedFirstDifference) {
          seedFirstDifference = `第 ${i} 个：生产 ${productionSeeds[i]} vs Rust ${rustSeeds[i]}`;
        }
      }
    }
    result.seed_comparison = {
      compared: productionSeeds.length,
      differing: seedDiffering,
      first_difference: seedFirstDifference,
    };

    // ⑥ 引擎侧：用**生产侧抓到的**取值画一张（与 P5-b 的生产对照**同一口径**）。
    //    ⇒ 背景 + 粒子**两个 pass**（生产侧同一帧也是两个 pass）。
    const engineUniforms: Record<string, number[]> = { ...captured.background };
    if (captured.sprite && !params.get('nosprite')) {
      (engineUniforms as Record<string, unknown>)['__sprite'] = captured.sprite;
    }
    for (const backend of ['webgl', 'webgpu']) {
      try {
        const raw = await wasm.run_intro(engineCanvases[backend], backend, 3, engineUniforms);
        (result as Record<string, unknown>)[backend] = JSON.parse(raw);
      } catch (error) {
        (result as Record<string, unknown>)[backend] = { error: String(error) };
      }
    }

    // ⑦ ⭐ **持续重绘**（截图前必须）：浏览器的 WebGL drawing buffer 默认不保留
    //    ⇒ 合成后被清空 ⇒ 「画完就停」⇒ 之后截图**全白**（实测 99.94% 差异）。
    const redrawHandles: Record<string, number> = {};
    const startRedraw = (backend: string) => {
      if (redrawHandles[backend] !== undefined) return;
      const tick = () => {
        wasm.redraw_intro(backend, 1).catch(() => {});
        redrawHandles[backend] = requestAnimationFrame(tick);
      };
      redrawHandles[backend] = requestAnimationFrame(tick);
    };
    const stopRedraw = (backend: string) => {
      const handle = redrawHandles[backend];
      if (handle === undefined) return;
      cancelAnimationFrame(handle);
      delete redrawHandles[backend];
    };
    window.__INTRO_REDRAW__ = { start: startRedraw, stop: stopRedraw };
  } catch (error) {
    result.error = String((error as Error)?.message ?? error);
  }
  window.__INTRO__ = result;
  window.__INTRO_DONE__ = true;
}

declare global {
  interface Window {
    __INTRO__: Record<string, unknown>;
    __INTRO_DONE__: boolean;
    __INTRO_REDRAW__: {
      start: (backend: string) => void;
      stop: (backend: string) => void;
    };
    __P5B_WASM__: unknown;
  }
}

main();
