#!/usr/bin/env python3
"""⭐⭐⭐⭐⭐ **非 Tauri 的 WebKitGTK 最小壳**（E-183 对照实验）。

## 它要回答的问题

E-183 实测：在 **Tauri 2 壳**里，「JS 回调 → wasm」会**冻结页面**
（定时器与 rAF 两路都中招 ⇒ ⇒ 与宿主 / 快照 / 渲染 / 绑定**全无关**）。

⇒ ⇒ **两个可能**：
① **壳特有**（Tauri 的 IPC / 事件循环集成把回调堵住了）；
② **内核普遍**（WebKitGTK 本身就不允许 wasm 回调重入）。

⇒ ⇒ **这一个数字决定后面所有排期**：
- 若 ① ⇒ ⇒ 「Tauri 壳 + 引擎宿主」这条路**可能要换壳或改集成方式**；
- 若 ② ⇒ ⇒ 必须改**引擎接线**（不用回调重入的模型）。

⇒ ⇒ ⇒ 所以必须**对照** ⇒ ⇒ ⇒ 本脚本用 **PyGObject + WebKit2 4.1** 起一个
**最简窗口**（**不经 Tauri、不经 wry、不经 Tauri 的 IPC**）
⇒ ⇒⇒ 加载**同一份页面、同一份 wasm** ⇒ ⇒⇒ 同样的观测。

## 观测什么

- `window.__L1__.asyncTick`：L1 页面里每 100ms 一次的**心跳**（由 JS 定时器驱动）
  ⇒ ⇒ **它能走到 9** ⇒ ⇒ **定时器回调没冻**；停在 1~2 ⇒ ⇒ 冻了。
- `window.__P5B_ASYNC_STAGE__`：wasm 侧写的阶段标记 ⇒ ⇒ 冻结发生在哪一跳。
- `window.__L1__`：整份 L1 探针结果。

## 用法

```bash
xvfb-run -a --server-args=-screen 0 1280x820x24 python3 drive_webkit_plain.py --out evidence/plain
```
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import subprocess
import sys
import threading
import time

HERE = pathlib.Path(__file__).resolve().parent
REPORT_FILE = HERE / "evidence" / "plain" / "report.json"

#: 页面加载后的最长观察时间（秒）—— 与 Tauri 壳的 900 秒预算同量级但更宽松。
WATCH_SECONDS = 300.0


def _fail(message: str) -> int:
    print(f"❌ {message}", file=sys.stderr)
    return 1


def _ensure_runtime_dir() -> None:
    """GTK 需要 `XDG_RUNTIME_DIR`（否则 `Gtk.init_check` 直接失败 ⇒ ⇒ 报「无法初始化」）。"""
    if not os.environ.get("XDG_RUNTIME_DIR"):
        runtime_dir = pathlib.Path("/tmp/runtime-webkit-plain")
        runtime_dir.mkdir(parents=True, exist_ok=True)
        runtime_dir.chmod(0o700)
        os.environ["XDG_RUNTIME_DIR"] = str(runtime_dir)


def _reexec_under_xvfb(args: list[str]) -> None:
    """没有 DISPLAY 就**用 xvfb-run 重新执行自己**（比自起 Xvfb 更稳：生命周期由它管）。"""
    if os.environ.get("DISPLAY"):
        return
    print("→ 无 DISPLAY ⇒ ⇒ 交给 xvfb-run 重跑本脚本", flush=True)
    os.execvp(
        "xvfb-run",
        [
            "xvfb-run",
            "-a",
            "--server-args=-screen 0 1280x820x24",
            sys.executable,
            str(pathlib.Path(__file__).resolve()),
            *args,
        ],
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=str(REPORT_FILE))
    parser.add_argument("--serve-root", default=str(HERE / "web"))
    args = parser.parse_args()

    out_path = pathlib.Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    # ① 无 DISPLAY ⇒ ⇒ 交给 xvfb-run 重跑（必须在 Gtk 之前）
    _reexec_under_xvfb(sys.argv[1:])

    serve_root = pathlib.Path(args.serve_root)
    if not (serve_root / "l1" / "index.html").exists():
        return _fail(f"缺少页面：{serve_root / 'l1' / 'index.html'}")
    glue = serve_root / "engine" / "pkg" / "p5b_harness.js"
    if not glue.exists():
        return _fail(f"缺少 wasm 胶水：{glue}")

    # ① 本地 HTTP 服务（wasm 模块需要 HTTP，file:// 会被模块 CORS 拦）
    sys.path.insert(0, str(HERE))
    import drive  # noqa: PLC0415  （复用同一份 serve，禁缓存口径一致）

    httpd, port = drive.serve(serve_root)
    url = f"http://127.0.0.1:{port}/l1/index.html"
    print(f"→ 本地服务：{url}", flush=True)

    # ② PyGObject：最简窗口（**不经 Tauri**）
    try:
        import gi  # noqa: PLC0415

        gi.require_version("Gtk", "3.0")
        gi.require_version("WebKit2", "4.1")
        from gi.repository import GLib, Gtk, WebKit2  # noqa: PLC0415
    except Exception as error:  # pragma: no cover - 环境缺依赖
        return _fail(f"缺 PyGObject / WebKit2：{error}")

    _ensure_runtime_dir()
    ok, _argv = Gtk.init_check([])
    if not ok:
        return _fail("Gtk 无法初始化（DISPLAY / XDG_RUNTIME_DIR 不对？）")

    state: dict[str, object] = {"js": None, "stage": None, "done": False, "error": None}
    pending: list[object] = []

    def evaluate(web_view, script: str, key: str) -> None:
        def done(source, task, _data=None):  # noqa: ANN001
            try:
                value = source.evaluate_javascript_finish(task)
                state[key] = value.to_string() if value is not None else None
            except Exception as error:  # pragma: no cover - 页面尚未就绪
                state[key] = None
                state["error"] = f"{key}: {error}"

        try:
            web_view.evaluate_javascript(script, -1, None, None, None, done, None)
        except Exception as error:  # pragma: no cover
            state[key] = None
            state["error"] = f"{key}: {error}"

    def on_load_changed(web_view, event):  # noqa: ANN001
        if event == WebKit2.LoadEvent.FINISHED:
            print("→ 页面加载完成，开始观察", flush=True)

    def on_tick():  # noqa: ANN001
        """每 200ms 读一次页面状态（**读**不是回调 ⇒ ⇒ 不受回调重入影响）。"""
        evaluate(
            web_view,
            "JSON.stringify({tick: (window.__L1__ && window.__L1__.asyncTick) || 0,"
            " stage: window.__P5B_ASYNC_STAGE__ || null,"
            " done: !!window.__L1_DONE__,"
            " hasL1: !!window.__L1__})",
            "js",
        )
        pending.append(GLib.timeout_add(200, on_tick))
        return False

    window = Gtk.Window()
    window.set_default_size(1280, 820)
    window.set_title("webkit-plain-probe")
    web_view = WebKit2.WebView()
    window.add(web_view)
    web_view.connect("load-changed", on_load_changed)
    window.connect("destroy", lambda *_a: Gtk.main_quit())
    window.show_all()
    web_view.load_uri(url)
    pending.append(GLib.timeout_add(200, on_tick))

    deadline = time.time() + WATCH_SECONDS
    result: dict[str, object] = {}

    def finish() -> None:
        for handle in pending:
            GLib.source_remove(handle)
        pending.clear()
        evaluate(
            web_view,
            "JSON.stringify(window.__L1__ || {ok:false,failures:['探针未产出结果']})",
            "final",
        )
        # evaluate 是异步的 ⇒ ⇒ 等它回来再收工
        def stop_loop() -> bool:
            Gtk.main_quit()
            return False

        GLib.timeout_add(500, stop_loop)

    def watch() -> bool:
        if time.time() > deadline:
            print("→ 观察超时", flush=True)
            finish()
            return False
        raw = state.get("js")
        if isinstance(raw, str):
            try:
                snapshot = json.loads(raw)
            except json.JSONDecodeError:
                snapshot = {}
            if snapshot.get("done"):
                print("→ 页面自报完成", flush=True)
                finish()
                return False
        return True

    pending.append(GLib.timeout_add(500, watch))
    Gtk.main()

    # ③ 收产物
    raw_final = state.get("final")
    try:
        result = json.loads(raw_final) if isinstance(raw_final, str) else {}
    except json.JSONDecodeError:
        result = {"parseError": True, "raw": raw_final}
    # ⭐⭐ **判据**（不是看心跳字段——Chromium 页面没有心跳，那是 WebKit 页面专用）：
    #   ① `asyncLine.ok` ⇒ ⇒ **定时器回调 → wasm** 走通了（乱序 + 仲裁全对）
    #   ② `app.after_frame.ok` ⇒ ⇒ **rAF 回调 → wasm** 走通了（帧循环真的在跑）
    async_line = result.get("asyncLine") or {}
    app = result.get("app") or {}
    after_frame = app.get("after_frame") or {}
    timer_ok = bool(async_line.get("ok"))
    raf_ok = bool(after_frame.get("ok"))
    if timer_ok and raf_ok:
        verdict = "callbacks-ok-in-plain-webkit"
    elif timer_ok:
        verdict = "timers-ok-but-raf-frozen"
    else:
        verdict = "frozen-in-plain-webkit"
    result["plainShell"] = {
        "tool": "PyGObject + WebKit2 4.1（**不经 Tauri**）",
        "display": os.environ.get("DISPLAY"),
        "url": url,
        "timerCallbackOk": timer_ok,
        "rafCallbackOk": raf_ok,
        "verdict": verdict,
        "asyncLine": async_line,
        "afterFrame": after_frame,
        "watchSeconds": WATCH_SECONDS,
    }
    out_path.write_text(
        json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(json.dumps(result.get("plainShell", {}), ensure_ascii=False, indent=2))

    shell = result.get("plainShell", {})
    print(f"→ 结果文件：{out_path}", flush=True)
    print(
        f"→ 判据：定时器回调={shell.get('timerCallbackOk')} "
        f"rAF 回调={shell.get('rafCallbackOk')} ⇒ ⇒ verdict={shell.get('verdict')}",
        flush=True,
    )
    if shell.get("verdict") == "callbacks-ok-in-plain-webkit":
        print(
            "✅ 非 Tauri 的 WebKitGTK 里**回调重入可用** ⇒ ⇒ ⇒ "
            "冻结是 **Tauri 壳特有**，**不用改引擎接线**",
            flush=True,
        )
    else:
        print(
            f"⚠️ 非 Tauri 壳里也异常（{shell.get('verdict')}）⇒ ⇒ "
            "⇒ ⇒ **WebKitGTK 内核普遍** ⇒ ⇒ 必须改引擎接线",
            file=sys.stderr,
        )
    httpd.shutdown()
    return 0


if __name__ == "__main__":
    sys.exit(main())
