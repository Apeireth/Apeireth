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
  await new Promise((resolve) => setTimeout(resolve, 900));
  results.asyncLine = p5b.l1_async_line_collect();
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
