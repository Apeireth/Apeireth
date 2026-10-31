#!/usr/bin/env python3
"""⭐ **L1 能力浏览器探针驱动**（台账 E-98）。

跑法（在 `crates-ui/evidence/p5-b/` 下）：

```bash
python3 drive_l1.py            # 跑探针，结果写 evidence/l1/report.json
python3 drive_l1.py --out DIR  # 指定输出目录
```

退出码：0 全过；1 有失败项；2 探针未产出结果（装置问题）。

判据（与 `harness/src/lib.rs::l1_probe` 对应）：
- **G1** 引擎 `bounding_box` 与浏览器 `getBoundingClientRect` 逐值一致（≤0.5px）；
- **G2** 真实 `input` 事件载荷可读（`target_value` / `checked` / 坐标字段存在）；
- **G3** `focus_at_path` 真的驱动 `document.activeElement`；`compositionstart/end`
  经 **document 级**监听翻转 `Dom::is_composing()`；
- **G6** `bounding_box_in_device` 按 `dpr × density` 换算（2.0 × 1.25 = 2.5）；
- **G8** 错误边界：强制渲染失败 ⇒ **兜底视图真的进了 DOM** 且 `captured_error()` 是**原始错误**；
- **产品宿主**：`boot` 挂载 ⇒ 批次边界渲染（N 次变更 ⇒ 1 次渲染）⇒ **rAF 帧驱动**
  （不调 flush ⇒ 下一帧自动更新）⇒ `dispose` ⇒ DOM 卸载；
- **双向绑定**：信号 → DOM（下行）与 DOM → 信号（上行，真实 `input` 事件）都成立，
  且**用户自己的处理器不被内部写回覆盖**；
- **组合**：在**同一棵渲染树**上同时跑 受控输入 / IME 静默期 / 焦点陷阱 / 虚拟列表窗口化
  ⇒ ⇒ 证明各能力**组合起来没有缺口**（单点测试全过但组合失效是这类引擎最常见的坑）；
  panic hook 可安装（⚠️ 不在浏览器里触发真 panic：wasm 可能 `panic=abort`，会 trap 掉实例）。

⚠️ 本脚本**只用标准库 + harness 既有 `compare` 工具**（`websocket-client` / `pillow` 之外无新依赖）。
"""
from __future__ import annotations

import argparse
import json
import pathlib
import os
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(
    0,
    str(HERE.parent.parent / "apeireth-ui-demo" / "harness" / "tools"),
)
sys.path.insert(0, str(HERE))

import compare  # noqa: E402

#: 伺服根（默认 vite 构建产物 `web/dist`）。
#:
#: ⚠️ **WebView2 验证走另一条路**：Windows runner 上**不装 node**（CI 里 harness 的
#: `node_modules` 是**符号链接**到生产前端 ⇒ Windows 上不可靠）⇒ ⇒ 改为直接伺服
#: `web/` 根（`l1/index.html` + `main.js` + `engine/pkg/`）⇒ ⇒ **零 node 依赖**。
#: ⇒ 用环境变量 `APEIRETH_L1_SERVE_ROOT` 切换（绝对路径）。
SERVE_ROOT_ENV = "APEIRETH_L1_SERVE_ROOT"
DIST = HERE / "web" / "dist"


def main() -> int:
    parser = argparse.ArgumentParser(description="L1 能力浏览器探针")
    parser.add_argument("--out", default=str(HERE / "evidence" / "l1"))
    parser.add_argument("--timeout", type=float, default=25.0)
    args = parser.parse_args()

    serve_root = Path(os.environ[SERVE_ROOT_ENV]) if os.environ.get(SERVE_ROOT_ENV) else DIST
    if not (serve_root / "l1" / "index.html").exists():
        print(
            f"❌ 缺少构建产物 {serve_root}/l1/index.html（先跑 build.sh 或 vite build）",
            file=sys.stderr,
        )
        return 2

    httpd, port = compare.serve(serve_root)
    chrome, devtools = compare.launch_chrome()
    try:
        cdp = compare.open_page(devtools)
        cdp.call("Page.navigate", {"url": f"http://127.0.0.1:{port}/l1/index.html"})
        deadline = time.time() + args.timeout
        while time.time() < deadline and not cdp.evaluate("!!window.__L1_DONE__"):
            time.sleep(0.2)
        raw = cdp.evaluate(
            "JSON.stringify(window.__L1__ || {ok:false,failures:['探针未产出结果'],probe_missing:true})"
        )
        report = json.loads(raw)
    finally:
        try:
            chrome.kill()
        except Exception:
            pass
        httpd.shutdown()

    out_dir = pathlib.Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(json.dumps(report, ensure_ascii=False, indent=2))
    if report.get("probe_missing"):
        return 2
    # 探针结果分两段：`dom`（G1/G2/G3/G6/G8）与 `upload`（G7 纹理上传端到端）
    dom = report.get("dom") or report
    upload = report.get("upload") or {}
    app = report.get("app") or {}
    binding = report.get("binding") or {}
    integration = report.get("integration") or {}
    dom_ok = bool(dom.get("ok"))
    upload_ok = bool(upload.get("ok"))
    # ⭐ 宿主段有三个子判定：主段 + **帧后**（rAF 帧驱动）+ **销毁**（DOM 卸载）
    #   ⇒ ⇒ 三者都要过 ⇒ ⇒ 任何一个红都会让整段红。
    app_main_ok = bool(app.get("ok"))
    app_frame_ok = bool((app.get("after_frame") or {}).get("ok"))
    app_dispose_ok = bool((app.get("dispose") or {}).get("ok"))
    app_frame_stopped_ok = bool((app.get("frame_stopped") or {}).get("ok"))
    app_reboot_ok = bool((app.get("reboot") or {}).get("ok"))
    app_ok = (
        app_main_ok
        and app_frame_ok
        and app_dispose_ok
        and app_frame_stopped_ok
        and app_reboot_ok
    )
    binding_ok = bool(binding.get("ok"))
    integration_ok = bool(integration.get("ok"))
    if not dom_ok:
        print(f"❌ DOM 探针失败项：{dom.get('failures')}", file=sys.stderr)
    if not upload_ok:
        print(f"❌ 上传探针失败项：{upload.get('failures')}", file=sys.stderr)
    if not app_ok:
        print(f"❌ 宿主探针失败项：{app.get('failures')}", file=sys.stderr)
        print(
            f"❌ 宿主子段：主段={app_main_ok} 帧后={app_frame_ok} 销毁={app_dispose_ok}"
            f" 帧停止={app_frame_stopped_ok} 反复换屏={app_reboot_ok}"
            f"（帧后 failures={(app.get('after_frame') or {}).get('failures')}"
            f" / 销毁 failures={(app.get('dispose') or {}).get('failures')}）",
            file=sys.stderr,
        )
    if not binding_ok:
        print(f"❌ 绑定探针失败项：{binding.get('failures')}", file=sys.stderr)
    if not integration_ok:
        print(f"❌ 集成探针失败项：{integration.get('failures')}", file=sys.stderr)
    if not (dom_ok and upload_ok and app_ok and binding_ok and integration_ok):
        return 1
    print(
        "✅ L1 浏览器探针全过（G1 测量 / G2 载荷 / G3 焦点与 IME / G6 缩放 / "
        "G8 错误边界与 panic hook / G7 纹理上传端到端 / 双向绑定 / 组合集成 / 产品宿主）"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
