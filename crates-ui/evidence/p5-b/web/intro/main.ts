/**
 * P5-b 片头腿 —— 生产 `intro/fire.ts` ↔ Rust 引擎（**逐值** + **逐像素**）。
 *
 * ## 这一页在测什么
 *
 * | 判据 | 含义 |
 * |---|---|
 * | **逐值** | 生产侧**实际用的那一帧**的 uniform 取值 vs Rust 场景层算出来的取值 ⇒ **逐个分量**比 |
 * | **逐像素** | 生产侧画布 vs 引擎侧画布（**同一帧**、同尺寸、同 DPR）⇒ 逐像素差 |
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
 * ⚠ **粒子 pass 可能有 0/1/2 组**（crossfade 期间两拍同时活跃 ⇒ 两组；
 *   `count01 <= 0.001` 或 `alpha <= 0.001` 的拍**不画** ⇒ 生产侧会**跳过**）
 * ⇒ ⇒ 本页按**绘制顺序**把**每一次**粒子绘制**逐组**记下来。
 *
 * ## ⭐ 隔离对照（`?only=`）
 *
 * `?only=backdrop` ⇒ 生产侧**跳过**粒子绘制、引擎侧**只**画背景 ⇒ 两者逐像素对照
 * ⇒ ⇒ 把「背景 pass」与「粒子 pass」的问题**分开**定位。
 * `?only=sprite` ⇒ 反之（只画粒子；背景是透明 ⇒ 画布上只剩粒子的加法）。
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
  backdrop: UniformSnapshot | null;
  sprites: UniformSnapshot[];
  draws: number;
  frame_modes: number[];
} = { backdrop: null, sprites: [], draws: 0, frame_modes: [] };

/** 只画某个 pass（`bg` / `sprite`）—— 生产侧**跳过**另一个 pass 的绘制。 */
const only = new URLSearchParams(location.search).get('only') ?? 'all';

/** 引擎侧用哪一套取值渲染：`rust` = Rust 场景层算的（端到端）；`production` = 生产抓到的。 */
const source = new URLSearchParams(location.search).get('source') ?? 'rust';

/**
 * ⭐ 粒子的**绘制路径**（`quad` = 实例化四边形（后端无关）/ `point` = 点图元）。
 *
 * ## 为什么要有这个开关
 *
 * 两条路径**必须产出同一画面** ⇒ ⇒ 驱动要把两条腿的截图**逐像素对照**
 * （⇒ 「四边形展开忠实复现点精灵」这条不变量**可复核**）。
 *
 * ⚠ `point` 路径在 **WebGPU 后端不可用**（规范没有点尺寸）⇒ 那一腿的
 *   webgpu 段会带 `error` ⇒ ⇒ 驱动按既有规则把该腿标成 **invalid** ✓
 *   （不把「没画」当「一致」）。
 */
