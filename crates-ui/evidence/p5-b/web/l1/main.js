// L1 能力浏览器探针：先初始化 wasm，再跑 DOM 侧探针与「纹理上传」端到端探针。
import init, * as p5b from '/engine/pkg/p5b_harness.js';

const results = {};
try {
  await init();
  results.dom = p5b.l1_probe();
} catch (error) {
  results.dom = { ok: false, failures: [`DOM 探针抛错：${String(error)}`] };
}
try {
  // 传一个真实 canvas（引擎需要它建 surface）；宽高非 2 幂、且后续上传 3×2 图
  const canvas = document.createElement('canvas');
  canvas.width = 64;
  canvas.height = 64;
  canvas.id = 'l1-upload-canvas';
  canvas.style.display = 'none';
  document.body.appendChild(canvas);
  results.upload = await p5b.l1_upload_probe(canvas, 'webgl');
} catch (error) {
  results.upload = { ok: false, failures: [`上传探针抛错：${String(error)}`] };
}
try {
  // ⭐⭐⭐ E-182：异步数据线（真定时器 + 乱序 + 仲裁）⇒ ⇒ 自己 boot / 自己 dispose
  //   ⇒ ⇒ **必须排在 `l1_app_probe` 之前**：它会 boot 一个新宿主并在结束时销毁
  //   ⇒ ⇒⇒ 若放在后面会把 app 探针的宿主顶掉 ⇒ ⇒ 后续 after_frame 探针全红。
  // ⚠️ **两段同步导出**（不是 async 导出）：WebKitGTK 里 async 导出实测会挂住
  //   （2026-11-05）⇒ ⇒ 两段式让「挂在哪一段」可观测。
  p5b.l1_async_line_arm();
  // ⚠️⚠️ **不要赌固定 900ms**（E-192：3 跑 1 败的根因是**单次采样竞态**）
  //
  // `arm` 只是**排了定时器**；定时器触发后，`AsyncLine` 发布的是**快照**
  // ⇒ ⇒ 还要等**宿主 tick（rAF）**把快照写进 DOM
  // ⇒ ⇒ ⇒ 固定等 900ms 只是「大概率够」，一旦定时器在 ~899ms 才回来
  //   ⇒ ⇒⇒ 采集时 DOM 还是占位文本 ⇒ ⇒ 假红「过期结果覆盖了新结果」。
  //
  // ⇒ ⇒ 正确做法：**条件等待** —— 轮询到占位文本消失，再等两帧稳定，才采集。
  // ⚠️⚠️ 这里我**写错过两版**，都如实记（两版都是「等错了东西」）：
  //   ① 固定等 900ms ⇒ ⇒ 慢请求正好卡在边界 ⇒ ⇒ **3 跑 1 败**（E-192 原症状）；
  //   ② 等「占位文本消失」⇒ ⇒ 第一条回来占位就消失 ⇒ ⇒ 采得太早 ⇒ ⇒ **实测 0/10 全败**
  //      （失败项从「DOM 没写上」变成「resolved=1」）；
  //   ③ 等「文本稳定 300ms」⇒ ⇒ 慢请求还没回来文本就已稳定 ⇒ ⇒ 仍然太早（实测 resolved=1）。
  //
  // ⇒ ⇒⇒ 三版的共同错因：**用「表象」推「状态」**。
  // ⇒ ⇒ 正解：**按真实状态等待** —— 轮询引擎的 `resolved` 计数（`l1_async_line_peek`
  //   是**只读**、不销毁宿主），等到 `resolved >= 2` 再等两帧才采集。
  const deadline = Date.now() + 8000;
  let waited = 0;
  let lastStats = null;
  while (Date.now() < deadline) {
    try {
      lastStats = p5b.l1_async_line_peek();
    } catch (error) {
      lastStats = { error: String(error) };
      break;
    }
    if ((lastStats.resolved || 0) >= 2) break;
    await new Promise((resolve) => setTimeout(resolve, 25));
    waited += 25;
  }
  // 再等**两帧**：`resolved=2` 说明回调都到了，但 DOM 是**宿主 tick** 写的 ⇒ 等它落盘
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  results.asyncLine = p5b.l1_async_line_collect();
  results.asyncLine.waitedMillis = waited;
  results.asyncLine.peekedAtCollect = lastStats;
} catch (error) {
  results.asyncLine = {
    ok: false,
    failures: [`异步数据线探针抛错：${String(error)}（阶段=${window.__P5B_ASYNC_STAGE__ || '未知'}）`],
  };
}
try {
  results.app = p5b.l1_app_probe();
} catch (error) {
  results.app = { ok: false, failures: [`宿主探针抛错：${String(error)}`] };
}
// ⭐ 帧驱动验证：改信号（不调 flush）⇒ 等**下一帧** ⇒ DOM 应当自动更新。
results.app.after_frame = { ok: false, failures: ["帧后探针未执行"] };

