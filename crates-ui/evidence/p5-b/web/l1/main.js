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
  results.app = p5b.l1_app_probe();
} catch (error) {
  results.app = { ok: false, failures: [`宿主探针抛错：${String(error)}`] };
}
// ⭐ 帧驱动验证：改信号（不调 flush）⇒ 等**下一帧** ⇒ DOM 应当自动更新。
results.app.after_frame = { ok: false, failures: ["帧后探针未执行"] };
requestAnimationFrame(() => {
  requestAnimationFrame(() => {
    try {
      results.app.after_frame = p5b.l1_app_probe_after_frame();
    } catch (error) {
      results.app.after_frame = { ok: false, failures: [`帧后探针抛错：${String(error)}`] };
    }
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
  results.binding = p5b.l1_binding_probe();
} catch (error) {
  results.binding = { ok: false, failures: [`绑定探针抛错：${String(error)}`] };
}
try {
  results.integration = p5b.l1_integration_probe();
} catch (error) {
  results.integration = { ok: false, failures: [`集成探针抛错：${String(error)}`] };
}
window.__L1__ = results;
// ⭐ DONE 推迟：必须等帧后探针跑完（否则驱动会提前判定）