const spriteMode = new URLSearchParams(location.search).get('sprite') ?? 'quad';

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

  // ⭐ **探针模式**（`?probe=flat`）：把**粒子**片元着色器换成平色 ⇒ 画布上只剩
  //   粒子 quad 的并集 ⇒ 可直接量出 sprite 的**位置/尺寸**，与引擎侧同款探针逐像素对照。
  //   （只认带 `vStreak` 的那支 = 火焰粒子；背景 matte 的片元不动。）
  const probe = new URLSearchParams(location.search).get('probe');
  // ⭐⭐ **探针自证计数器**（2026-10-27）：模块级、位于所有 hook **之前** ⇒ ⇒ **无 TDZ 风险**
  // ⇒ ⇒ 用途：确认片元**真的**被替换过 ⇒ ⇒ 「差异 0」与「两侧都没画」**可区分**。
  let probeReplacements = 0;
  // ⭐ **诊断日志**（2026-10-27）：探针分支是否进入
  console.log('[probe-diag] search=', location.search, 'probe=', probe);
  // ⭐⭐ **诊断信息写进页面全局**（2026-10-27）：
  // ⇒ ⇒ 驱动的 console 收集是**导航之后**才开始的 ⇒ ⇒ **导航期间的消息全丢**
  // ⇒ ⇒ ⇒ 探针的诊断**必须**放在页面里可读的地方，否则「看不到日志」
  //    **不能**推出「hook 没被调用」（实测踩过这个推理错误）。
  const diag = {
    search: location.search,
    probe,
    shaderSources: [] as string[],
    // ⭐ **上下文判定**：生产到底用哪个 GL 原型？
    contexts: [] as string[],
    getContextCalls: 0,
    hookInstalledOn: [] as string[],
    get replacements() { return probeReplacements; },
  };
  // ⭐ hook `HTMLCanvasElement.prototype.getContext`（**正确**的对象）
  {
    const cp = (HTMLCanvasElement as unknown as { prototype: Record<string, unknown> }).prototype;
    const orig = cp['getContext'] as (...a: unknown[]) => unknown;
    cp['getContext'] = function (this: unknown, ...args: unknown[]) {
      diag.getContextCalls += 1;
      diag.contexts.push(String(args[0]));
      return orig.apply(this, args);
    };
    diag.hookInstalledOn.push('HTMLCanvasElement');
  }
  (window as unknown as Record<string, unknown>).__PROBE_DIAG__ = diag;
  // ⭐ hook **两个**原型（WebGL1 / WebGL2）⇒ ⇒ 排除「装错原型」。
  for (const [name, protoAny] of [
    ['WebGL2', (WebGL2RenderingContext as unknown as { prototype: Record<string, unknown> }).prototype],
    ['WebGL1', (WebGLRenderingContext as unknown as { prototype: Record<string, unknown> }).prototype],
  ]) {
    const p2 = protoAny;
    const origGetContext = p2['getContext'] as (...a: unknown[]) => unknown;
    p2['getContext'] = function (this: unknown, ...args: unknown[]) {
      diag.getContextCalls += 1;
      diag.contexts.push(String(args[0]));
      return origGetContext.apply(this, args);
    };
    diag.hookInstalledOn.push(name);
  }
  if (probe === 'varying') {
    // ⭐⭐ **varying 编码探针**（2026-10-27 诊断用）。
    // ⇒ ⇒ 与引擎侧 `assets/intro_sprite_probe.frag` **逐字同源**。
    // ⇒ ⇒ 把 varying 编码进 RGB ⇒ ⇒ 逐像素差分**直接读出是哪个 varying 不同**。
    //
    // ⚠⚠️ **必须碰一次 `gl_PointCoord`**：引擎的**四边形路径**会把
    // `gl_PointCoord` 换成 varying `_engine_point_coord`，且**顶点阶段总是输出**它
    // ⇒ ⇒ 若片元完全不提及 ⇒ ⇒ 顶点输出与片元输入**不匹配**
    // ⇒ ⇒ wgpu 报错 ⇒ ⇒ wasm panic（实测踩过）。
    // ⇒ ⇒ 生产侧是**原生 WebGL2** ⇒ ⇒ `gl_PointCoord` 是**内建** ⇒ ⇒ 天然合法。
    //
    // ⚠ 注释里**不能出现反引号**（这段在 TS 模板字符串里 ⇒ 反引号会终止它）。
    const varyingFrag = `#version 300 es
precision highp float;
in vec3 vCol;
in float vAlpha;
in float vStreak;
in vec2 vAxis;
in float vSoft;
out vec4 fragColor;
void main(){
  // ⚠⚠️ **必须触碰全部 5 个 varying**（2026-10-27 实测踩到的坑）：
  // 未被使用的 varying 会被**编译器优化剔除** ⇒ ⇒ 顶点输出集合与片元输入集合
  // **不匹配** ⇒ ⇒ wgpu 报 FRAGMENT varying vCol does not match any VERTEX varying
  // ⇒ ⇒ 建管线失败（实测）。
  //
  // 手法：用「**不可能成立的分支**」引用它们
  // ⇒ ⇒ 编译器**无法证明**它恒假 ⇒ ⇒ varying **被保留**
  // ⇒ ⇒ 而输出值**不变**（该分支永不进入）。
  float unusedCoord = gl_PointCoord.x;
  float guard = vSoft * vAxis.x;
  float guard2 = unusedCoord * vSoft;
  if (guard < -1.0 || guard2 < -1.0) {
    fragColor = vec4(0.0, 0.0, 0.0, 1.0);
  }
  fragColor = vec4(vAlpha, vCol.g, vStreak, 1.0);
}`;
    const originalShaderSource = proto['shaderSource'] as (
      shader: WebGLShader,
      source: string,
    ) => void;
    proto['shaderSource'] = function (this: WebGL2RenderingContext, shader: WebGLShader, source: string) {
      // ⭐⭐ **纯源码特征判「片元」**（2026-10-27 修的真缺陷）：
      // 原先用 `getShaderParameter(shader, SHADER_TYPE)` ⇒ ⇒ 在 `shaderSource`
      // **调用期间**查询会**抛异常**（实测：ANGLE 拒绝）⇒ ⇒ hook 中断
      // ⇒ ⇒ ⇒ 探针**从不生效**（而且日志上看不出来）。
      // ⇒ ⇒ 改用源码特征：**含 `vStreak`**（火焰片元有）
      // 且**不含 `gl_Position`**（顶点着色器有）⇒ ⇒ 无需任何 API 查询。
      const isFragment =
        source.includes('vStreak') && !source.includes('gl_Position');
      console.log('[probe] shaderSource', { isFragment, hasVStreak: source.includes('vStreak'), head: source.slice(0, 70).replace(/\s+/g, ' ') });
      if (isFragment) probeReplacements += 1;
      const src = isFragment ? varyingFrag : source;
      return originalShaderSource.call(this, shader, src);
    };
  } else if (probe === 'coord') {
    // ⭐⭐ **坐标编码探针**（2026-10-27 诊断用）：R=x、G=y ⇒ ⇒ 直接读出坐标差。
    // ⚠ 与引擎侧 `assets/intro_sprite_coord.frag` **逐字同源**；注释里**不能有反引号**。
    const coordFrag = `#version 300 es
precision highp float;
in vec3 vCol;
in float vAlpha;
in float vStreak;
in vec2 vAxis;
in float vSoft;
out vec4 fragColor;
void main(){
  fragColor = vec4(gl_PointCoord.x, gl_PointCoord.y, 0.0, 1.0);
}`;
    const originalShaderSource = proto['shaderSource'] as (
      shader: WebGLShader,
      source: string,
    ) => void;
    proto['shaderSource'] = function (this: WebGL2RenderingContext, shader: WebGLShader, source: string) {
      const isFragment =
        source.includes('vStreak') && !source.includes('gl_Position');
      if (isFragment) probeReplacements += 1;
      if (isFragment) probeReplacements += 1;
      const src = isFragment ? coordFrag : source;
      return originalShaderSource.call(this, shader, src);
    };
  } else if (probe === 'flat') {
    const flatFrag = `#version 300 es
precision highp float;
in vec3 vCol;
in float vAlpha;
in float vStreak;
in vec2 vAxis;
in float vSoft;
out vec4 fragColor;
void main(){
  // ⚠⚠️ **必须碰一下 gl_PointCoord**（2026-10-26 实测修的坑）：
  // 引擎的**四边形路径**会把 gl_PointCoord 换成 varying
  // _engine_point_coord，且**顶点阶段总是输出**它
  // ⇒ ⇒ 若片元**完全不提及**它 ⇒ ⇒ 顶点输出与片元输入**不匹配**
  // ⇒ ⇒ wgpu 报错 ⇒ ⇒ wasm panic（实测：页面跑不完 ⇒ 「片头页没有跑完」）。
  // ⇒ ⇒ 这里引用一次 ⇒ ⇒ 引擎的替换器会替换它并**补上** in vec2 声明
  // ⇒ ⇒ 而**生产侧**（WebGL2）里 gl_PointCoord 是**内建** ⇒ ⇒ 天然合法。
  // ⚠️ 注释里**不能出现反引号**（这段在 TS 模板字符串里 ⇒ 反引号会终止它）。
  float unusedCoord = gl_PointCoord.x;
  fragColor = vec4(1.0, 0.0, 0.0, unusedCoord * 0.0 + 1.0);
}`;
    const originalShaderSource = proto['shaderSource'] as (
      shader: WebGLShader,
      source: string,
    ) => void;
    proto['shaderSource'] = function (this: WebGL2RenderingContext, shader: WebGLShader, source: string) {
      // ⭐⭐ **纯源码特征判「片元」**（2026-10-27 修的真缺陷）：
      // ⇒ ⇒ `getShaderParameter(SHADER_TYPE)` 在 `shaderSource` 调用期间**抛异常**
      // ⇒ ⇒ ⇒ hook 中断 ⇒ ⇒ **探针从不生效**（且日志上看不出来）。
      // ⇒ ⇒ 判据：**含 `vStreak`** 且**不含 `gl_Position`**（顶点着色器有 `gl_Position`）。
      // ⚠⚠️ **必须判「片元」**：顶点着色器**也含** `vStreak`（`out float vStreak;`）
      // ⇒ ⇒ 若不判 ⇒ ⇒ 顶点被换成片元 ⇒ ⇒ **编译失败** ⇒ ⇒ 画面空 ⇒ ⇒ **假绿**。
      const isFragment =
        source.includes('vStreak') && !source.includes('gl_Position');
      if (isFragment) probeReplacements += 1;
      const src = isFragment ? flatFrag : source;
      return originalShaderSource.call(this, shader, src);
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
    const isPoints = mode === 0;
    // ⚠ **只留最新一帧**：生产侧每帧都重画 ⇒ 若把**所有帧**的粒子快照都攒着
    //   ⇒ 帧数一多就与 Rust 侧的「每拍一组」**错位** ⇒ ⇒ 背景绘制 = 帧边界
    //   ⇒ 一见背景绘制就把粒子快照**清空**（生产侧 `render()` 里背景**先**画）。
    if (isPoints) {
      captured.sprites.push(snapshot);
      captured.frame_modes.push(mode);
    } else {
      captured.backdrop = snapshot;
      captured.sprites = [];
      captured.frame_modes = [mode];
    }
    // ⚠ **隔离对照**：被跳过的 pass **不画**（但快照照样记 ⇒ 取值仍可对照）。
    if (only === 'backdrop' && isPoints) return;
    if (only === 'sprite' && !isPoints) return;
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

// ⭐ **单帧模式**（`?frames=1`）：生产侧**只画一帧**。
//
// ## 为什么需要这个开关
//
// 浏览器的 drawing buffer 在合成后被清空 ⇒ 但 **rAF 帧率可能远高于合成帧率**
// ⇒ ⇒ 在两次合成之间，生产侧可能已经画了**很多帧** ⇒ ⇒ 画布上是**多帧叠加**
// ⇒ ⇒ 引擎侧（每帧清屏 ⇒ 只画一帧）就与它**对不上**。
//
// ⇒ 本开关把 rAF 掐成「只跑一次」⇒ ⇒ 生产侧画布 = **确定的一帧**。
const singleFrame = params.get('frames') === '1';
if (singleFrame) {
  const realRaf = window.requestAnimationFrame.bind(window);
  let calls = 0;
  window.requestAnimationFrame = ((callback: FrameRequestCallback) => {
    calls += 1;
    if (calls === 1) return realRaf(callback);
    return 0;
  }) as typeof window.requestAnimationFrame;
}

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
  const result: Record<string, unknown> = { frozen_at: frozenAt, only, source, sprite_mode: spriteMode };
  try {
    // ① 等生产侧画出第一帧（录制器会抓到取值）。
    const deadline = Date.now() + 30000;
    while (captured.draws < 2 && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    result.production_draws = captured.draws;
    if (!captured.backdrop) {
      result.error = '生产侧没有抓到背景 uniform';
      window.__INTRO__ = result;
      window.__INTRO_DONE__ = true;
      return;
    }

    // ② Rust 侧算同一时刻的取值。
    const wasm = await import(/* @vite-ignore */ '/engine/pkg/p5b_harness.js');
    await wasm.default();
    window.__P5B_WASM__ = wasm;

    const rect = productionCanvas.getBoundingClientRect();
    const aspect = rect.width / rect.height;
    // ⚠ 生产侧 `layout()`：`dpr = min(window.devicePixelRatio, PERFORMANCE.dprMax=2)`。
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const rustRaw = wasm.intro_uniforms(
      frozenAt,
      productionCanvas.width,
      productionCanvas.height,
      aspect,
      dpr,
    );
    const rust = JSON.parse(rustRaw);
    result.rust = rust;
    result.production = {
      backdrop: captured.backdrop,
      sprites: captured.sprites,
    };
    result.viewport = {
      canvas_width: productionCanvas.width,
      canvas_height: productionCanvas.height,
      css_width: rect.width,
      css_height: rect.height,
      aspect,
      dpr,
    };

    // ③ **逐值**对照：背景 matte 的 27 项。
    const valueDiff: unknown[] = [];
    let compared = 0;
    for (const [name, value] of Object.entries(captured.backdrop)) {
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

    // ④ 粒子 pass 的取值对照（**逐组**：生产侧画了几组就比几组）。
    //    ⚠ 生产侧会**跳过** `count01 <= 0.001` / `alpha <= 0.001` 的拍 ⇒
    //    ⇒ Rust 侧也必须**只报实际会画的那几组**（`rust.sprites`）。
    const spriteDiff: unknown[] = [];
    let spriteCompared = 0;
    const rustSprites = Array.isArray(rust.sprites) ? rust.sprites : [];
    result.sprite_pass_counts = {
      production: captured.sprites.length,
      rust: rustSprites.length,
    };
    for (let pass = 0; pass < captured.sprites.length; pass += 1) {
      const productionPass = captured.sprites[pass];
      const rustPass = rustSprites[pass] as Record<string, number[]> | undefined;
      for (const [name, value] of Object.entries(productionPass)) {
        const rustValue = rustPass ? rustPass[name] : undefined;
        const outcome = compareComponents(value, rustValue);
        spriteCompared += 1;
        if (!outcome.same) {
          spriteDiff.push({ pass, name, production: value, rust: rustValue, reason: outcome.reason });
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

    // ⑥ 引擎侧：用**选定来源**的取值画（与 P5-b 的生产对照**同一口径**）。
    //    - `rust`（默认）：Rust 场景层算出来的取值 ⇒ **端到端**对照
    //      （生产 TS 编舞 ↔ Rust 编舞 ↔ 着色器 ↔ 像素）；
    //    - `production`：生产抓到的取值 ⇒ 隔离出「着色器是否等价」。
    const engineUniforms: Record<string, unknown> = {
      __backdrop: source === 'rust' ? rust.backdrop : captured.backdrop,
      __sprites:
        source === 'rust'
          ? rustSprites
          : captured.sprites.map((pass) => ({ ...pass })),
    };
    const passFilter = only;
    for (const backend of ['webgl', 'webgpu']) {
      try {
        const raw = await wasm.run_intro(
          engineCanvases[backend],
          backend,
          3,
          engineUniforms,
          passFilter,
          spriteMode,
          // ⭐ **诊断探针**：与生产侧 hook 用**同一个**探针
          // ⇒ ⇒ 两侧逐像素差分**直接读出是哪个 varying 不同**。
          // ⚠ 在**此处**现取 URL 参数（外层的 `probe` 常量与本处**不同作用域**，
          //   实测报 `ReferenceError: probe is not defined`）。
          (() => {
            const pr = new URLSearchParams(location.search).get('probe');
            return pr === 'varying' || pr === 'flat' || pr === 'coord' ? pr : '';
          })(),
        );
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