// ⭐⭐ 「延后一帧再窗口化」实验（E-160）：
//   ① 第一步（**同步**、此刻）：挂载容器但**不**窗口化；
//   ② 第二步（**四帧后**，浏览器已 paint）：才窗口化 ⇒ ⇒ 量「延后后」的耗时
//   ⇒ ⇒ 验证「88 ms 能否并入浏览器首次绘制」。
try {
  results.deferredWindow = { step1: p5b.l1_deferred_window_probe() };
} catch (error) {
  results.deferredWindow = { ok: false, failures: [`延后窗口化(第一步)抛错：${String(error)}`] };
}

requestAnimationFrame(() => {
  requestAnimationFrame(() => {
    try {
      results.app.after_frame = p5b.l1_app_probe_after_frame();
    } catch (error) {
      results.app.after_frame = { ok: false, failures: [`帧后探针抛错：${String(error)}`] };
    }
    // ⭐ 延后窗口化的**第二步**：再多等两帧 ⇒ ⇒ 浏览器已 paint ⇒ ⇒ 再窗口化。
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        try {
          results.deferredWindow.step2 = p5b.l1_deferred_window_probe_after_frame();
        } catch (error) {
          results.deferredWindow.step2 = {
            ok: false,
            failures: [`延后窗口化(第二步)抛错：${String(error)}`],
          };
        }
      });
    });

    let rendersAtDispose = 0;
    try {
      results.app.dispose = p5b.l1_app_dispose_probe();
      rendersAtDispose = (results.app.dispose.renders_at_dispose) || 0;
    } catch (error) {
      results.app.dispose = { ok: false, failures: [`销毁探针抛错：${String(error)}`] };
    }
    // ⭐ 再等两帧 ⇒ 帧循环若没停，渲染次数会增长 ⇒ 「帧停止」探针会红。
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        try {
          results.app.frame_stopped = p5b.l1_app_frame_stopped_probe(rendersAtDispose);
        try {
          results.app.reboot = p5b.l1_app_reboot_probe();
        } catch (error) {
          results.app.reboot = { ok: false, failures: [`反复换屏探针抛错：${String(error)}`] };
        }
        } catch (error) {
          results.app.frame_stopped = { ok: false, failures: [`帧停止探针抛错：${String(error)}`] };
        }
        // ── G8：**真 panic 兜底证据**（⚠️ 必须是**最后一个** wasm 调用）──────
        //
        // 为什么放这里：上面所有探针都是**顶层同步**执行的（rAF 之前就跑完了）
        // ⇒ ⇒ 这个 rAF 回调用到的 `reboot` / `frame_stopped` 是**最后一批** wasm 调用
        // ⇒ ⇒ ⇒ 真 panic 会把 wasm 实例 trap 掉 ⇒ 之后任何 wasm 调用都会抛
        //   `RuntimeError` ⇒ ⇒ 所以必须等它们全跑完再崩。
        //
        // ⭐ 页面 JS **不会**跟着死（死的是 wasm 实例，不是页面上下文）
        //   ⇒ ⇒ 所以「崩之后」仍可用**纯 JS** 读兜底证据。
        results.panicFallbackBefore =
          typeof window.__PANIC_FALLBACK__ !== 'undefined' ? window.__PANIC_FALLBACK__ : null;
        results.panicDomBefore = document.getElementById('panic-fallback')?.textContent ?? null;
        results.panicTriggered = false;
        results.panicTrap = null;
        try {
          p5b.l1_panic_trigger();
          // 返回来了 ⇒ 说明 abort 没发生（panic=unwind 情形）⇒ 照实记
          results.panicTriggered = true;
          results.panicReturned = true;
        } catch (error) {
          results.panicTriggered = window.__PANIC_TRIGGERED__ === true;
          results.panicTrap = String(error);
        }
        // ④ 兜底证据（纯 JS，不受 wasm 死亡影响）
        results.panicFallbackAfter =
          typeof window.__PANIC_FALLBACK__ !== 'undefined' ? window.__PANIC_FALLBACK__ : null;
        results.panicDomAfter = document.getElementById('panic-fallback')?.textContent ?? null;
        window.__L1_DONE__ = true;
      });
    });
  });
});
try {
  results.perf = p5b.l1_perf_probe();
} catch (error) {
  results.perf = { ok: false, failures: [`规模基线探针抛错：${String(error)}`] };
}
try {
  results.realLoad = p5b.l1_real_load_probe();
} catch (error) {
  results.realLoad = { ok: false, failures: [`真实负载基线探针抛错：${String(error)}`] };
}
try {
  results.w0Window = p5b.l1_w0_window_probe();
} catch (error) {
  results.w0Window = { ok: false, failures: [`W0 窗口化保持性探针抛错：${String(error)}`] };
}
try {
  // ⭐ 对照：挂载与窗口化在**同一个 wasm 调用**里 ⇒ ⇒ 窗口可能算错（E-162）。
  results.w0SyncProbe = p5b.w0_first_screen_sync_probe();
} catch (error) {
  results.w0SyncProbe = { error: `同步窗口化对照探针抛错：${String(error)}` };
}
try {
  // ⭐⭐ 最小复现：清空 ⇒ 再填充 ⇒ DOM 会不会翻倍（E-163）。
  results.patchChildren = p5b.l1_patch_children_probe();
} catch (error) {
  results.patchChildren = { error: `patch_children 最小复现抛错：${String(error)}` };
}
try {
  // ⭐⭐⭐ 滚动信号探针：引擎到底有没有收到 `scroll` 事件（E-164）。
  results.scrollSignal = p5b.l1_scroll_signal_probe();
} catch (error) {
  results.scrollSignal = { error: `滚动信号探针抛错：${String(error)}` };
}
try {
  results.binding = p5b.l1_binding_probe();
} catch (error) {
  results.binding = { ok: false, failures: [`绑定探针抛错：${String(error)}`] };
}
try {
  // ⭐⭐⭐⭐ CLS（布局位移）观测：真位移 > 0 / 亚像素 = 0 / 视口外 = 0（L1-G7 最后一段）
  results.layoutShift = p5b.l1_layout_shift_probe('l1-root-host');
} catch (error) {
  results.layoutShift = { ok: false, failures: [`CLS 探针抛错：${String(error)}`] };
}
try {
  // ⭐⭐⭐ 键盘语义探针：Enter 与 Shift+Enter 可区分？IME 组合期可识别？（E-169）
  results.keyboardSemantics = p5b.l1_keyboard_semantics_probe();
} catch (error) {
  results.keyboardSemantics = { ok: false, failures: [`键盘语义探针抛错：${String(error)}`] };
}
try {
  // ⭐⭐⭐ 焦点与模态探针：Tab 真走引擎？焦点陷阱在真实浏览器里成立？（E-174）
  results.focusModal = p5b.l1_focus_modal_probe();
} catch (error) {
  results.focusModal = { ok: false, failures: [`焦点与模态探针抛错：${String(error)}`] };
}
try {
  results.integration = p5b.l1_integration_probe();
} catch (error) {
  results.integration = { ok: false, failures: [`集成探针抛错：${String(error)}`] };
}
window.__L1__ = results;
// ⭐ DONE 推迟：必须等帧后探针跑完（否则驱动会提前判定）
