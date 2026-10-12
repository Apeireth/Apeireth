# avatar-perf-probe —— 桌面常驻形象渲染性能探针

针对桌面常驻小窗（约 220×260 透明置顶窗，WebView2/Chromium 语义）里的角色形象动画，
同页顺序实测三条形象负载路线，并验证两条渲染纪律。预算口径来自
`docs/design/00-PHILOSOPHY.md` §8（T0 预算目标 <150MB RAM、滚动 60fps，数值待实测校准）。

## 三条负载线（全部代码程序化生成，零美术资产）

| 线 | 负载内容 | 技术要点 |
|---|---|---|
| 3D 卡通渲染路线 | 程序化人形（约 20 节关节、33 draw call、约 6.7k 三角形）+ 卡通渐变着色 + 待机呼吸/口型开合/视线摆动/发丝摆动 | WebGL 渲染器，透明底，三灯照明，脚下伪影代替 shadow map |
| 2D 形象路线 | 程序化骨骼小人：35 节关节、34 条骨骼 + 发丝/面部描画段（共 40+ 段），待机呼吸/口型开合/视线摆动/眨眼 | 纯 Canvas2D 正向运动学解算 + 两级卡通勾线 |
| 光态基线 | 呼吸光点（光晕 + 亮核，4s 呼吸周期） | CSS keyframes + SVG 径向渐变/高斯模糊，合成器驱动，JS 每帧近零 |

## 测量口径

- **稳态 FPS**：rAF 采样。均值 = 窗口内实际帧数 / 时长；**1% low** = 瞬时帧率最低 1% 帧的均值。
  注意 rAF 受显示器刷新率封顶（本机 165Hz），因此 FPS 读数是"该刷新率下的稳态帧率"，
  换算 60Hz 显示器要看每帧 JS 预算（`frameJsMs`）。
- **JS 堆**：`performance.memory.usedJSHeapSize`，采样期每 1s 一记，报稳态中位数/峰值；
  另附 `performance.measureUserAgentSpecificMemory()` 的上下文内存量（需 COOP/COEP，静态服务已带）。
- **hidden 纪律**：强制注入 `document.hidden = true` 并派发 `visibilitychange`，
  同时用一条**不看 hidden 的全局 rAF 心跳**做对照——隐藏期渲染帧必须为 0 且全局心跳必须 >0，
  恢复可见后必须继续出帧。（真实窗口失焦/最小化也会走同一处理路径，可在手动测试时最小化窗口复验。）
- **WebGL 降级**：两路验证——① 页内把 `getContext("webgl*")` stub 成返回 null 后重建 3D 负载；
  ② 浏览器以 `--disable-3d-apis` 真实禁用 WebGL 启动。判据：3D 线报不可用（不抛穿），
  宿主回落 2D 形象路线，2D/光态线照常出帧。
- **浸泡**：三线并发跑 5 分钟（最坏情况口径），报前后"同条件 GC 后"JS 堆差与线性斜率。

## 用法

一键采集（起静态服务 + 无头 Edge/Chromium CDP 采集 + 存结果 + 打印摘要，约 10 分钟）：

```powershell
node scripts/avatar-perf-probe.mjs
# 或从本目录：
npm run probe
```

结果落在本目录 `results/`：

- `results/result.json` —— 全量结果（`window.__AVATAR_PERF_PROBE__` 原样 + 浏览器进程树工作集采样）
- `results/result.webgl-off.json` —— `--disable-3d-apis` 真实禁用 WebGL 的降级验证
- `results/result.detail.json` —— 进程级内存拆分补测（`--tag detail`，短时长）
- `results/shot-*.png` —— 运行留证截图（验证形象确实画出来了）

常用参数：

```powershell
node scripts/avatar-perf-probe.mjs --serve          # 只起服务，浏览器手动开页自测
node scripts/avatar-perf-probe.mjs --skip-soak --fps-sec 10 --skip-degrade   # 快速冒烟
node scripts/avatar-perf-probe.mjs --headed         # 有头调试
node scripts/avatar-perf-probe.mjs --soak-sec 600   # 更长浸泡
```

手动自测：`--serve` 后打开 `http://127.0.0.1:8765/index.html`，跑完看页面结果面板，
或在控制台读 `window.__AVATAR_PERF_PROBE__`。URL 参数：`?mode=full|degrade&fpsSec=&soakSec=&hiddenSec=`。

## 文件

| 文件 | 说明 |
|---|---|
| `package.json` | 依赖清单（唯一依赖为 WebGL 渲染库，见下表） |
| `index.html` | 探针页骨架 + 光态基线 SVG/CSS + 结果面板 |
| `main.js` | 三线负载 + 全部测量逻辑 |
| `README.md` | 本文件 |
| `results/` | 采集产物 |

## 依赖表

| 依赖 | 版本 | 用途 | 许可 |
|---|---|---|---|
| `three` | 0.180.0 | 3D 卡通渲染路线的 WebGL 渲染与卡通材质 | MIT |

安装命令（仅此一个依赖）：

```powershell
cd frontend/perf-probe
npm install three
```

运行库经 importmap 解析（`./node_modules/three/build/three.module.js`）；
**未安装时探针如实报"3D 线不可用（three-not-installed）"**，不会谎报 3D 数据。
WebGL 不可用时 3D 线报不可用由宿主回落，2D/光态线照常。

## 诚实边界

- 无头浏览器是 WebView2 同内核（Chromium/Edge）近似，**不是 WebView2 实体**；
  WebView2 的宿主进程共享、合成器与真实透明置顶窗行为未实测。
- rAF FPS 受显示器刷新率封顶（本机 165Hz），跨机对比请用每帧 JS 预算折算。
- `document.hidden` 验证用强制注入（rAF 心跳对照证明"停帧是循环自律"）；
  真实失焦暂停的浏览器兜底行为以手动最小化窗口复验为准。
- 浏览器进程树工作集（Windows WorkingSetSize 合计）是宿主壳 + 内核的近似量级参考，
  与 JS 堆是两种口径，报告里分开陈述。
